import type { CommandDef, CommandServices, SaveAsTarget } from '../types';
import { PageSetupDialog } from '@/ui/page-setup-dialog';
import { AboutDialog } from '@/ui/about-dialog';
import { showSaveAs } from '@/ui/save-as-dialog';
import { showUnsavedChangesDialog } from '@/ui/unsaved-changes-dialog';
import { t } from '@/i18n/t';
import {
  pickOpenFileHandle,
  readFileFromHandle,
  saveDocumentToFileSystem,
  isCrossOriginEmbedded,
  type FileSystemFileHandleLike,
  type FileSystemWindowLike,
} from '@/command/file-system-access';
import { uploadToVfinder } from '@/command/vfinder-upload';
import { mountVfinderModal } from '@/view/vfinder-modal';

/** [Task #833] 사용자 명시 cancel 에러 검출.
 * - AbortError: showSaveFilePicker / showOpenFilePicker 다이얼로그 취소
 * - NotAllowedError: writeBlobToHandle 권한 거부 (Chrome "변경사항 저장" 프롬프트 취소)
 *
 * 두 케이스 모두 fallback download 우회 — 사용자가 명시적으로 취소했으므로
 * 의도하지 않은 Downloads 폴더 저장 + chrome-extension viewer 자동 연결 차단. */
function isUserCancelError(e: unknown): boolean {
  return e instanceof DOMException
      && (e.name === 'AbortError' || e.name === 'NotAllowedError');
}

function hwpSaveFileName(fileName: string): string {
  const trimmed = fileName.trim() || 'document.hwp';
  if (/\.(hwp|hwpx)$/i.test(trimmed)) {
    return trimmed.replace(/\.(hwp|hwpx)$/i, '.hwp');
  }
  return `${trimmed}.hwp`;
}

function hwpSaveBaseName(fileName: string): string {
  return hwpSaveFileName(fileName).replace(/\.hwp$/i, '');
}

function hwpSaveCurrentHandle(
  sourceFormat: string,
  handle: FileSystemFileHandleLike | null,
): FileSystemFileHandleLike | null {
  if (sourceFormat === 'hwpx' && handle && !handle.name.toLowerCase().endsWith('.hwp')) {
    return null;
  }
  return handle;
}

export type SaveCurrentDocumentResult = 'saved' | 'cancelled' | 'failed' | 'unsupported';

/**
 * vfinder save-as picker iframe modal 자리. *picker 한 번만* 띄우고 결과 반환.
 *
 * `services.pickVfinderSaveAsTarget` 가 설정된 자리 (SSR 모드 + non-SSR 양쪽 모두
 * main.ts 에서 정의) 자리 그쪽 위임. 미설정 자리 (e2e/스탠드얼론) 로컬 inline 호출.
 */
async function pickSaveAsTarget(
  services: CommandServices,
  suggestedName: string,
): Promise<SaveAsTarget | null> {
  if (services.pickVfinderSaveAsTarget) {
    return await services.pickVfinderSaveAsTarget(suggestedName);
  }
  // fallback inline — services 미설정 자리.
  return await new Promise<SaveAsTarget | null>((resolve) => {
    let settled = false;
    const handle = mountVfinderModal({
      mode: 'save-as',
      suggestedName,
      vfinderBase: services.vfinderBase,
      userId: services.vfinderUserId,
      onSaveAs: (r) => { if (settled) return; settled = true; resolve(r); },
      onCancel: () => { if (settled) return; settled = true; resolve(null); },
    });
    window.setTimeout(() => {
      if (settled) return;
      settled = true;
      handle.close();
      resolve(null);
    }, 5 * 60 * 1000);
  });
}

/**
 * 현재 문서 blob 을 vfinder `/api/upload` 자리 직호출. *target 이 있으면 path 모드*
 * (신규/덮어쓰기), *없으면 file_id 모드* (재저장).
 *
 * 본 helper 가 *picker 를 안 띄움* 이 핵심 — caller (file.ts) 가 picker 결과를
 * 미리 받아 본 함수에 *그대로 전달*. 그러면 *picker 가 두 번 뜨는* 사고가 원천 차단.
 *
 * 성공 자리 result.fileId 를 `WasmBridge.vfinderFileId` 자리 박음 — 다음 Ctrl+S 자리
 * 곧장 file_id 모드 덮어쓰기.
 */
