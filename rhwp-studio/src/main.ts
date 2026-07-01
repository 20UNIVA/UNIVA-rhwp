import { t, onLangChange, type MessageKey } from '@/i18n/t';
import { applyInitialLangFromUrl, attachLangPostMessageListener } from '@/i18n/lang-boundary';
import { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentInfo } from '@/core/types';
import { EventBus } from '@/core/event-bus';
import { CanvasView } from '@/view/canvas-view';
import { InputHandler } from '@/engine/input-handler';
import { SessionClient } from '@/core/session-client';
import { Toolbar } from '@/ui/toolbar';
import { MenuBar } from '@/ui/menu-bar';
import { loadWebFonts } from '@/core/font-loader';
import { CommandRegistry } from '@/command/registry';
import { CommandDispatcher } from '@/command/dispatcher';
import type { EditorContext, CommandServices } from '@/command/types';
import { confirmSaveBeforeReplacingDocument, fileCommands } from '@/command/commands/file';
import { editCommands } from '@/command/commands/edit';
import { viewCommands } from '@/command/commands/view';
import { formatCommands } from '@/command/commands/format';
import { insertCommands } from '@/command/commands/insert';
import { tableCommands } from '@/command/commands/table';
import { pageCommands } from '@/command/commands/page';
import { toolCommands } from '@/command/commands/tool';
import { ContextMenu } from '@/ui/context-menu';
import { CommandPalette } from '@/ui/command-palette';
import { showValidationModalIfNeeded } from '@/ui/validation-modal';
import { showToast } from '@/ui/toast';
import { mountVfinderModal } from '@/view/vfinder-modal';
import { initRhwpDev } from '@/core/rhwp-dev';
import { getActionDef } from '@/hwpctl/action-registry';
import { HwpCtrl, ParameterSet } from '@/hwpctl/index';
import { DocumentDirtyState } from '@/core/document-dirty-state';
import { CellSelectionRenderer } from '@/engine/cell-selection-renderer';
import { TableObjectRenderer } from '@/engine/table-object-renderer';
import { TableResizeRenderer } from '@/engine/table-resize-renderer';
import { Ruler } from '@/view/ruler';
import type { CanvasKitLayerRenderer } from '@/view/canvaskit-renderer';
import {
  resolveCanvasKitRenderMode,
  resolveCanvasKitSurfaceRequest,
  resolveRenderBackendRequest,
  resolveRenderProfile,
} from '@/view/render-backend';

// vm:ready 신호 — VM iframe 마운트 완료를 부모창에 1회 알림.
// 부모창은 이 신호 도착 전까지 로딩 스피너, 미도착 시 주기적 reload.
// 데이터 로드는 기다리지 않음 — 정적 셸 표시 시점이면 충분.
(() => {
  if (window.parent === window) return;
  const parentOrigin = new URLSearchParams(location.search).get('parentOrigin') || '*';
  try {
    window.parent.postMessage({ type: 'vm:ready' }, parentOrigin);
  } catch {
    /* postMessage 실패해도 앱 진행에 영향 없음 — silent */
  }
})();

/**
 * index.html 박힌 *정적 한국어*를 현재 lang 자료로 교체한다.
 * data-i18n="키" → textContent, data-i18n-aria="키" → aria-label,
 * data-i18n-title="키" → title 속성.
 * 진입 시점과 onLangChange 시점에 호출.
 */
function applyStaticTexts(): void {
  document.querySelectorAll<HTMLElement>('[data-i18n]').forEach((el) => {
    const key = el.dataset.i18n as MessageKey | undefined;
    if (key) el.textContent = t(key);
  });
  document.querySelectorAll<HTMLElement>('[data-i18n-aria]').forEach((el) => {
    const key = el.dataset.i18nAria as MessageKey | undefined;
    if (key) el.setAttribute('aria-label', t(key));
  });
  document.querySelectorAll<HTMLElement>('[data-i18n-title]').forEach((el) => {
    const key = el.dataset.i18nTitle as MessageKey | undefined;
    if (key) el.setAttribute('title', t(key));
  });
}

// 경계 결선: URL ?sysLang= 자료 + 부모 postMessage 자료 두 자리 모두 수신.
{
  const parentOrigin = new URLSearchParams(location.search).get('parentOrigin') || '*';
  applyInitialLangFromUrl();
  attachLangPostMessageListener(parentOrigin);
  // DOM 준비된 시점에 한 번 + lang 바뀌면 또 한 번.
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', applyStaticTexts);
  } else {
    applyStaticTexts();
  }
  onLangChange(applyStaticTexts);
}

const wasm = new WasmBridge();
const eventBus = new EventBus();
const documentState = new DocumentDirtyState(eventBus);
documentState.installBeforeUnload(window);

// E2E 테스트용 전역 노출 (개발 모드 전용)
if (import.meta.env.DEV) {
  (window as any).__wasm = wasm;
  (window as any).__eventBus = eventBus;
  (window as any).__documentState = documentState;
  initRhwpDev(wasm);
}
let canvasView: CanvasView | null = null;
let inputHandler: InputHandler | null = null;
let toolbar: Toolbar | null = null;
let ruler: Ruler | null = null;

// ─── SSR 세션 (fileId 단위 서버 미러링) ──────────────────
let sessionClient: SessionClient | null = null;
/** 현재 연결된 SSR 세션 fileId. */
let currentSsrFileId: string | null = null;
/** 현재 세션이 "빈 문서로 시작"되었는지(열기 시 미편집이면 닫기 위함). */
let currentIsBlank = false;
/**
 * 브라우저가 마지막으로 적용한 op seq. WS `ServerEvent::Ops` 도착마다 갱신.
 * 서버로 page map 을 POST 할 때 staleness 판정용으로 함께 송신.
 */
let lastAppliedSeq = 0;
/** page map POST debounce 타이머 핸들. */
let pageMapPostTimer: ReturnType<typeof setTimeout> | null = null;

const SSR_PARAMS = new URLSearchParams(location.search);
/**
 * iframe 부모가 `?fileId=&ssrBase=` 로 전달. 미지정 시 same-origin 으로 보고
 * `import.meta.env.BASE_URL` ('/hwp/') 의 trailing slash 만 제거해 prefix 로 사용 —
 * fetch 와 WebSocket 모두 `/hwp/sessions/...` 로 자동 전송된다.
 *
 * ssrBase 가 cross-origin 으로 명시되면 그 값을 그대로 사용 (deploy 가이드의 책임).
 */
const SSR_BASE_URL =
  SSR_PARAMS.get('ssrBase') ?? import.meta.env.BASE_URL.replace(/\/$/, '');
const SSR_URL_FILE_ID = SSR_PARAMS.get('fileId');
/** SSR 모드 활성 조건 — fileId/ssrBase/ssr 중 하나라도 있으면. */
const SSR_MODE = SSR_PARAMS.has('fileId') || SSR_PARAMS.has('ssrBase') || SSR_PARAMS.has('ssr');
/** URL `?user=` 자리에서 추출. vfinder modal 의 X-Vfinder-User 헤더 자리에 박힘.
 *  미설정이면 vfinder 서버의 VFINDER_DEFAULT_USER_ID 환경변수 폴백. */
const SSR_USER_ID = SSR_PARAMS.get('user') ?? undefined;

/**
 * 현재 세션 fileId를 주소창 URL(`?fileId=`)에 반영한다(history.replaceState).
 * 새로고침/공유 시 그 문서로 복원되도록 한다. ssrBase 등 다른 파라미터는 보존.
 */
function syncUrlFileId(fileId: string): void {
  try {
    const u = new URL(location.href);
    if (u.searchParams.get('fileId') === fileId) return;
    u.searchParams.set('fileId', fileId);
    history.replaceState(history.state, '', u.toString());
  } catch {
    /* URL 갱신 실패는 치명적이지 않음 */
  }
}

/**
 * 현재 wasm paginate 결과를 page → (sec, para_start, para_end) 묶음으로 서버에 POST.
 *
 * 측정기 격차 우회 — 서버 native paginator (EmbeddedTextMeasurer) 와 WASM Canvas
 * 측정기가 페이지 경계를 다르게 그릴 때, 브라우저 결과를 *진실* 로 삼게 한다.
 *
 * 디바운스 600ms — 연속 편집 중에 매번 POST 하지 않는다. `document-changed` 마다 호출되며
 * 마지막 호출 후 600ms 무이벤트 자리에서 한 번 발사.
 */
function schedulePageMapPost(): void {
  if (!currentSsrFileId) return;
  if (pageMapPostTimer) clearTimeout(pageMapPostTimer);
  pageMapPostTimer = setTimeout(() => {
    pageMapPostTimer = null;
    if (!currentSsrFileId) return;
    let map;
    try {
      map = wasm.getPageMap();
    } catch (e) {
      console.warn('[main] getPageMap 실패:', e);
      return;
    }
    if (!map.pages || map.pages.length === 0) return;
    const body = JSON.stringify({
      seq: lastAppliedSeq,
      total_pages: map.total_pages,
      pages: map.pages,
    });
    const pmHeaders: Record<string, string> = { 'Content-Type': 'application/json' };
    if (SSR_USER_ID) pmHeaders['X-Rhwp-User'] = SSR_USER_ID;
    fetch(`${SSR_BASE_URL}/sessions/${encodeURIComponent(currentSsrFileId)}/page-map`, {
      method: 'POST',
      headers: pmHeaders,
      body,
    }).catch((e) => {
      console.warn('[main] page-map POST 실패:', e);
    });
  }, 600);
}

