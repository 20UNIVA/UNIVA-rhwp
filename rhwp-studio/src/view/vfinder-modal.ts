/**
 * vfinder iframe 을 rhwp studio 자체 modal 안에 띄우는 자리.
 *
 * 원래 설계는 *agent 부모창이 vfinder iframe 을 띄우고 rhwp 와 메시지 중계* 였으나,
 * agent frontend 가 별도 외부 레포라 직접 박지 못하는 자리. 그래서 *rhwp studio 가
 * 자체 modal 로 vfinder iframe 을 직접 띄우는* 흐름으로 정착한다.
 *
 * rhwp 와 vfinder 가 *같은 host* 인 자리에서는 iframe 안 iframe 도 같은 origin 룰로
 * 자연 통과 (브라우저 정책상 grand-iframe 자리 제약 없음).
 *
 * postMessage 약속은 vfinder 의 emitSaveAs / emitPick / emitCancel 그대로 받는다 —
 * parentOrigin 자리는 *rhwp studio 의 origin* 이 박힌다.
 */

export type VfinderModalMode = 'save-as' | 'picker';

export interface SaveAsResult {
  path: string;
  name: string;
  overwrite: boolean;
}

export interface PickItem {
  file_id: string | null;
  path: string;
  name: string;
  kind: 'file' | 'folder';
  size: number | null;
  mtime: string | null;
}

export interface VfinderModalOptions {
  /** modal 모드 — vfinder iframe 의 `?mode=` 자리. */
  mode: VfinderModalMode;
  /** save-as 모드 — 이름 입력 칸 초기값. */
  suggestedName?: string;
  /** picker 모드 — 선택 가능 종류. */
  kind?: 'file' | 'folder' | 'any';
  /** vfinder studio base URL. 기본 `/vfinder/` (같은 호스트). */
  vfinderBase?: string;
  /** vfinder 의 X-Vfinder-User 자리에 박을 사용자 식별자. URL `?user=` 로 전달. */
  userId?: string;
  /** save-as 모드 — 사용자가 *저장* 누르면 호출. */
  onSaveAs?: (result: SaveAsResult) => void;
  /** picker 모드 — 사용자가 *확인* 누르면 호출. */
  onPick?: (items: PickItem[]) => void;
  /** 사용자가 *취소* 또는 modal 닫기. */
  onCancel?: () => void;
}

export interface VfinderModalHandle {
  /** 외부에서 modal 강제 닫기. onCancel 은 호출되지 않는다. */
  close(): void;
}

const MODAL_STYLE_ID = 'vfinder-modal-styles';