async function uploadCurrentDocumentToVfinder(
  services: CommandServices,
  target: SaveAsTarget | null,
): Promise<SaveCurrentDocumentResult> {
  const sourceFormat = services.wasm.getSourceFormat();
  if (sourceFormat === 'hwpx') {
    alert(t('file.save.hwpx_disabled'));
    return 'unsupported';
  }

  const bytes = services.wasm.exportHwp();
  const blob = new Blob([bytes as unknown as BlobPart], { type: 'application/x-hwp' });
  const fileName = hwpSaveFileName(services.wasm.fileName);

  try {
    if (target) {
      // path 모드 (신규/덮어쓰기) — picker 가 반환한 이름에 *확장자 자동 부여* 후
      // multipart 전송. vfinder picker 자리 사용자가 확장자 없이 입력해도 `.hwp` 가
      // 강제로 붙어 저장 (요청 정합 — 다른 이름으로 저장 자리 항상 .hwp 으로 끝).
      const correctedName = hwpSaveFileName(target.name);
      const result = await uploadToVfinder({
        blob,
        fileName: correctedName,
        vfinderBase: services.vfinderBase,
        userId: services.vfinderUserId,
        path: target.path,
        overwrite: target.overwrite,
      });
      if (result.fileId) services.wasm.vfinderFileId = result.fileId;
      services.wasm.fileName = result.name;
      services.documentState.markClean('save');
      console.log(`[file:save] vfinder 저장 완료: ${result.path} (${(bytes.length / 1024).toFixed(1)}KB)`);
      return 'saved';
    }
    // file_id 모드 (재저장) — vfinderFileId 보유 가정.
    const fileId = services.wasm.vfinderFileId;
    if (!fileId) {
      console.error('[file:save] file_id 모드 호출 — vfinderFileId 부재');
      return 'failed';
    }
    const result = await uploadToVfinder({
      blob,
      fileName,
      vfinderBase: services.vfinderBase,
      userId: services.vfinderUserId,
      fileId,
    });
    services.documentState.markClean('save');
    console.log(`[file:save] vfinder 덮어쓰기 완료: ${result.path} (${(bytes.length / 1024).toFixed(1)}KB)`);
    return 'saved';
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.error('[file:save] vfinder 저장 실패:', msg);
    alert(t('error.client.save_failed_generic', { message: msg }));
    return 'failed';
  }
}

/**
 * agent VM iframe 자리 `Ctrl+S` 흐름 — *picker 한 번 또는 없음*.
 *
 * - vfinderFileId 보유 자리: picker 없이 곧장 file_id 모드 덮어쓰기 (재저장).
 * - 미보유 자리: picker 한 번 → path 모드 신규/덮어쓰기.
 *
 * `Ctrl+Shift+S` (다른 이름으로 저장) 흐름은 본 함수를 *안 부르고* file:save-as
 * execute 자리 직접 `pickSaveAsTarget` + `uploadCurrentDocumentToVfinder(target)` 자리
 * 조합 — server forward 시도 자리 *같은 target 재활용* 자리 위함.
 */
async function saveCurrentDocumentViaVfinder(
  services: CommandServices,
): Promise<SaveCurrentDocumentResult> {
  if (services.wasm.vfinderFileId) {
    // 재저장 — picker 없음.
    return await uploadCurrentDocumentToVfinder(services, null);
  }
  // 첫 저장 — picker 한 번.
  const target = await pickSaveAsTarget(services, hwpSaveFileName(services.wasm.fileName));
  if (!target) return 'cancelled';
  return await uploadCurrentDocumentToVfinder(services, target);
}