/** Uint8Array → base64 (청크 처리). */
function ssrBytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

/** fileId용 SessionClient를 만든다(세션 생성/연결은 호출부에서). */
function buildSessionClient(fileId: string): SessionClient {
  const format = wasm.getSourceFormat() === 'hwpx' ? 'hwpx' : 'hwp';
  return new SessionClient({
    baseUrl: SSR_BASE_URL,
    fileId,
    format,
    userId: SSR_USER_ID,
    getSnapshotBytes: () => {
      try {
        return wasm.exportHwpx();
      } catch {
        return null;
      }
    },
    onServerEvent: (ev) => {
      // shape 가드 (Task 4 review Important #2)
      if (!ev || typeof ev !== 'object' || !('kind' in ev)) {
        console.warn('[main] ServerEvent shape 미일치 — 무시:', ev);
        return;
      }
      // [Sub-6] WS broadcast self-echo skip — 자기 clientId 가 발행한 ops 는
      // 이미 로컬 wasm 에 적용됨. 다시 적용하면 600ms 디바운스 단위마다 *복제* 발생.
      // origin_client_id 가 *다른 값* 이거나 *누락* (HTTP /workbench 등) 이면 그대로 적용.
      if (ev.kind === 'ops'
          && typeof (ev as { origin_client_id?: string }).origin_client_id === 'string'
          && (ev as { origin_client_id?: string }).origin_client_id === sessionClient?.getClientId()) {
        return;
      }
      if (ev.kind === 'ops') {
        if (!Array.isArray(ev.ops)) {
          console.warn('[main] ops 필드 누락 — 무시');
          return;
        }
        let appliedCount = 0;
        for (const op of ev.ops) {
          try {
            switch (op.op) {
              case 'insert_text':
                if (typeof op.section !== 'number' ||
                    typeof op.para !== 'number' ||
                    typeof op.offset !== 'number' ||
                    typeof op.text !== 'string') {
                  console.warn('[main] insert_text 필드 누락:', op);
                  break;
                }
                wasm.insertText(op.section, op.para, op.offset, op.text);
                appliedCount += 1;
                break;
              case 'split_paragraph':
                if (typeof op.section !== 'number' ||
                    typeof op.para !== 'number' ||
                    typeof op.offset !== 'number') {
                  console.warn('[main] split_paragraph 필드 누락:', op);
                  break;
                }
                wasm.splitParagraph(op.section, op.para, op.offset);
                appliedCount += 1;
                break;
              case 'replace_runs':
                if (typeof op.section !== 'number' ||
                    typeof op.para !== 'number' ||
                    !Array.isArray(op.runs)) {
                  console.warn('[main] replace_runs payload 미일치 — 무시', op);
                  break;
                }
                wasm.replaceRuns(op.section, op.para, JSON.stringify(op.runs));
                appliedCount += 1;
                break;
              case 'set_paragraph_style':
                if (typeof op.section !== 'number' ||
                    typeof op.para !== 'number' ||
                    typeof op.style !== 'object' || op.style === null) {
                  console.warn('[main] set_paragraph_style payload 미일치 — 무시', op);
                  break;
                }
                wasm.applyParaFormat(op.section, op.para, JSON.stringify(op.style));
                appliedCount += 1;
                break;
              case 'delete_range':
                if (typeof op.section !== 'number' ||
                    typeof op.para_start !== 'number' ||
                    typeof op.char_start !== 'number' ||
                    typeof op.para_end !== 'number' ||
                    typeof op.char_end !== 'number') {
                  console.warn('[main] delete_range payload 미일치 — 무시', op);
                  break;
                }
                wasm.deleteRange(op.section, op.para_start, op.char_start, op.para_end, op.char_end);
                appliedCount += 1;
                break;
              case 'insert_paragraph':
                if (typeof op.section !== 'number' || typeof op.after_para !== 'number') {
                  console.warn('[main] insert_paragraph payload 미일치 — 무시', op);
                  break;
                }
                {
                  const cnt = typeof op.count === 'number' ? op.count : 1;
                  for (let i = 0; i < cnt; i++) {
                    wasm.insertParagraph(op.section, op.after_para + i);
                    if (op.style && typeof op.style === 'object') {
                      wasm.applyParaFormat(op.section, op.after_para + i + 1, JSON.stringify(op.style));
                    }
                  }
                  appliedCount += 1;
                }
                break;
              case 'press_enter': {
                // 한컴 Enter / Ctrl+Enter — 본문 / 셀 모드 분기 (table_para 키 박힘 = 셀 모드).
                // 내부 자체 split_paragraph / split_paragraph_in_cell 헬퍼 호출 — wasm-bridge 의
                // 기존 함수 자체 자체 활용. count 회 반복, page_break 자세 자체 첫 회만 적용.
                const opPe = op as unknown as {
                  section?: number;
                  para?: number;
                  table_para?: number;
                  row?: number;
                  col?: number;
                  cell_idx?: number;
                  cell_para?: number;
                  char_offset?: number;
                  count?: number;
                  style?: unknown;
                  page_break?: boolean;
                };
                if (typeof opPe.section !== 'number') {
                  console.warn('[main] press_enter section 누락 — 무시', op);
                  break;
                }
                const charOffsetRaw = typeof opPe.char_offset === 'number' ? opPe.char_offset : -1;
                const count = typeof opPe.count === 'number' ? opPe.count : 1;
                const pageBreak = opPe.page_break === true;
                const cellMode = typeof opPe.table_para === 'number';

                if (cellMode) {
                  if (pageBreak) {
                    console.warn('[main] press_enter 셀 모드 + page_break:true — 무시 (셀 안 page_break 미지원)', op);
                    break;
                  }
                  if (typeof opPe.row !== 'number' || typeof opPe.col !== 'number' ||
                      typeof opPe.cell_para !== 'number' || typeof opPe.cell_idx !== 'number') {
                    console.warn('[main] press_enter 셀 모드 payload 미일치 (cell_idx fill 부재) — 무시', op);
                    break;
                  }
                  // 셀 char_offset = -1 자세 자체 자체 자체 자체 자체 splitParagraphInCell 자세 자체 자체
                  // 자체 자체 *u32 받음* — -1 자체 자체 자체 자체 *큰 양수* 자체 자체 자체 자체 split
                  // helper 의 `.min(total_chars)` clamp 자세 자체 자체.
                  const cellCharOffset = charOffsetRaw < 0 ? 0x7FFFFFFF : charOffsetRaw;
                  for (let i = 0; i < count; i++) {
                    const targetCellPara = opPe.cell_para + i;
                    const targetOffset = i === 0 ? cellCharOffset : 0;
                    wasm.splitParagraphInCell(opPe.section, opPe.table_para!, 0, opPe.cell_idx, targetCellPara, targetOffset);
                  }
                } else {
                  if (typeof opPe.para !== 'number') {
                    console.warn('[main] press_enter 본문 모드 para 누락 — 무시', op);
                    break;
                  }
                  // 본문 char_offset = -1 자세 자체 자체 자체 자체 자체 큰 양수 자체 자세 split clamp.
                  const bodyCharOffset = charOffsetRaw < 0 ? 0x7FFFFFFF : charOffsetRaw;
                  for (let i = 0; i < count; i++) {
                    const targetPara = opPe.para + i;
                    const targetOffset = i === 0 ? bodyCharOffset : 0;
                    // page_break(i==0): insertPageBreak *단독* = split + 쪽나누기 한 파이프라인
                    // (native insert_page_break_native 와 동일) → 새 문단 1개에 break.
                    // 그 외: 일반 split. (구: split + insertPageBreak = 이중 split, 또는
                    // split + setPageBreak = 이중 파이프라인 → 다중 페이지 재페이지네이션 콜랩스.)
                    if (i === 0 && pageBreak) {
                      wasm.insertPageBreak(opPe.section, targetPara, targetOffset);
                    } else {
                      wasm.splitParagraph(opPe.section, targetPara, targetOffset);
                    }
                    if (opPe.style && typeof opPe.style === 'object') {
                      wasm.applyParaFormat(opPe.section, targetPara + 1, JSON.stringify(opPe.style));
                    }
                  }
                }
                appliedCount += 1;
                break;
              }
              case 'delete_element':
                if (typeof op.section !== 'number' || typeof op.para !== 'number' ||
                    typeof op.element_type !== 'string') {
                  console.warn('[main] delete_element payload 미일치 — 무시', op);
                  break;
                }
                if (op.element_type === 'paragraph') {
                  wasm.deleteParagraph(op.section, op.para);
                } else if (op.element_type === 'table') {
                  // controlIdx 는 broadcast 페이로드에 없음 — 0 가정 (한 문단에 한 표).
                  wasm.deleteTableControl(op.section, op.para, 0);
                } else {
                  console.warn(`[main] 알 수 없는 element_type: ${op.element_type}`);
                  break;
                }
                appliedCount += 1;
                break;
              case 'insert_table':
                if (typeof op.section !== 'number' ||
                    typeof op.insert_after_para !== 'number' ||
                    typeof op.rows !== 'number' ||
                    typeof op.cols !== 'number') {
                  console.warn('[main] insert_table payload 미일치 — 무시', op);
                  break;
                }
                {
                  // 서버는 char_offset = 문단 길이로 호출. 클라도 동일하게 — 문단 끝에 삽입.
                  const paraLen = wasm.getParagraphLength(op.section, op.insert_after_para);
                  wasm.createTable(op.section, op.insert_after_para, paraLen, op.rows, op.cols);
                  appliedCount += 1;
                }
                break;
              case 'set_cell_style':
                if (typeof op.section !== 'number' ||
                    typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' ||
                    typeof op.col !== 'number' ||
                    typeof op.style !== 'object' || op.style === null) {
                  console.warn('[main] set_cell_style payload 미일치 — 무시', op);
                  break;
                }
                {
                  // [4-4 fix] 서버가 변환해 보낸 cell_idx + ctrl_idx 우선 사용 — 다중 사용자 race 회피.
                  // 없으면 wasm.findCellIdx fallback (구 서버 호환). ctrl_idx 는 paragraph 안
                  // Table 위치 (section_def + column_def 가 앞에 박힐 때 0 이 아님).
                  const ctrlIdx = typeof op.ctrl_idx === 'number' ? op.ctrl_idx : 0;
                  const cellIdx = typeof op.cell_idx === 'number'
                    ? op.cell_idx
                    : wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col);
                  // setCellProperties wrapper 가 자체적으로 JSON.stringify 함 — object 그대로 전달.
                  wasm.setCellProperties(op.section, op.table_para, ctrlIdx, cellIdx, op.style as any);
                  appliedCount += 1;
                }
                break;
              case 'merge_cells':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row_start !== 'number' || typeof op.col_start !== 'number' ||
                    typeof op.row_end !== 'number' || typeof op.col_end !== 'number') {
                  console.warn('[main] merge_cells payload 미일치 — 무시', op);
                  break;
                }
                {
                  const ctrlIdx = typeof op.ctrl_idx === 'number' ? op.ctrl_idx : 0;
                  wasm.mergeTableCells(op.section, op.table_para, ctrlIdx, op.row_start, op.col_start, op.row_end, op.col_end);
                  appliedCount += 1;
                }
                break;
              case 'replace_cell_runs':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para !== 'number' || !Array.isArray(op.runs)) {
                  console.warn('[main] replace_cell_runs payload 미일치 — 무시', op);
                  break;
                }
                {
                  // [4-4 fix] cell_idx + ctrl_idx 우선 + fallback.
                  const ctrlIdx = typeof op.ctrl_idx === 'number' ? op.ctrl_idx : 0;
                  const cellIdx = typeof op.cell_idx === 'number'
                    ? op.cell_idx
                    : wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col);
                  wasm.replaceCellRuns(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, JSON.stringify(op.runs));
                  appliedCount += 1;
                }
                break;
              case 'insert_text_in_cell':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para !== 'number' || typeof op.offset !== 'number' ||
                    typeof op.text !== 'string') {
                  console.warn('[main] insert_text_in_cell payload 미일치 — 무시', op);
                  break;
                }
                {
                  // [4-4 fix] cell_idx + ctrl_idx 우선 + fallback.
                  const ctrlIdx = typeof op.ctrl_idx === 'number' ? op.ctrl_idx : 0;
                  const cellIdx = typeof op.cell_idx === 'number'
                    ? op.cell_idx
                    : wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col);
                  wasm.insertTextInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, op.offset, op.text);
                  if (op.style && typeof op.style === 'object') {
                    wasm.applyCharFormatInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, op.offset, op.offset + op.text.length, JSON.stringify(op.style));
                  }
                  appliedCount += 1;
                }
                break;
              case 'delete_range_in_cell':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para_start !== 'number' || typeof op.char_start !== 'number' ||
                    typeof op.cell_para_end !== 'number' || typeof op.char_end !== 'number') {
                  console.warn('[main] delete_range_in_cell payload 미일치 — 무시', op);
                  break;
                }
                {
                  // [4-4 fix] cell_idx + ctrl_idx 우선 + fallback.
                  const ctrlIdx = typeof op.ctrl_idx === 'number' ? op.ctrl_idx : 0;
                  const cellIdx = typeof op.cell_idx === 'number'
                    ? op.cell_idx
                    : wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col);
                  wasm.deleteRangeInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para_start, op.char_start, op.cell_para_end, op.char_end);
                  appliedCount += 1;
                }
                break;
              default:
                console.warn(`[main] Sub-2 미지원 ops op: ${op.op}`);
            }
          } catch (e) {
            console.error('[main] WASM op 적용 실패:', op, e);
          }
        }
        // CanvasView 가 'document-changed' 만 듣고 refreshPages 한다 — IR 만 바꾸고 emit 안 하면
        // 화면이 새로고침 전까지 옛 그림 그대로. InputHandler 도 wasm 편집 직후 같은 이벤트 발행.
        if (appliedCount > 0) {
          // 적용된 마지막 op seq 추적 — page map POST 시 staleness 판정용.
          if (typeof ev.seq === 'number' && ev.seq > lastAppliedSeq) {
            lastAppliedSeq = ev.seq;
          }
          eventBus.emit('document-changed');
        }
      } else if (ev.kind === 'workbench') {
        if (typeof ev.action !== 'string') {
          console.warn('[main] workbench action 필드 누락');
          return;
        }
        const def = getActionDef(ev.action);
        if (!def?.executor) {
          console.warn(`[main] 알 수 없는 hwpctl action: ${ev.action}`);
          return;
        }
        try {
          const ctrl = new HwpCtrl(wasm as any);
          let set: ParameterSet | null = null;
          if (ev.payload && typeof ev.payload === 'object') {
            set = new ParameterSet(def.parameterSetId ?? ev.action);
            for (const [k, v] of Object.entries(ev.payload as Record<string, unknown>)) {
              set.SetItem(k, v);
            }
          }
          def.executor(ctrl, set);
          // ops 분기와 동일 이유 — wasm 변경 후 CanvasView refresh 트리거 필요.
          eventBus.emit('document-changed');
        } catch (e) {
          console.error(`[main] hwpctl executor 예외: ${ev.action}`, e);
        }
      } else if (ev.kind === 'snapshot_restored') {
        if (typeof ev.snapshot_base64 !== 'string') {
          console.warn('[main] snapshot_restored snapshot_base64 누락');
          return;
        }
        try {
          // base64 → Uint8Array
          const binStr = atob(ev.snapshot_base64);
          const bin = new Uint8Array(binStr.length);
          for (let i = 0; i < binStr.length; i++) bin[i] = binStr.charCodeAt(i);
          // wasm-bridge.loadDocument 가 기존 인스턴스 release 후 새 HwpDocument 로 교체.
          wasm.loadDocument(bin);
          eventBus.emit('document-changed');
          console.log(`[main] snapshot 복구 적용 — seq ${ev.seq}`);
        } catch (e) {
          console.error('[main] snapshot_restored 적용 실패 — 새로고침 필요:', e);
        }
      } else if (ev.kind === 'complete') {
        // 워크벤치 종료 시그널. UI 통합은 Sub-3.
        console.log(`[main] 워크벤치 종료 시그널 — seq ${ev.seq}`);
      } else {
        console.warn(`[main] 알 수 없는 ServerEvent kind:`, (ev as { kind?: string }).kind);
      }
    },
  });
}

