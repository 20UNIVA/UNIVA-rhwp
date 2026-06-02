/**
 * SSR 세션 클라이언트.
 *
 * iframe(rhwp-studio)이 VM의 rhwp-server에 직접 HTTP로 연결하여,
 * 편집을 서버 세션에 미러링한다(디바운스 배치). 프론트가 닫혀도 서버단에
 * 문서·patch가 유지되고 모델이 조회할 수 있게 하는 클라이언트 측 절반이다.
 *
 * - 연산형 편집: `queueOp(op)` → 디바운스 후 `POST /sessions/{id}/ops`
 * - 스냅샷형 편집(붙여넣기/객체/표 등): `requestSnapshot()` → 전체 export `PUT /snapshot`
 * - 종료 시 잔여 큐 flush(`beforeunload`).
 */
import type { EditOperation } from '@/engine/edit-op';

/** 편집 미러링 싱크 — InputHandler가 편집 후 호출한다. */
export interface MirrorSink {
  /** 연산형 편집을 큐에 넣는다(디바운스 배치 전송). */
  queueOp(op: EditOperation): void;
  /** 스냅샷형 편집 — 전체 문서 동기화를 요청한다. */
  requestSnapshot(): void;
}

/** Uint8Array → base64 (청크 처리로 대용량 안전). */
function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export interface SessionClientOptions {
  /** 서버 base URL. 빈 문자열이면 same-origin 상대경로. */
  baseUrl: string;
  fileId: string;
  /** 스냅샷 동기화 시 현재 문서를 내보내는 함수(예: () => wasm.exportHwpx()). */
  getSnapshotBytes: () => Uint8Array | null;
  /** 원본 포맷("hwp" | "hwpx"). 세션 생성/스냅샷 메타. */
  format?: string;
  /** 디바운스 간격(ms). 기본 600. */
  debounceMs?: number;
}

export class SessionClient implements MirrorSink {
  private readonly baseUrl: string;
  private readonly fileId: string;
  private readonly getSnapshotBytes: () => Uint8Array | null;
  private readonly format: string;
  private readonly debounceMs: number;

  private queue: EditOperation[] = [];
  private opTimer: ReturnType<typeof setTimeout> | null = null;
  private snapTimer: ReturnType<typeof setTimeout> | null = null;
  private flushing = false;
  private unloadHandler: (() => void) | null = null;

  constructor(opts: SessionClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, '');
    this.fileId = opts.fileId;
    this.getSnapshotBytes = opts.getSnapshotBytes;
    this.format = opts.format ?? 'hwpx';
    this.debounceMs = opts.debounceMs ?? 600;
  }

  private url(path: string): string {
    return `${this.baseUrl}${path}`;
  }

  /** fileId + 원본 바이트로 서버 세션을 생성/재생성한다. */
  async createSession(bytes: Uint8Array): Promise<void> {
    const body = JSON.stringify({
      fileId: this.fileId,
      format: this.format,
      fileBase64: bytesToBase64(bytes),
    });
    const res = await fetch(this.url('/sessions'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    if (!res.ok) {
      throw new Error(`세션 생성 실패: HTTP ${res.status}`);
    }
    this.installUnloadFlush();
  }

  queueOp(op: EditOperation): void {
    this.queue.push(op);
    this.scheduleOpFlush();
  }

  requestSnapshot(): void {
    // 스냅샷은 전체 문서 상태를 포함하므로 보류 중인 연산 큐는 폐기한다.
    this.queue.length = 0;
    if (this.opTimer) {
      clearTimeout(this.opTimer);
      this.opTimer = null;
    }
    if (this.snapTimer) return;
    this.snapTimer = setTimeout(() => {
      this.snapTimer = null;
      void this.flushSnapshot();
    }, this.debounceMs);
  }

  private scheduleOpFlush(): void {
    if (this.opTimer) clearTimeout(this.opTimer);
    this.opTimer = setTimeout(() => {
      this.opTimer = null;
      void this.flushOps();
    }, this.debounceMs);
  }

  /** 큐에 쌓인 연산을 서버에 배치 전송한다. */
  async flushOps(): Promise<void> {
    if (this.flushing || this.queue.length === 0) return;
    this.flushing = true;
    const batch = this.queue.splice(0);
    try {
      const res = await fetch(this.url(`/sessions/${encodeURIComponent(this.fileId)}/ops`), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(batch),
      });
      if (!res.ok) throw new Error(`ops 전송 실패: HTTP ${res.status}`);
    } catch (e) {
      // 실패 시 큐 복원(다음 flush에서 재시도).
      this.queue.unshift(...batch);
      console.warn('[SessionClient] ops flush 실패, 재시도 예약', e);
      this.scheduleOpFlush();
    } finally {
      this.flushing = false;
    }
  }

  private async flushSnapshot(): Promise<void> {
    const bytes = this.getSnapshotBytes();
    if (!bytes) return;
    try {
      const res = await fetch(this.url(`/sessions/${encodeURIComponent(this.fileId)}/snapshot`), {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ fileBase64: bytesToBase64(bytes) }),
      });
      if (!res.ok) throw new Error(`snapshot 전송 실패: HTTP ${res.status}`);
    } catch (e) {
      console.warn('[SessionClient] snapshot flush 실패', e);
    }
  }

  /** 종료 직전 잔여 연산을 sendBeacon으로 best-effort 전송한다. */
  private installUnloadFlush(): void {
    if (this.unloadHandler) return;
    this.unloadHandler = () => {
      if (this.queue.length === 0) return;
      const batch = this.queue.splice(0);
      const url = this.url(`/sessions/${encodeURIComponent(this.fileId)}/ops`);
      const blob = new Blob([JSON.stringify(batch)], { type: 'application/json' });
      navigator.sendBeacon?.(url, blob);
    };
    window.addEventListener('beforeunload', this.unloadHandler);
  }

  /** 리스너 해제 + 잔여 flush. */
  dispose(): void {
    if (this.unloadHandler) {
      window.removeEventListener('beforeunload', this.unloadHandler);
      this.unloadHandler = null;
    }
    void this.flushOps();
  }
}