export async function saveCurrentDocument(services: CommandServices): Promise<SaveCurrentDocumentResult> {
  // SSR 세션 모드: 로컬 저장 대신 서버(minio 덮어쓰기)에 저장한다.
  if (services.saveToServer) {
    try {
      if (await services.saveToServer()) {
        services.documentState.markClean('save');
        console.log('[file:save] SSR 서버 저장 완료');
        return 'saved';
      }
      // saveToServer 가 false 반환 — 세션 식별자 누락 등. *cross-origin 체크 없이*
      // vfinder 직호출 흐름으로 자동 fallback. top window 자리 (VM URL 직접 진입)
      // 에서도 vfinder 가 같은 host 라 자연 작동 — 사용자 의도가 *저장이 실제로 이뤄짐*
      // 인데 cross-origin 만 체크하면 top window 진입 자리에서 로컬 폴백으로 흐른다.
      console.warn('[file:save] SSR 서버 저장 거부 — vfinder 직호출로 fallback');
      const r1 = await saveCurrentDocumentViaVfinder(services);
      if (r1 !== 'failed') return r1;
      console.warn('[file:save] vfinder 직호출도 실패 — 로컬 폴백');
    } catch (e) {
      // SSR 활성 상태의 *진짜 서버 실패* (502·timeout 등). cross-origin 체크 없이
      // vfinder 직호출 시도. vfinder 자체도 실패하면 로컬 폴백.
      const msg = e instanceof Error ? e.message : String(e);
      console.warn(`[file:save] SSR 서버 저장 실패 (${msg}) — vfinder 직호출로 fallback`);
      try {
        const r1 = await saveCurrentDocumentViaVfinder(services);
        if (r1 !== 'failed') return r1;
      } catch (e2) {
        console.warn('[file:save] vfinder 직호출도 실패 — 로컬 폴백', e2);
      }
    }
  } else if (isCrossOriginEmbedded(window as FileSystemWindowLike)) {
    // SSR 비활성 + cross-origin iframe (agent VM iframe). agent host 의 SSR 파라미터
    // 부착에 의존하지 않고 *VM 내부 vfinder /api/upload 직호출* 흐름으로 저장.
    try {
      const r1 = await saveCurrentDocumentViaVfinder(services);
      if (r1 !== 'failed') return r1;
    } catch (e) {
      console.warn('[file:save] vfinder 직호출 실패 — 로컬 폴백', e);
    }
  }
  // SSR 비활성 + top window / same-origin 자리 — 기존 file picker / download 흐름.
  try {
    const saveName = services.wasm.fileName;
    const sourceFormat = services.wasm.getSourceFormat();
    const isHwpx = sourceFormat === 'hwpx';
    if (isHwpx) {
      alert(t('file.save.hwpx_disabled'));
      return 'unsupported';
    }

    const bytes = services.wasm.exportHwp();
    const blob = new Blob([bytes as unknown as BlobPart], { type: 'application/x-hwp' });
    console.log(`[file:save] format=${sourceFormat}, isHwpx=${isHwpx}, ${bytes.length} bytes`);

    try {
      const saveResult = await saveDocumentToFileSystem({
        blob,
        suggestedName: saveName,
        currentHandle: services.wasm.currentFileHandle,
        windowLike: window as FileSystemWindowLike,
      });

      if (saveResult.method !== 'fallback') {
        services.wasm.currentFileHandle = saveResult.handle;
        services.wasm.fileName = saveResult.fileName;
        services.documentState.markClean('save');
        console.log(`[file:save] ${saveResult.fileName} (${(bytes.length / 1024).toFixed(1)}KB)`);
        return 'saved';
      }
    } catch (e) {
      if (isUserCancelError(e)) return 'cancelled';
      console.warn('[file:save] File System Access API 실패, 폴백:', e);
    }

    let downloadName = saveName;
    if (services.wasm.isNewDocument) {
      const baseName = saveName.replace(/\.hwp$/i, '');
      const result = await showSaveAs(baseName);
      if (!result) return 'cancelled';
      downloadName = result;
      services.wasm.fileName = downloadName;
    }

    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = downloadName;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);

    services.documentState.markClean('save');
    console.log(`[file:save] ${downloadName} (${(bytes.length / 1024).toFixed(1)}KB)`);
    return 'saved';
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error('[file:save] 저장 실패:', msg);
    alert(t('error.client.save_file_failed', { message: msg }));
    return 'failed';
  }
}