/**
 * 원본 바이트로 서버 세션을 **생성**하고 미러링을 연결한다.
 * (외부가 fileId 를 명시한 경우 — loadFile({fileId}))
 */
async function createSsrSession(bytes: Uint8Array, fileId: string): Promise<void> {
  try {
    sessionClient?.dispose();
    const client = buildSessionClient(fileId);
    await client.createSession(bytes);
    sessionClient = client;
    currentSsrFileId = fileId;
    syncUrlFileId(fileId);
    if (inputHandler) inputHandler.mirrorSink = client;
    console.info(`[SSR] 세션 생성됨: fileId=${fileId}`);
    // 첫 페이지 맵 역공급 — 첫 ir-slice 호출 전까지 native paginator 가 안 쓰이도록.
    schedulePageMapPost();
  } catch (e) {
    console.warn('[SSR] 세션 생성 실패 — 로컬 편집으로 계속', e);
  }
}

/**
 * 이미 서버에 존재하는 세션에 미러링만 **연결**한다(세션 재생성 없음).
 * 화면 문서는 호출 전에 이미 로드된 상태여야 한다.
 */
function attachSsrMirror(fileId: string): void {
  sessionClient?.dispose();
  const client = buildSessionClient(fileId);
  client.attach();
  sessionClient = client;
  currentSsrFileId = fileId;
  syncUrlFileId(fileId);
  if (inputHandler) inputHandler.mirrorSink = client;
  console.info(`[SSR] 세션 미러링 연결됨: fileId=${fileId}`);
  // 첫 페이지 맵 역공급 — 새로고침으로 attach 한 경우, 화면 로드 후 즉시 서버 정합.
  schedulePageMapPost();
}

/** 문서 바이트를 서버에 업로드(POST /documents)하여 발급된 fileId를 반환한다. */
async function ssrUploadNewDocument(bytes: Uint8Array, filename: string): Promise<string | null> {
  try {
    const docHeaders: Record<string, string> = { 'Content-Type': 'application/json' };
    if (SSR_USER_ID) docHeaders['X-Rhwp-User'] = SSR_USER_ID;
    const res = await fetch(`${SSR_BASE_URL}/documents`, {
      method: 'POST',
      headers: docHeaders,
      body: JSON.stringify({ filename, fileBase64: ssrBytesToBase64(bytes) }),
    });
    if (!res.ok) {
      console.warn('[SSR] 문서 업로드 실패', res.status);
      return null;
    }
    return (await res.json()).fileId ?? null;
  } catch (e) {
    console.warn('[SSR] 문서 업로드 예외', e);
    return null;
  }
}