function ensureStyles(): void {
  if (document.getElementById(MODAL_STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = MODAL_STYLE_ID;
  style.textContent = `
    .rhwp-vfinder-modal-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.45);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 9999;
    }
    .rhwp-vfinder-modal-box {
      width: min(880px, 92vw);
      height: min(640px, 86vh);
      background: #fff;
      border-radius: 10px;
      box-shadow: 0 12px 40px rgba(0, 0, 0, 0.22);
      display: grid;
      grid-template-rows: auto 1fr;
      overflow: hidden;
    }
    .rhwp-vfinder-modal-header {
      background: #fafafa;
      border-bottom: 1px solid #d8d8d8;
      padding: 10px 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .rhwp-vfinder-modal-title {
      margin: 0;
      font-size: 13px;
      font-weight: 600;
      color: #333;
    }
    .rhwp-vfinder-modal-close {
      width: 28px;
      height: 28px;
      border: 0;
      background: transparent;
      cursor: pointer;
      font-size: 18px;
      color: #666;
      border-radius: 4px;
    }
    .rhwp-vfinder-modal-close:hover { background: #ececec; }
    .rhwp-vfinder-modal-iframe {
      width: 100%;
      height: 100%;
      border: 0;
      display: block;
    }
  `;
  document.head.appendChild(style);
}

import { t, getLang, onLangChange } from '@/i18n/t';

function buildVfinderUrl(opts: VfinderModalOptions): string {
  const base = opts.vfinderBase ?? '/vfinder/';
  const params = new URLSearchParams({
    mode: opts.mode,
    parentOrigin: window.location.origin,
    sysLang: getLang(), // [Task #m700-11] 자식 iframe 도 같은 lang
  });
  if (opts.userId) params.set('user', opts.userId);
  if (opts.mode === 'save-as') {
    params.set('suggestedName', opts.suggestedName ?? 'untitled.hwp');
  } else if (opts.mode === 'picker') {
    params.set('kind', opts.kind ?? 'file');
    params.set('multi', 'false');
  }
  return `${base}?${params.toString()}`;
}

/**
 * vfinder iframe modal 을 띄운다. 사용자가 결과를 확정하거나 취소할 때까지 표시.
 * 반환 핸들의 close() 로 외부에서 닫을 수 있다.
 */
export function mountVfinderModal(opts: VfinderModalOptions): VfinderModalHandle {
  ensureStyles();

  const overlay = document.createElement('div');
  overlay.className = 'rhwp-vfinder-modal-overlay';

  const box = document.createElement('div');
  box.className = 'rhwp-vfinder-modal-box';

  const header = document.createElement('header');
  header.className = 'rhwp-vfinder-modal-header';

  const title = document.createElement('h2');
  title.className = 'rhwp-vfinder-modal-title';
  const titleKey = opts.mode === 'save-as' ? 'menu.file.save_as' : 'menu.file.open';
  title.textContent = t(titleKey);

  const closeBtn = document.createElement('button');
  closeBtn.className = 'rhwp-vfinder-modal-close';
  closeBtn.type = 'button';
  closeBtn.setAttribute('aria-label', t('button.close'));
  closeBtn.textContent = '×';

  header.appendChild(title);
  header.appendChild(closeBtn);

  const iframe = document.createElement('iframe');
  iframe.className = 'rhwp-vfinder-modal-iframe';
  iframe.src = buildVfinderUrl(opts);

  box.appendChild(header);
  box.appendChild(iframe);
  overlay.appendChild(box);
  document.body.appendChild(overlay);

  // vfinder iframe 의 origin — 같은 호스트면 window.location.origin 그대로.
  // 다른 host 자리면 vfinderBase URL 의 origin 으로 검증.
  let vfinderOrigin: string;
  try {
    vfinderOrigin = new URL(iframe.src, window.location.href).origin;
  } catch {
    vfinderOrigin = window.location.origin;
  }

  let cancelled = false;
  let closed = false;

  // [Task #m700-11] 부모 lang 갈리면 자식 iframe 에도 발사 + modal 자체
  // 라벨 갱신. modal 닫을 때 unsubscribe.
  const unsubscribeLang = onLangChange(() => {
    title.textContent = t(titleKey);
    closeBtn.setAttribute('aria-label', t('button.close'));
    try {
      iframe.contentWindow?.postMessage(
        { type: 'vfinder:set-locale', sysLang: getLang() },
        vfinderOrigin,
      );
    } catch {
      // 자식 iframe 자체 자체 아직 로드 안된 자체 자체 무시
    }
  });

  function teardown(): void {
    if (closed) return;
    closed = true;
    window.removeEventListener('message', onMessage);
    document.removeEventListener('keydown', onKeyDown);
    unsubscribeLang();
    overlay.remove();
  }

  function onMessage(ev: MessageEvent): void {
    // origin + source 검증 — vfinder iframe 에서 온 메시지만 받음.
    if (ev.origin !== vfinderOrigin) return;
    if (ev.source !== iframe.contentWindow) return;

    const data = ev.data as
      | { type?: string; path?: string; name?: string; overwrite?: boolean; items?: unknown }
      | null;
    if (!data || typeof data !== 'object') return;

    if (data.type === 'vfinder:save-as' &&
        typeof data.path === 'string' &&
        typeof data.name === 'string') {
      opts.onSaveAs?.({
        path: data.path,
        name: data.name,
        overwrite: data.overwrite === true,
      });
      teardown();
      return;
    }

    if (data.type === 'vfinder:pick' && Array.isArray(data.items)) {
      opts.onPick?.(data.items as PickItem[]);
      teardown();
      return;
    }

    if (data.type === 'vfinder:cancel') {
      cancelled = true;
      teardown();
      opts.onCancel?.();
      return;
    }
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && !closed) {
      cancelled = true;
      teardown();
      opts.onCancel?.();
    }
  }

  function onCloseClick(): void {
    cancelled = true;
    teardown();
    opts.onCancel?.();
  }

  // overlay 클릭 (modal 밖) — 닫기
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) {
      cancelled = true;
      teardown();
      opts.onCancel?.();
    }
  });

  closeBtn.addEventListener('click', onCloseClick);
  document.addEventListener('keydown', onKeyDown);
  window.addEventListener('message', onMessage);

  return {
    close(): void {
      teardown();
    },
  };
}