export async function confirmSaveBeforeReplacingDocument(
  services: CommandServices,
): Promise<boolean> {
  const ctx = services.getContext();
  if (!ctx.hasDocument || !ctx.isDirty) return true;

  const choice = await showUnsavedChangesDialog({
    fileName: services.wasm.fileName,
    canSave: ctx.sourceFormat !== 'hwpx',
  });

  if (choice === 'cancel') return false;
  if (choice === 'discard') return true;

  const result = await saveCurrentDocument(services);
  return result === 'saved';
}

function appendPrintStyle(doc: Document, widthMm: number, heightMm: number): void {
  const style = doc.createElement('style');
  style.textContent = `
@page { size: ${widthMm}mm ${heightMm}mm; margin: 0; }
* { margin: 0; padding: 0; }
body { background: #fff; }
.page { page-break-after: always; width: ${widthMm}mm; height: ${heightMm}mm; overflow: hidden; }
.page:last-child { page-break-after: auto; }
.page svg { width: 100%; height: 100%; }
@media screen {
  body { background: #e5e7eb; display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 16px; }
  .page { background: #fff; box-shadow: 0 2px 8px rgba(0,0,0,0.15); }
  .print-bar { position: fixed; top: 0; left: 0; right: 0; background: #1e293b; color: #fff; padding: 8px 16px; display: flex; align-items: center; gap: 12px; font: 14px sans-serif; z-index: 100; }
  .print-bar button { padding: 6px 16px; background: #2563eb; color: #fff; border: none; border-radius: 4px; cursor: pointer; font-size: 14px; }
  .print-bar button:hover { background: #1d4ed8; }
  body { padding-top: 56px; }
}
@media print { .print-bar { display: none; } }
`;
  doc.head.appendChild(style);
}

function createPrintButton(doc: Document, id: string, label: string, background?: string): HTMLButtonElement {
  const button = doc.createElement('button');
  button.id = id;
  button.type = 'button';
  button.textContent = label;
  if (background) button.style.background = background;
  return button;
}

function appendSvgPage(doc: Document, container: HTMLElement, svg: string): void {
  const page = doc.createElement('div');
  page.className = 'page';

  const parsed = new DOMParser().parseFromString(svg, 'image/svg+xml');
  const parseError = parsed.querySelector('parsererror');
  if (parseError) {
    throw new Error(t('file.print.svg_parse_failed', { detail: parseError.textContent || 'parsererror' }));
  }

  page.appendChild(doc.importNode(parsed.documentElement, true));
  container.appendChild(page);
}

function setupPrintDocument(
  printWin: Window,
  fileName: string,
  pageCount: number,
  widthMm: number,
  heightMm: number,
  svgPages: string[],
): void {
  const doc = printWin.document;
  doc.documentElement.lang = 'ko';
  doc.title = t('file.print.title', { fileName });

  doc.head.replaceChildren();
  const meta = doc.createElement('meta');
  meta.setAttribute('charset', 'UTF-8');
  doc.head.appendChild(meta);
  appendPrintStyle(doc, widthMm, heightMm);

  const printBar = doc.createElement('div');
  printBar.className = 'print-bar';
  const printButton = createPrintButton(doc, 'print-btn', t('file.print.btn_print'));
  const closeButton = createPrintButton(doc, 'close-btn', t('file.print.btn_close'), '#475569');
  const title = doc.createElement('span');
  title.textContent = t('file.print.page_summary', { fileName, pageCount });
  printBar.append(printButton, closeButton, title);

  doc.body.replaceChildren(printBar);
  for (const svg of svgPages) {
    appendSvgPage(doc, doc.body, svg);
  }

  printButton.addEventListener('click', () => {
    printWin.print();
  });
  closeButton.addEventListener('click', () => {
    printWin.close();
  });
}