/** 서버 세션을 메모리에서 해제(DELETE). 영속(sqlite/minio)은 유지된다. */
async function ssrDeleteSession(fileId: string): Promise<void> {
  try {
    const delHeaders: Record<string, string> = {};
    if (SSR_USER_ID) delHeaders['X-Rhwp-User'] = SSR_USER_ID;
    await fetch(`${SSR_BASE_URL}/sessions/${encodeURIComponent(fileId)}`, {
      method: 'DELETE',
      headers: delHeaders,
    });
  } catch {
    /* best-effort */
  }
}

/**
 * 부팅 시 fileId 가 없고 SSR 모드면, 빈 문서를 만들어 업로드(fileId 발급)하고 미러링한다.
 * 한 번도 편집하지 않은 채 다른 문서를 열면 이 세션은 닫힌다(handleSsrLocalOpen).
 */
async function startBlankSsrDocument(): Promise<void> {
  try {
    await createNewDocument();
    const bytes = wasm.exportHwpx();
    const fid = await ssrUploadNewDocument(bytes, 'document.hwpx');
    if (fid) {
      attachSsrMirror(fid);
      currentIsBlank = true;
      console.info(`[SSR] 빈 문서 세션 시작: fileId=${fid}`);
    }
  } catch (e) {
    console.warn('[SSR] 빈 문서 시작 실패', e);
  }
}

/**
 * 부팅 시 fileId 가 있으면 서버 현재 상태(export)를 화면에 복원 로드한다.
 * 서버에 세션이 없어도 GET /export 가 minio download 폴백으로 가져온다.
 */
async function restoreSsrSessionIfNeeded(): Promise<void> {
  if (!SSR_URL_FILE_ID) return;
  if (SSR_PARAMS.get('url')) return; // `?url=` 우선
  try {
    const exportHeaders: Record<string, string> = {};
    if (SSR_USER_ID) exportHeaders['X-Rhwp-User'] = SSR_USER_ID;
    const res = await fetch(
      `${SSR_BASE_URL}/sessions/${encodeURIComponent(SSR_URL_FILE_ID)}/export`,
      { headers: exportHeaders },
    );
    if (!res.ok) return; // 세션·저장소에 없음 — 빈 상태 유지
    const bytes = new Uint8Array(await res.arrayBuffer());
    await loadBytes(bytes, SSR_URL_FILE_ID, null, performance.now(), SSR_URL_FILE_ID, true);
    console.info(`[SSR] 서버 세션 복원 로드 완료: fileId=${SSR_URL_FILE_ID}`);
  } catch (e) {
    console.warn('[SSR] 세션 복원 시도 실패', e);
  }
}

/** SSR 부팅 — fileId 있으면 복원, 없으면 빈 문서 시작. */
async function ssrBootstrap(): Promise<void> {
  if (!SSR_MODE) return;
  if (SSR_URL_FILE_ID) {
    await restoreSsrSessionIfNeeded();
  } else {
    await startBlankSsrDocument();
  }
}


// ─── 커맨드 시스템 ─────────────────────────────
const registry = new CommandRegistry();

function getContext(): EditorContext {
  const hasDoc = wasm.pageCount > 0;
  return {
    hasDocument: hasDoc,
    hasSelection: inputHandler?.hasSelection() ?? false,
    inTable: inputHandler?.isInTable() ?? false,
    inCellSelectionMode: inputHandler?.isInCellSelectionMode() ?? false,
    inTableObjectSelection: inputHandler?.isInTableObjectSelection() ?? false,
    inPictureObjectSelection: inputHandler?.isInPictureObjectSelection() ?? false,
    inField: inputHandler?.isInField() ?? false,
    isEditable: true,
    canUndo: inputHandler?.canUndo() ?? false,
    canRedo: inputHandler?.canRedo() ?? false,
    zoom: canvasView?.getViewportManager().getZoom() ?? 1.0,
    showControlCodes: wasm.getShowControlCodes(),
    isDirty: documentState.isDirty(),
    sourceFormat: hasDoc ? (wasm.getSourceFormat() as 'hwp' | 'hwpx') : undefined,
  };
}

const commandServices: CommandServices = {
  eventBus,
  wasm,
  documentState,
  getContext,
  getInputHandler: () => inputHandler,
  getViewportManager: () => canvasView?.getViewportManager() ?? null,
  // SSR 모드면 저장을 서버 외부 저장소 덮어쓰기로 라우팅.
  saveToServer: SSR_MODE
    ? async () => {
        if (!currentSsrFileId) return false;
        // 디바운스 큐에 남은 편집을 먼저 서버에 반영한 뒤 저장.
        await sessionClient?.flushOps();
        const saveHeaders: Record<string, string> = {};
        if (SSR_USER_ID) saveHeaders['X-Rhwp-User'] = SSR_USER_ID;
        const res = await fetch(
          `${SSR_BASE_URL}/sessions/${encodeURIComponent(currentSsrFileId)}/save`,
          { method: 'POST', headers: saveHeaders },
        );
        if (!res.ok) throw new Error(`save HTTP ${res.status}`);
        return true;
      }
    : undefined,
  // *vfinder save-as picker iframe* 만 띄움 — 결과만 반환. picker 책임 분리로
  // file.ts 가 picker 결과를 *server forward* 와 *client direct vfinder upload* 양쪽
  // 어느 흐름에든 *같은 target* 으로 흘릴 수 있게 한다 (picker 가 한 번만 뜨도록).
  pickVfinderSaveAsTarget: async (suggestedName: string) => {
    return await new Promise<{ path: string; name: string; overwrite: boolean } | null>(
      (resolve) => {
        let settled = false;
        const handle = mountVfinderModal({
          mode: 'save-as',
          suggestedName,
          userId: SSR_USER_ID,
          onSaveAs: (result) => {
            if (settled) return;
            settled = true;
            resolve(result);
          },
          onCancel: () => {
            if (settled) return;
            settled = true;
            resolve(null);
          },
        });
        // 5 분 timeout — modal 자리에서 사용자가 잊었을 자리.
        window.setTimeout(() => {
          if (settled) return;
          settled = true;
          handle.close();
          resolve(null);
        }, 5 * 60 * 1000);
      },
    );
  },
  // SSR 활성 자리 *server-side `/save-as` forward*. picker 결과 (`target`) 를 인자로 받음.
  // server-side 흐름이 깨졌을 때 (502 등) caller 가 *같은 target 으로* client direct
  // vfinder upload 로 흘릴 수 있도록 picker 호출은 본 함수 밖에 둔다.
  forwardSaveAsToServer: SSR_MODE
    ? async (target) => {
        if (!currentSsrFileId) return false;
        await sessionClient?.flushOps();
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (SSR_USER_ID) headers['X-Rhwp-User'] = SSR_USER_ID;
        const res = await fetch(
          `${SSR_BASE_URL}/sessions/${encodeURIComponent(currentSsrFileId)}/save-as`,
          { method: 'POST', headers, body: JSON.stringify(target) },
        );
        if (!res.ok) {
          const body = await res.text().catch(() => '');
          throw new Error(`save-as HTTP ${res.status}: ${body}`);
        }
        const body = (await res.json()) as { fileId: string; path: string };
        const url = new URL(window.location.href);
        url.searchParams.set('fileId', body.fileId);
        window.history.replaceState({}, '', url.toString());
        currentSsrFileId = body.fileId;
        wasm.fileName = target.name;
        return true;
      }
    : undefined,
  // *호환성 보장* — 외부 e2e/툴 자리 `saveAsViaVfinder` 직접 호출이 있을 자리. 내부
  // 구현은 신설 두 함수 조합 (picker + server forward).
  saveAsViaVfinder: SSR_MODE
    ? async () => {
        const target = await commandServices.pickVfinderSaveAsTarget!(wasm.fileName);
        if (!target) return false;
        return await commandServices.forwardSaveAsToServer!(target);
      }
    : undefined,
  // SSR 환경에서 *파일 열기* — rhwp studio 자체 modal 안에 vfinder picker iframe.
  // 사용자가 파일 고르면 URL ?fileId= 갱신 + iframe in-place 재진입 (window.location.replace).
  // 새 file_id 의 세션이 서버 메모리에 없으면 get_or_restore 가 storage.download 로 자동 복원.
  openViaVfinder: SSR_MODE
    ? async () => {
        // 1) modal 띄움 + 사용자 선택 대기
        const picked = await new Promise<{ fileId: string; name: string } | null>(
          (resolve) => {
            let settled = false;
            const handle = mountVfinderModal({
              mode: 'picker',
              kind: 'file',
              userId: SSR_USER_ID,
              onPick: (items) => {
                if (settled) return;
                settled = true;
                const first = items[0];
                if (!first || !first.file_id) {
                  resolve(null);
                  return;
                }
                resolve({ fileId: first.file_id, name: first.name });
              },
              onCancel: () => {
                if (settled) return;
                settled = true;
                resolve(null);
              },
            });
            window.setTimeout(() => {
              if (settled) return;
              settled = true;
              handle.close();
              resolve(null);
            }, 5 * 60 * 1000);
          },
        );

        if (!picked) return false;

        // 2) URL 갱신 + iframe reload — 새 file_id 로 *처음부터 진입*.
        const url = new URL(window.location.href);
        url.searchParams.set('fileId', picked.fileId);
        window.location.replace(url.toString());
        return true;
      }
    : undefined,
  // SSR 비활성 + cross-origin iframe (agent VM) 자리에서 *vfinder /api/upload 직호출*
  // 흐름이 인증에 사용. iframe URL ?user= 그대로 전달.
  vfinderUserId: SSR_USER_ID,
  // vfinder studio base — SSR_BASE_URL 과 다른 자리. rhwp-server 의 SSR base 는
  // `/hwp` 이지만 vfinder 는 `/vfinder` 로 떠 있다. agent 환경 override 가 필요하면
  // URL `?vfinderBase=` 같은 별 파라미터 도입 검토.
  vfinderBase: SSR_PARAMS.get('vfinderBase') ?? '/vfinder',
};

const dispatcher = new CommandDispatcher(registry, commandServices, eventBus);

// 모든 내장 커맨드 등록
registry.registerAll(fileCommands);
registry.registerAll(editCommands);
registry.registerAll(viewCommands);
registry.registerAll(formatCommands);
registry.registerAll(insertCommands);
registry.registerAll(tableCommands);
registry.registerAll(pageCommands);
registry.registerAll(toolCommands);

// 상태 바 요소
const sbMessage = () => document.getElementById('sb-message')!;
const sbPage = () => document.getElementById('sb-page')!;
const sbSection = () => document.getElementById('sb-section')!;
const sbZoomVal = () => document.getElementById('sb-zoom-val')!;

async function initialize(): Promise<void> {
  const msg = sbMessage();
  try {
    msg.textContent = t('main.loading.webfont');
    await loadWebFonts([]);  // CSS @font-face 등록 + CRITICAL 폰트만 로드
    msg.textContent = t('main.loading.wasm');
    await wasm.initialize();
    if (import.meta.env.DEV) {
      initRhwpDev(wasm);
    }
    const renderBackendRequest = resolveRenderBackendRequest(window.location.search);
    const canvaskitMode = resolveCanvasKitRenderMode(window.location.search);
    const canvaskitSurfaceRequest = resolveCanvasKitSurfaceRequest(window.location.search);
    const renderProfile = resolveRenderProfile(window.location.search);
    if (renderBackendRequest.unsupportedReason) {
      console.warn(
        `[main] 지원하지 않는 renderer 값입니다: ${renderBackendRequest.requested}; Canvas2D를 사용합니다.`,
      );
    }
    let renderBackend = renderBackendRequest.backend;
    let canvaskitRenderer: CanvasKitLayerRenderer | null = null;

    if (renderBackend === 'canvaskit') {
      msg.textContent = t('main.loading.canvaskit');
      try {
        const { CanvasKitLayerRenderer } = await import('@/view/canvaskit-renderer');
        canvaskitRenderer = await CanvasKitLayerRenderer.create(canvaskitMode, canvaskitSurfaceRequest);
      } catch (error) {
        console.error('[main] CanvasKit 초기화 실패, Canvas2D로 폴백합니다:', error);
        renderBackend = 'canvas2d';
      }
    }
    msg.textContent = t('main.prompt.select_file');

    const container = document.getElementById('scroll-container')!;
    canvasView = new CanvasView(
      container,
      wasm,
      eventBus,
      renderBackend,
      renderProfile,
      canvaskitRenderer,
    );

    // 눈금자 초기화
    ruler = new Ruler(
      document.getElementById('h-ruler') as HTMLCanvasElement,
      document.getElementById('v-ruler') as HTMLCanvasElement,
      container,
      eventBus,
      wasm,
      canvasView.getVirtualScroll(),
      canvasView.getViewportManager(),
    );

    inputHandler = new InputHandler(
      container, wasm, eventBus,
      canvasView.getVirtualScroll(),
      canvasView.getViewportManager(),
    );

    toolbar = new Toolbar(document.getElementById('style-bar')!, wasm, eventBus, dispatcher);
    toolbar.setEnabled(false);

    // InputHandler에 커맨드 디스패처 및 컨텍스트 메뉴 주입
    inputHandler.setDispatcher(dispatcher);
    inputHandler.setContextMenu(new ContextMenu(dispatcher, registry));
    inputHandler.setCommandPalette(new CommandPalette(registry, dispatcher));
    inputHandler.setCellSelectionRenderer(
      new CellSelectionRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setTableObjectRenderer(
      new TableObjectRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setTableResizeRenderer(
      new TableResizeRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setPictureObjectRenderer(
      new TableObjectRenderer(container, canvasView.getVirtualScroll(), true),
    );

    new MenuBar(document.getElementById('menu-bar')!, eventBus, dispatcher);

    // 툴바 내 data-cmd 버튼 클릭 → 커맨드 디스패치
    document.querySelectorAll('.tb-btn[data-cmd]').forEach(btn => {
      btn.addEventListener('mousedown', (e) => {
        e.preventDefault();
        const cmd = (btn as HTMLElement).dataset.cmd;
        if (cmd) dispatcher.dispatch(cmd, { anchorEl: btn as HTMLElement });
      });
    });

    // 스플릿 버튼 드롭다운 메뉴
    document.querySelectorAll('.tb-split').forEach(split => {
      const arrow = split.querySelector('.tb-split-arrow');
      if (arrow) {
        arrow.addEventListener('mousedown', (e) => {
          e.preventDefault();
          e.stopPropagation();
          // 다른 열린 메뉴 닫기
          document.querySelectorAll('.tb-split.open').forEach(s => {
            if (s !== split) s.classList.remove('open');
          });
          split.classList.toggle('open');
        });
      }
      split.querySelectorAll('.tb-split-item[data-cmd]').forEach(item => {
        item.addEventListener('mousedown', (e) => {
          e.preventDefault();
          split.classList.remove('open');
          const cmd = (item as HTMLElement).dataset.cmd;
          if (cmd) dispatcher.dispatch(cmd, { anchorEl: item as HTMLElement });
        });
      });
    });
    // 외부 클릭 시 스플릿 메뉴 닫기
    document.addEventListener('mousedown', () => {
      document.querySelectorAll('.tb-split.open').forEach(s => s.classList.remove('open'));
    });

    // #780: 도구 모음/서식 도구 모음 영역 mousedown 시 focus 이동 방지
    // — 편집 영역의 텍스트 선택(cursor.anchor)이 보존되어야 서식 적용이 동작함
    for (const id of ['icon-toolbar', 'style-bar']) {
      const el = document.getElementById(id);
      if (el) el.addEventListener('mousedown', (e) => {
        if ((e.target as HTMLElement).tagName !== 'INPUT' && (e.target as HTMLElement).tagName !== 'SELECT') {
          e.preventDefault();
        }
      });
    }

    setupFileInput();
    setupZoomControls();
    setupEventListeners();
    setupGlobalShortcuts();
    loadFromUrlParam();
    // SSR: fileId 있으면 서버 상태 복원, 없으면 빈 문서 업로드로 세션 시작.
    void ssrBootstrap();

    // E2E 테스트용 전역 노출 (개발 모드 전용)
    if (import.meta.env.DEV) {
      (window as any).__inputHandler = inputHandler;
      (window as any).__canvasView = canvasView;
      (window as any).__renderBackend = renderBackend;
      (window as any).__canvaskitRenderMode = canvaskitMode;
      (window as any).__canvaskitSurfaceRequest = canvaskitSurfaceRequest;
      (window as any).__renderProfile = renderProfile;
      (window as any).__ssr = {
        get fileId() { return currentSsrFileId; },
        get isBlank() { return currentIsBlank; },
      };
      (window as any).__dispatcher = dispatcher;
    }
  } catch (error) {
    msg.textContent = t('main.error.wasm_init_failed', { error: String(error) });
    console.error('[main] WASM 초기화 실패:', error);
  }
}

/**
 * 전역 단축키 핸들러 — InputHandler.active 여부와 무관하게 동작해야 하는 단축키.
 * 예: 문서 미로드 상태에서도 Alt+N(새 문서), Ctrl+O(열기) 등.
 */
function setupGlobalShortcuts(): void {
  document.addEventListener('keydown', (e) => {
    // input/textarea 등 편집 가능 요소 내부에서는 무시
    const target = e.target as HTMLElement;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return;
    // InputHandler가 활성 상태이면 자체 처리에 맡김
    if (inputHandler?.isActive()) return;

    const ctrlOrMeta = e.ctrlKey || e.metaKey;

    // Alt+N / Alt+ㅜ → 새 문서 (문서 미로드 상태에서도 동작)
    if (e.altKey && !ctrlOrMeta && !e.shiftKey) {
      if (e.key === 'n' || e.key === 'N' || e.key === 'ㅜ') {
        e.preventDefault();
        dispatcher.dispatch('file:new-doc');
        return;
      }
    }
    // Ctrl/Cmd+O → 열기 (문서 미로드 상태에서도 동작)
    if (ctrlOrMeta && !e.altKey && !e.shiftKey) {
      if (e.key === 'o' || e.key === 'O' || e.key === 'ㅐ') {
        e.preventDefault();
        dispatcher.dispatch('file:open');
        return;
      }
    }
  }, false);
}

function setupFileInput(): void {
  const fileInput = document.getElementById('file-input') as HTMLInputElement;

  fileInput.addEventListener('change', async (e) => {
    const input = e.target as HTMLInputElement;
    const skipUnsavedGuard = input.dataset.skipUnsavedGuard === 'true';
    delete input.dataset.skipUnsavedGuard;
    const file = input.files?.[0];
    if (!file) return;
    const name = file.name.toLowerCase();
    if (!name.endsWith('.hwp') && !name.endsWith('.hwpx')) {
      alert(t('main.error.hwp_only'));
      fileInput.value = '';
      return;
    }
    await loadFile(file, { skipUnsavedGuard });
    fileInput.value = '';
  });

  // 문서 전체에서 브라우저 기본 드롭 동작 방지 (파일 열기/다운로드 방지)
  document.addEventListener('dragover', (e) => e.preventDefault());
  document.addEventListener('drop', (e) => e.preventDefault());

  // 드래그 앤 드롭 지원 (scroll-container 영역)
  const container = document.getElementById('scroll-container')!;
  container.addEventListener('dragover', (e) => {
    e.preventDefault();
    container.classList.add('drag-over');
  });
  container.addEventListener('dragleave', () => {
    container.classList.remove('drag-over');
  });
  container.addEventListener('drop', async (e) => {
    e.preventDefault();
    container.classList.remove('drag-over');
    const file = e.dataTransfer?.files[0];
    if (!file) return;
    const dropName = file.name.toLowerCase();
    const imageExts = ['.png', '.jpg', '.jpeg', '.gif', '.bmp', '.webp'];
    if (imageExts.some(ext => dropName.endsWith(ext))) {
      if (!inputHandler || wasm.pageCount === 0) return;
      const data = new Uint8Array(await file.arrayBuffer());
      const ext = file.name.split('.').pop()?.toLowerCase() || 'png';
      const img = new Image();
      const url = URL.createObjectURL(file);
      try {
        img.src = url;
        await img.decode();
        inputHandler.enterImagePlacementMode(data, ext, img.naturalWidth, img.naturalHeight, file.name);
      } catch {
        console.warn('[drop] 이미지 디코딩 실패:', file.name);
      } finally {
        URL.revokeObjectURL(url);
      }
      return;
    }
    if (!dropName.endsWith('.hwp') && !dropName.endsWith('.hwpx')) {
      alert(t('main.error.hwp_or_image_only'));
      return;
    }
    await loadFile(file);
  });
}

function setupZoomControls(): void {
  if (!canvasView) return;
  const vm = canvasView.getViewportManager();

  document.getElementById('sb-zoom-in')!.addEventListener('click', () => {
    vm.setZoom(vm.getZoom() + 0.1);
  });
  document.getElementById('sb-zoom-out')!.addEventListener('click', () => {
    vm.setZoom(vm.getZoom() - 0.1);
  });

  // 폭 맞춤: 용지 폭에 맞게 줌 조절
  document.getElementById('sb-zoom-fit-width')!.addEventListener('click', () => {
    if (wasm.pageCount === 0) return;
    const container = document.getElementById('scroll-container')!;
    const containerWidth = container.clientWidth - 40; // 좌우 여백 제외
    const pageInfo = wasm.getPageInfo(0);
    // pageInfo.width는 이미 px 단위 (96dpi 기준)
    const zoom = containerWidth / pageInfo.width;
    console.log(`[zoom-fit-width] container=${containerWidth} page=${pageInfo.width} zoom=${zoom.toFixed(3)}`);
    vm.setZoom(Math.max(0.1, Math.min(zoom, 4.0)));
  });

  // 쪽 맞춤: 한 페이지 전체가 보이도록 줌 조절
  document.getElementById('sb-zoom-fit')!.addEventListener('click', () => {
    if (wasm.pageCount === 0) return;
    const container = document.getElementById('scroll-container')!;
    const containerWidth = container.clientWidth - 40;
    const containerHeight = container.clientHeight - 40;
    const pageInfo = wasm.getPageInfo(0);
    // pageInfo.width/height는 이미 px 단위 (96dpi 기준)
    const zoomW = containerWidth / pageInfo.width;
    const zoomH = containerHeight / pageInfo.height;
    console.log(`[zoom-fit-page] containerW=${containerWidth} containerH=${containerHeight} pageW=${pageInfo.width} pageH=${pageInfo.height} zoomW=${zoomW.toFixed(3)} zoomH=${zoomH.toFixed(3)}`);
    vm.setZoom(Math.max(0.1, Math.min(zoomW, zoomH, 4.0)));
  });

  // 모바일: 줌 값 클릭 → 100% 토글
  document.getElementById('sb-zoom-val')!.addEventListener('click', () => {
    const currentZoom = vm.getZoom();
    if (Math.abs(currentZoom - 1.0) < 0.05) {
      // 현재 100% → 쪽 맞춤으로 전환
      document.getElementById('sb-zoom-fit')!.click();
    } else {
      // 현재 쪽 맞춤/기타 → 100%로 전환
      vm.setZoom(1.0);
    }
  });

  document.addEventListener('keydown', (e) => {
    if (!e.ctrlKey && !e.metaKey) return;
    if (e.key === '=' || e.key === '+') {
      e.preventDefault();
      vm.setZoom(vm.getZoom() + 0.1);
    } else if (e.key === '-') {
      e.preventDefault();
      vm.setZoom(vm.getZoom() - 0.1);
    } else if (e.key === '0') {
      e.preventDefault();
      vm.setZoom(1.0);
    }
  });
}

let totalSections = 1;

function setupEventListeners(): void {
  eventBus.on('current-page-changed', (page, _total) => {
    const pageIdx = page as number;
    sbPage().textContent = t('statusbar.page', { current: pageIdx + 1, total: _total as number });

    // 구역 정보: 현재 페이지의 sectionIndex로 갱신
    if (wasm.pageCount > 0) {
      try {
        const pageInfo = wasm.getPageInfo(pageIdx);
        sbSection().textContent = t('statusbar.section', { current: pageInfo.sectionIndex + 1, total: totalSections });
      } catch { /* 무시 */ }
    }
  });

  eventBus.on('zoom-level-display', (zoom) => {
    sbZoomVal().textContent = `${Math.round((zoom as number) * 100)}%`;
  });

  // 삽입/수정 모드 토글
  eventBus.on('insert-mode-changed', (insertMode) => {
    document.getElementById('sb-mode')!.textContent = (insertMode as boolean) ? t('statusbar.insert_mode') : t('statusbar.overwrite_mode');
  });

  eventBus.on('document-mutated', (reason) => {
    documentState.markDirty(typeof reason === 'string' ? reason : 'document-mutated');
  });

  eventBus.on('document-changed', (reason) => {
    documentState.markDirty(typeof reason === 'string' ? reason : 'document-changed');
    // 측정기 격차 우회 — 페이지 경계가 *브라우저 진실* 이 되도록 서버에 역공급.
    schedulePageMapPost();
  });

  eventBus.on('document-dirty-changed', () => {
    eventBus.emit('command-state-changed');
  });

  // 필드 정보 표시
  const sbField = document.getElementById('sb-field');
  eventBus.on('field-info-changed', (info) => {
    if (!sbField) return;
    const fi = info as { fieldId: number; fieldType: string; guideName?: string } | null;
    if (fi) {
      const label = fi.guideName || `#${fi.fieldId}`;
      sbField.textContent = t('statusbar.field_label', { label });
      sbField.style.display = '';
    } else {
      sbField.textContent = '';
      sbField.style.display = 'none';
    }
  });

  // 개체 선택 시 회전/대칭 버튼 그룹 표시/숨김
  const rotateGroup = document.querySelector('.tb-rotate-group') as HTMLElement | null;
  if (rotateGroup) {
    eventBus.on('picture-object-selection-changed', (selected) => {
      rotateGroup.style.display = (selected as boolean) ? '' : 'none';
    });
  }

  // 머리말/꼬리말 편집 모드 시 도구상자 전환 + 본문 dimming
  const hfGroup = document.querySelector('.tb-headerfooter-group') as HTMLElement | null;
  const hfLabel = hfGroup?.querySelector('.tb-hf-label') as HTMLElement | null;
  const defaultTbGroups = document.querySelectorAll('#icon-toolbar > .tb-group:not(.tb-headerfooter-group):not(.tb-rotate-group), #icon-toolbar > .tb-sep');
  const scrollContainer = document.getElementById('scroll-container');
  const styleBar = document.getElementById('style-bar');

  eventBus.on('headerFooterModeChanged', (mode) => {
    const isActive = (mode as string) !== 'none';
    // 도구상자 전환
    if (hfGroup) {
      hfGroup.style.display = isActive ? '' : 'none';
    }
    if (hfLabel) {
      hfLabel.textContent = (mode as string) === 'header' ? t('menu.page.header') : (mode as string) === 'footer' ? t('menu.page.footer') : '';
    }
    defaultTbGroups.forEach((el) => {
      (el as HTMLElement).style.display = isActive ? 'none' : '';
    });
    // 서식 도구 모음은 머리말/꼬리말 편집 시에도 유지 (문단/글자 모양 설정 필요)
    // 본문 dimming
    if (scrollContainer) {
      if (isActive) {
        scrollContainer.classList.add('hf-editing');
      } else {
        scrollContainer.classList.remove('hf-editing');
      }
    }
  });
}

/** 문서 초기화 공통 시퀀스 (loadFile, createNewDocument 양쪽에서 사용) */
async function initializeDocument(docInfo: DocumentInfo, displayName: string): Promise<void> {
  const msg = sbMessage();
  let normalizedDuringLoad = false;
  try {
    console.log('[initDoc] 1. 폰트 로딩 시작');
    if (docInfo.fontsUsed?.length) {
      await loadWebFonts(docInfo.fontsUsed, (loaded, total) => {
        msg.textContent = t('main.loading.fonts_progress', { loaded, total });
      });
    }
    console.log('[initDoc] 2. 폰트 로딩 완료');
    msg.textContent = displayName;
    totalSections = docInfo.sectionCount ?? 1;
    sbSection().textContent = t('statusbar.section', { current: 1, total: totalSections });
    console.log('[initDoc] 3. inputHandler deactivate');
    inputHandler?.deactivate();
    console.log('[initDoc] 4. canvasView loadDocument');
    canvasView?.loadDocument();
    console.log('[initDoc] 5. toolbar setEnabled');
    toolbar?.setEnabled(true);
    console.log('[initDoc] 6. toolbar initFontDropdown + initStyleDropdown');
    toolbar?.initFontDropdown(docInfo.fontsUsed);
    toolbar?.initStyleDropdown();
    console.log('[initDoc] 7. inputHandler activateWithCaretPosition');
    inputHandler?.activateWithCaretPosition();
    console.log('[initDoc] 8. 완료');

    // #177: HWPX 비표준 lineseg 감지 → 경고 있으면 모달로 사용자 선택 요청
    try {
      const report = wasm.getValidationWarnings();
      console.log(`[validation] ${report.count} warnings`, report.summary);
      if (report.count > 0) {
        const choice = await showValidationModalIfNeeded(report);
        console.log(`[validation] user choice: ${choice}`);
        if (choice === 'auto-fix') {
          const n = wasm.reflowLinesegs();
          console.log(`[validation] reflowed ${n} paragraphs`);
          // 렌더 재계산
          canvasView?.loadDocument();
          msg.textContent = t('main.lineseg_auto_fixed', { displayName, n });
          normalizedDuringLoad = n > 0;
        }
      }
    } catch (e) {
      console.warn('[validation] 감지/보정 실패 (치명적이지 않음):', e);
    }
    if (normalizedDuringLoad) {
      documentState.markDirty('validation-auto-fix');
    } else {
      documentState.markClean('document-initialized');
    }
  } catch (error) {
    console.error('[initDoc] 오류:', error);
    if (window.innerWidth < 768) alert(t('error.client.init_failed', { message: String(error) }));
  }
}

async function loadFile(file: File, options: { skipUnsavedGuard?: boolean } = {}): Promise<boolean> {
  const msg = sbMessage();
  try {
    if (!options.skipUnsavedGuard) {
      const canReplace = await confirmSaveBeforeReplacingDocument(commandServices);
      if (!canReplace) return false;
    }
    msg.textContent = t('main.loading.file');
    const startTime = performance.now();
    const data = new Uint8Array(await file.arrayBuffer());
    await loadBytes(data, file.name, null, startTime);
    return true;
  } catch (error) {
    showLoadError(error);
    return false;
  }
}

async function loadBytes(
  data: Uint8Array,
  fileName: string,
  fileHandle: typeof wasm.currentFileHandle,
  startTime = performance.now(),
  fileId: string | null = null,
  restore = false,
): Promise<void> {
  // SSR 로컬 열기 분기를 위해, 새 문서 로드(=markClean) 전에 직전 세션의
  // "빈 문서 + 미편집" 여부를 캡처한다.
  const prevBlankUnedited =
    SSR_MODE && !restore && !fileId && currentIsBlank && currentSsrFileId != null && !documentState.isDirty();
  const prevFileId = currentSsrFileId;

  const docInfo = wasm.loadDocument(data, fileName);
  wasm.currentFileHandle = fileHandle;
  const elapsed = performance.now() - startTime;
  // initializeDocument 안에서 #177 validation 모달이 표시될 수 있음.
  // HWPX 토스트는 모달과의 이벤트 충돌을 피하기 위해 모달 닫힌 후 표시.
  await initializeDocument(docInfo, t('main.doc_loaded_summary', { fileName, pages: docInfo.pageCount, ms: elapsed.toFixed(1) }));
  notifyHwpxSaveModeIfNeeded();

  // SSR 세션 연결
  if (restore && fileId) {
    // 부팅 복원: 서버 기존 세션에 미러링만 연결
    attachSsrMirror(fileId);
    currentIsBlank = false;
  } else if (fileId) {
    // 외부가 fileId 명시(loadFile({fileId})): 그 fileId로 세션 생성
    await createSsrSession(data, fileId);
    currentIsBlank = false;
  } else if (SSR_MODE && currentSsrFileId && !prevBlankUnedited) {
    // [post-Sub-2 fix] 로컬 "열기" — 기존 세션 fileId 유지 + 서버 core 만 교체.
    // URL 의 fileId 가 보존되므로 노트북·외부 호출자도 동일 fileId 로 새 문서 IR 조회 가능.
    // sessionClient.requestSnapshot() 는 현재 wasm 의 export bytes 를 ClientMessage::Snapshot 으로
    // WS 전송 → 서버 ws.rs::handle_client_text 의 Snapshot 분기가 session.core 를
    // DocumentCore::from_bytes(...) 로 통째 교체.
    sessionClient?.requestSnapshot();
    currentIsBlank = false;
  } else if (SSR_MODE) {
    // 직전 세션이 빈 문서 + 미편집이거나 기존 세션 없음 → 새 fileId 발급 경로.
    // minio 업로드로 새 fileId 발급 후 미러링.
    if (prevBlankUnedited && prevFileId) await ssrDeleteSession(prevFileId);
    const fid = await ssrUploadNewDocument(data, fileName);
    if (fid) {
      attachSsrMirror(fid);
      currentIsBlank = false;
    }
  }
}

/**
 * #888: HWPX 출처 문서 로드 시 HWP 변환 저장 안내.
 * - 우상단 토스트 1회
 * - 상태 표시줄 메시지
 */
function notifyHwpxSaveModeIfNeeded(): void {
  if (wasm.getSourceFormat() !== 'hwpx') return;

  showToast({
    message: t('toast.hwpx_to_hwp_notice'),
    durationMs: 0, // 자동 페이드 없음 — 사용자가 확인 버튼으로 닫음
    action: {
      label: t('toast.action_issue'),
      onClick: () => {
        window.open('https://github.com/edwardkim/rhwp/issues/888', '_blank');
      },
    },
    confirmLabel: t('toast.action_ok'),
  });

  const sb = sbMessage();
  if (sb) sb.textContent = t('main.statusbar.hwpx_convert_mode');
}

type DocumentByteKind = 'hwp' | 'hwpx' | 'html' | 'unknown';

const HWP_CFB_SIGNATURE = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] as const;
const ZIP_SIGNATURES = [
  [0x50, 0x4B, 0x03, 0x04],
  [0x50, 0x4B, 0x05, 0x06],
  [0x50, 0x4B, 0x07, 0x08],
] as const;

function startsWithBytes(bytes: Uint8Array, signature: readonly number[]): boolean {
  if (bytes.length < signature.length) return false;
  return signature.every((byte, index) => bytes[index] === byte);
}

function detectDocumentByteKind(bytes: Uint8Array, contentType?: string | null): DocumentByteKind {
  if (startsWithBytes(bytes, HWP_CFB_SIGNATURE)) return 'hwp';
  if (ZIP_SIGNATURES.some(signature => startsWithBytes(bytes, signature))) return 'hwpx';

  const declaredContentType = contentType?.toLowerCase() ?? '';
  if (declaredContentType.includes('text/html')) return 'html';

  const prefix = new TextDecoder('utf-8')
    .decode(bytes.subarray(0, Math.min(bytes.length, 256)))
    .trimStart()
    .toLowerCase();

  if (prefix.startsWith('<!doctype') || prefix.startsWith('<html') || prefix.startsWith('<?xml')) {
    return 'html';
  }

  return 'unknown';
}

function assertRemoteDocumentBytes(bytes: Uint8Array, contentType?: string | null): void {
  const kind = detectDocumentByteKind(bytes, contentType);
  if (kind === 'hwp' || kind === 'hwpx') return;

  if (kind === 'html') {
    throw new Error(t('main.error.not_hwp_html_preview'));
  }

  throw new Error(t('main.error.not_hwp_unknown_signature'));
}

async function createNewDocument(): Promise<void> {
  const msg = sbMessage();
  try {
    msg.textContent = t('main.creating_new_doc');
    const docInfo = wasm.createNewDocument();
    await initializeDocument(docInfo, t('main.new_doc_summary', { pages: docInfo.pageCount }));
  } catch (error) {
    msg.textContent = t('main.error.new_doc_failed', { error: String(error) });
    console.error('[main] 새 문서 생성 실패:', error);
  }
}

async function canReplaceCurrentDocument(skipUnsavedGuard?: boolean): Promise<boolean> {
  return skipUnsavedGuard === true || await confirmSaveBeforeReplacingDocument(commandServices);
}

// 커맨드에서 새 문서 생성 호출
eventBus.on('create-new-document', (payload) => {
  void (async () => {
    const options = payload as { skipUnsavedGuard?: boolean } | undefined;
    if (!await canReplaceCurrentDocument(options?.skipUnsavedGuard)) return;
    await createNewDocument();
  })();
});
eventBus.on('open-document-bytes', async (payload) => {
  const data = payload as {
    bytes: Uint8Array;
    fileName: string;
    fileHandle: typeof wasm.currentFileHandle;
    skipUnsavedGuard?: boolean;
    /** 문서 비교 등: 로드 완료를 기다리는 쪽과 짝을 맞출 때만 전달 */
    requestId?: string;
  };
  const notifyDone = (ok: boolean, error?: string) => {
    if (!data.requestId) return;
    eventBus.emit('open-document-bytes:done', { requestId: data.requestId, ok, error });
  };
  try {
    if (!await canReplaceCurrentDocument(data.skipUnsavedGuard)) {
      notifyDone(false, t('main.error.doc_open_cancelled'));
      return;
    }
    await loadBytes(data.bytes, data.fileName, data.fileHandle);
    notifyDone(true);
  } catch (error) {
    // #265: WASM 파서 에러 (예: HWP 3.0 미지원) 를 사용자에게 전파
    showLoadError(error);
    const msg = error instanceof Error ? error.message : String(error);
    notifyDone(false, msg);
  }
});

// 수식 더블클릭 → 수식 편집 대화상자
eventBus.on('equation-edit-request', () => {
  dispatcher.dispatch('insert:equation-edit');
});

/**
 * URL 파라미터(?url=)로 전달된 HWP 파일을 자동 로드한다.
 * Chrome 확장 프로그램에서 뷰어 탭을 열 때 사용.
 */
async function loadFromUrlParam(): Promise<void> {
  const params = new URLSearchParams(window.location.search);
  const fileUrl = params.get('url');
  if (!fileUrl) return;

  const fileName = params.get('filename') || fileUrl.split('/').pop()?.split('?')[0] || 'document.hwp';
  const msg = sbMessage();

  try {
    msg.textContent = t('main.loading.file');
    console.log(`[loadFromUrlParam] ${fileUrl}`);

    let response: Response;

    // Chrome 확장 환경: Service Worker를 통한 CORS 우회 fetch
    if (typeof chrome !== 'undefined' && chrome.runtime?.sendMessage) {
      try {
        response = await fetch(fileUrl);
      } catch {
        // 직접 fetch 실패 시 Service Worker 프록시
        const result = await chrome.runtime.sendMessage({ type: 'fetch-file', url: fileUrl });
        if (result.error) throw new Error(result.error);
        const data = new Uint8Array(result.data);
        assertRemoteDocumentBytes(data);
        await loadBytes(data, fileName, null);
        return;
      }
    } else {
      response = await fetch(fileUrl);
    }

    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const contentType = response.headers.get('content-type');
    const buffer = await response.arrayBuffer();
    const data = new Uint8Array(buffer);
    assertRemoteDocumentBytes(data, contentType);
    await loadBytes(data, fileName, null);
  } catch (error) {
    // 로컬 file:// 로드 실패 + "파일 URL 액세스 허용" 미허용 → 전용 안내 (#1131)
    if (fileUrl.startsWith('file:') && typeof chrome !== 'undefined') {
      const allowed = await isFileSchemeAccessAllowed();
      if (allowed === false) {
        showFileUrlAccessGuidance();
        return;
      }
    }
    showLoadError(error);
  }
}

/**
 * 확장 프로그램의 "파일 URL에 대한 액세스 허용" 권한 상태를 조회한다 (#1131).
 *
 * 확장 페이지에서만 의미가 있다. API 부재(비-확장 환경 등) 시 판정 불가로
 * `null` 을 반환하여 호출부가 기존 동작(일반 에러)으로 폴백하도록 한다.
 *
 * @returns 허용=true, 미허용=false, 판정 불가=null
 */
async function isFileSchemeAccessAllowed(): Promise<boolean | null> {
  const ext = (typeof chrome !== 'undefined' ? chrome.extension : undefined) as
    | { isAllowedFileSchemeAccess?: () => Promise<boolean> }
    | undefined;
  if (!ext?.isAllowedFileSchemeAccess) return null;
  try {
    return await ext.isAllowedFileSchemeAccess();
  } catch {
    return null;
  }
}

/**
 * 로컬 file:// 문서를 열 때 "파일 URL 액세스 허용" 권한이 꺼져 있어 로드가
 * 실패한 경우, 일반 "Failed to fetch" 대신 원인과 해결 방법을 안내한다 (#1131).
 *
 * 설정 화면(chrome://extensions/?id=...)은 일반 링크로는 열리지 않으므로
 * 확장 컨텍스트의 chrome.tabs.create 로 연다.
 */
function showFileUrlAccessGuidance(): void {
  const errMsg = t('main.error.file_url_access_denied_long');
  const sb = sbMessage();
  if (sb) sb.textContent = t('main.error.file_url_access_denied_short');
  console.error('[main] file:// 로드 실패 — 파일 URL 액세스 미허용 (#1131)');
  showToast({
    message: errMsg,
    durationMs: 0, // 사용자가 읽고 직접 닫기
    confirmLabel: t('toast.action_ok'),
    action: {
      label: t('toast.action_settings'),
      onClick: () => {
        if (typeof chrome !== 'undefined' && chrome.tabs?.create && chrome.runtime?.id) {
          chrome.tabs.create({ url: `chrome://extensions/?id=${chrome.runtime.id}` });
        }
      },
    },
  });
}

/**
 * 파일 로드 실패 시 사용자에게 에러를 명확히 알린다 (#265).
 *
 * 상태 표시줄은 22px 한 줄로 긴 에러 메시지가 ellipsis 로 잘리므로,
 * 우상단 토스트 (긴 메시지 줄바꿈 지원 · 사용자 닫기 · action 링크) 를
 * 병행 사용한다.
 */
function showLoadError(error: unknown): void {
  const raw = String(error).replace(/^Error:\s*/, '');
  const errMsg = t('main.error.file_load_failed', { reason: raw });
  const sb = sbMessage();
  if (sb) sb.textContent = errMsg;
  console.error('[main] 파일 로드 실패:', error);
  showToast({
    message: errMsg,
    durationMs: 0, // 에러는 자동 페이드 없음 — 사용자가 읽고 닫기
    confirmLabel: t('toast.action_ok'),
  });
}

const initPromise = initialize();

// ── iframe 연동 API (postMessage) ──
// 부모 페이지에서 postMessage로 에디터를 제어할 수 있다.
// 요청: { type: 'rhwp-request', id, method, params }
// 응답: { type: 'rhwp-response', id, result?, error? }
window.addEventListener('message', async (e) => {
  const msg = e.data;
  if (!msg || typeof msg !== 'object') return;

  // 기존 hwpctl-load 호환
  if (msg.type === 'hwpctl-load' && msg.data) {
    try {
      await initPromise;
      if (!await canReplaceCurrentDocument(Boolean(msg.skipUnsavedGuard))) {
        e.source?.postMessage({ type: 'rhwp-response', id: msg.id, error: t('main.error.doc_open_cancelled') }, { targetOrigin: '*' });
        return;
      }
      const bytes = new Uint8Array(msg.data);
      await loadBytes(bytes, msg.fileName || 'document.hwp', null, performance.now(), msg.fileId ?? null);
      e.source?.postMessage({ type: 'rhwp-response', id: msg.id, result: { pageCount: wasm.pageCount } }, { targetOrigin: '*' });
    } catch (err: any) {
      e.source?.postMessage({ type: 'rhwp-response', id: msg.id, error: err.message || String(err) }, { targetOrigin: '*' });
    }
    return;
  }

  // rhwp-request: 범용 API
  if (msg.type !== 'rhwp-request' || !msg.method) return;
  const { id, method, params } = msg;
  const reply = (result?: any, error?: string) => {
    e.source?.postMessage({ type: 'rhwp-response', id, result, error }, { targetOrigin: '*' });
  };

  try {
    switch (method) {
      case 'ready':
        // wasm 초기화 완료 후에만 true 응답 — race condition 방지 (#522)
        await initPromise;
        reply(true);
        break;
      case 'loadFile': {
        await initPromise;
        if (!await canReplaceCurrentDocument(Boolean(params?.skipUnsavedGuard))) {
          reply(undefined, t('main.error.doc_open_cancelled'));
          break;
        }
        const bytes = new Uint8Array(params.data);
        await loadBytes(bytes, params.fileName || 'document.hwp', null, performance.now(), params.fileId ?? null);
        reply({ pageCount: wasm.pageCount });
        break;
      }
      case 'pageCount':
        await initPromise;
        reply(wasm.pageCount);
        break;
      case 'getPageSvg':
        await initPromise;
        reply(wasm.renderPageSvg(params.page ?? 0));
        break;
      case 'exportHwp':
        await initPromise;
        reply(Array.from(wasm.exportHwp()));
        break;
      case 'exportHwpx':
        await initPromise;
        reply(Array.from(wasm.exportHwpx()));
        break;
      case 'exportHwpVerify':
        await initPromise;
        reply(JSON.parse(wasm.exportHwpVerify()));
        break;
      default:
        reply(undefined, `Unknown method: ${method}`);
    }
  } catch (err: any) {
    reply(undefined, err.message || String(err));
  }
});