export const fileCommands: CommandDef[] = [
  {
    id: 'file:new-doc',
    label: t('cmd.file.new_doc'),
    icon: 'icon-new-doc',
    shortcutLabel: 'Alt+N',
    canExecute: () => true,
    execute(services) {
      services.eventBus.emit('create-new-document');
    },
  },
  {
    id: 'file:open',
    label: t('cmd.file.open'),
    async execute(services) {
      // SSR + iframe 환경: vfinder picker 흐름이 우선. cross-origin sub frame 자리에서
      // 브라우저가 showOpenFilePicker 자체를 차단하므로 로컬 폴백이 무의미.
      if (services.openViaVfinder) {
        try {
          const canReplace = await confirmSaveBeforeReplacingDocument(services);
          if (!canReplace) return;
          if (await services.openViaVfinder()) {
            console.log('[file:open] vfinder 진입 트리거 발사');
            return;
          }
        } catch (e) {
          console.warn('[file:open] vfinder 진입 실패 — 로컬 흐름으로 폴백', e);
        }
      }
      try {
        const canReplace = await confirmSaveBeforeReplacingDocument(services);
        if (!canReplace) return;

        const handle = await pickOpenFileHandle(window as FileSystemWindowLike);
        if (!handle) {
          const fileInput = document.getElementById('file-input') as HTMLInputElement | null;
          if (fileInput) {
            fileInput.dataset.skipUnsavedGuard = 'true';
            fileInput.click();
          }
          return;
        }

        const { bytes, name } = await readFileFromHandle(handle);
        services.eventBus.emit('open-document-bytes', {
          bytes,
          fileName: name,
          fileHandle: handle,
          skipUnsavedGuard: true,
        });
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:open] 열기 실패:', msg);
        alert(t('error.client.open_file_failed', { message: msg }));
      }
    },
  },
  {
    id: 'file:save',
    label: t('cmd.file.save'),
    icon: 'icon-save',
    shortcutLabel: 'Ctrl+S',
    canExecute: (ctx) => ctx.hasDocument,
    async execute(services) {
      await saveCurrentDocument(services);
    },
  },
  {
    // [Task #833] 다른 이름으로 저장 — currentFileHandle 무시 + 항상 picker.
    id: 'file:save-as',
    label: t('cmd.file.save_as'),
    shortcutLabel: 'Ctrl+Shift+S',
    canExecute: (ctx) => ctx.hasDocument,
    async execute(services) {
      // *vfinder 흐름 우선* — SSR 활성 자리 (forwardSaveAsToServer 정의) 또는
      // cross-origin iframe (agent VM iframe) 자리. *cross-origin 만 체크하면* top
      // window 로 VM URL 진입한 자리 (vfinder 도 같은 host) 에서도 로컬 폴백으로
      // 흘러 사용자 의도 위반. SSR 활성 도 진입 조건 에 추가.
      const useVfinderFlow =
        !!services.forwardSaveAsToServer ||
        isCrossOriginEmbedded(window as FileSystemWindowLike);
      if (useVfinderFlow) {
        // picker 가 *정확히 한 번* 만 뜨도록 picker 호출 과 저장 위임 을 명시적 분리:
        //   1) pickSaveAsTarget → SaveAsTarget 받음 (한 번)
        //   2) SSR 활성 자리 server-side `/save-as` forward 시도
        //   3) server-side 실패 (false·throw) 자리 *같은 target* 으로 vfinder direct upload
        //
        // SSR 비활성 자리 (2) 건너뛰고 곧장 (3). 어느 경로든 picker 두 번 안 뜸.
        const suggested = hwpSaveFileName(services.wasm.fileName);
        const target = await pickSaveAsTarget(services, suggested);
        if (!target) return; // 사용자 취소

        // (2) SSR 활성 자리 server-side forward
        if (services.forwardSaveAsToServer) {
          try {
            if (await services.forwardSaveAsToServer(target)) {
              services.documentState.markClean('save');
              console.log('[file:save-as] server-side 저장 완료');
              return;
            }
            console.warn('[file:save-as] server-side 거부 — vfinder 직호출로 fallback');
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            console.warn(`[file:save-as] server-side 실패 (${msg}) — vfinder 직호출로 fallback`);
          }
        }

        // (3) client direct vfinder upload — picker 결과 그대로 재활용. 확장자 자동 부여는
        // uploadCurrentDocumentToVfinder 내부 자리 hwpSaveFileName(target.name) 자리.
        const r = await uploadCurrentDocumentToVfinder(services, target);
        if (r !== 'failed') return;
        console.warn('[file:save-as] vfinder 직호출도 실패 — 로컬 폴백');
      }
      try {
        const sourceFormat = services.wasm.getSourceFormat();
        const isHwpx = sourceFormat === 'hwpx';
        const saveName = hwpSaveFileName(services.wasm.fileName);
        const bytes = services.wasm.exportHwp();
        const blob = new Blob([bytes as unknown as BlobPart], { type: 'application/x-hwp' });
        console.log(`[file:save-as] format=${sourceFormat}, hwpExport=${isHwpx}, ${bytes.length} bytes`);

        try {
          const saveResult = await saveDocumentToFileSystem({
            blob,
            suggestedName: saveName,
            currentHandle: null,
            windowLike: window as FileSystemWindowLike,
            forceSaveAs: true,
          });
          if (saveResult.method !== 'fallback') {
            services.wasm.currentFileHandle = saveResult.handle;
            services.wasm.fileName = saveResult.fileName;
            console.log(`[file:save-as] ${saveResult.fileName} (${(bytes.length / 1024).toFixed(1)}KB)`);
            return;
          }
        } catch (e) {
          if (isUserCancelError(e)) return;
          console.warn('[file:save-as] File System Access API 실패, 폴백:', e);
        }

        // 폴백: 파일명 입력 → blob download
        const baseName = hwpSaveBaseName(saveName);
        const result = await showSaveAs(baseName);
        if (!result) return;
        const downloadName = result;
        services.wasm.fileName = downloadName;

        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = downloadName;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);

        console.log(`[file:save-as] ${downloadName} (${(bytes.length / 1024).toFixed(1)}KB)`);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:save-as] 저장 실패:', msg);
        alert(t('error.client.save_file_failed', { message: msg }));
      }
    },
  },
  {
    id: 'file:page-setup',
    label: t('cmd.file.page_setup'),
    icon: 'icon-page-setup',
    shortcutLabel: 'F7',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      const dialog = new PageSetupDialog(services.wasm, services.eventBus, 0);
      dialog.show();
    },
  },
  {
    id: 'file:print',
    label: t('cmd.file.print'),
    icon: 'icon-print',
    shortcutLabel: 'Ctrl+P',
    canExecute: (ctx) => ctx.hasDocument,
    async execute(services) {
      const wasm = services.wasm;
      const pageCount = wasm.pageCount;
      if (pageCount === 0) return;

      // 진행률 표시
      const statusEl = document.getElementById('sb-message');
      const origStatus = statusEl?.textContent || '';

      try {
        // SVG 페이지 생성
        const svgPages: string[] = [];
        for (let i = 0; i < pageCount; i++) {
          if (statusEl) statusEl.textContent = t('file.print.preparing', { current: i + 1, total: pageCount });
          const svg = wasm.renderPageSvg(i);
          svgPages.push(svg);
          // UI 갱신을 위한 양보
          if (i % 5 === 0) await new Promise(r => setTimeout(r, 0));
        }

        // 첫 페이지 정보로 용지 크기 결정
        const pageInfo = wasm.getPageInfo(0);
        const widthMm = Math.round(pageInfo.width * 25.4 / 96);
        const heightMm = Math.round(pageInfo.height * 25.4 / 96);

        // 인쇄 전용 창 생성
        const printWin = window.open('', '_blank');
        if (!printWin) {
          alert(t('error.client.popup_blocked'));
          return;
        }

        setupPrintDocument(printWin, wasm.fileName, pageCount, widthMm, heightMm, svgPages);

        if (statusEl) statusEl.textContent = origStatus;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:print]', msg);
        if (statusEl) statusEl.textContent = t('file.print.failed', { message: msg });
      }
    },
  },
  {
    id: 'file:about',
    label: t('cmd.file.about'),
    icon: 'icon-help',
    execute() {
      new AboutDialog().show();
    },
  },
];
