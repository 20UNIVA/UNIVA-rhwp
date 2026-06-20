/**
 * SSR 세션 클라이언트 — *양방향 WebSocket*.
 *
 * 한 WS 채널로:
 *   - *클라 → 서버* 미러링: queueOp(디바운스), requestSnapshot, attach(beforeunload flush)
 *   - *서버 → 클라* 수신: onServerEvent(콜백)에 ServerEvent를 전달
 *
 * 기존 외부 API(MirrorSink, queueOp, requestSnapshot, attach, createSession)는 유지.
 * 호출자(InputHandler 등)는 변경 0.
 */
import type { EditOperation } from '@/engine/edit-op';

export interface MirrorSink {
  queueOp(op: EditOperation): void;
  requestSnapshot(): void;
}

/** WS 텍스트 프레임 본문 — 서버 → 클라 */
export type ServerEvent =
  // [Sub-6] `origin_client_id` — *원래 발신한 브라우저의 식별자*. WS broadcast 가
  // 발신자 자신에게도 echo 되는 구조에서, main.ts 가 자기 clientId 와 같으면 skip 한다.
  // HTTP `/workbench` 같은 외부 경로는 발신자 식별 없음 → 직렬화 시 키 자체가 누락
  // (서버 `skip_serializing_if`).
  | { kind: 'ops'; seq: number; ops: EditOpJson[]; origin_client_id?: string }
  | { kind: 'workbench'; seq: number; action: string; payload: unknown }
  | { kind: 'snapshot_restored'; seq: number; snapshot_base64: string }
  | { kind: 'complete'; seq: number };

interface EditOpJson {
  op: string;
  section?: number;
  para?: number;
  offset?: number;
  text?: string;
  count?: number;
  deleted_text?: string;
  prev_len?: number;
  // 2f.2 신규 ops payload — 정방향 EditOp 전체 broadcast.
  runs?: unknown[];
  style?: Record<string, unknown> | null;
  para_start?: number;
  char_start?: number;
  para_end?: number;
  char_end?: number;
  after_para?: number;
  element_type?: string;
  insert_after_para?: number;
  rows?: number;
  cols?: number;
  table_para?: number;
  row?: number;
  col?: number;
  row_start?: number;
  col_start?: number;
  row_end?: number;
  col_end?: number;
  cell_para?: number;
  cell_para_start?: number;
  cell_para_end?: number;
  // [4-4 fix] 서버가 broadcast 전에 (row, col) → cell_idx 변환 결과를 채워 보냄.
  // 클라는 우선 사용 (없으면 wasm.findCellIdx fallback). 다중 사용자 race 회피.
  cell_idx?: number;
  // paragraph 안 Table control 위치 (section_def + column_def 가 앞에 박힐 때 0 아님).
  // 서버 broadcast 자리 (row, col) → cell_idx 변환과 함께 ctrl_idx 도 함께 보냄.
  ctrl_idx?: number;
}

function bytesToBase64(bytes: Uint8Array): string {
  let bin = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(bin);
}

export interface SessionClientOptions {
  /** 서버 base URL. http(s)://host:port. WS URL은 ws(s)://...로 자동 변환. */
  baseUrl: string;
  fileId: string;
  getSnapshotBytes: () => Uint8Array | null;
  format?: string;
  debounceMs?: number;
  /** 서버가 발행한 이벤트를 받았을 때 콜백. main.ts에서 ops/workbench 분기 처리. */
  onServerEvent?: (ev: ServerEvent) => void;
  /** WS 재연결 백오프 — 기본 [500, 1000, 2000, 5000, 10000] ms */
  reconnectDelaysMs?: number[];
  /**
   * URL `?user=` 자리에서 추출된 사용자 식별자. 모든 HTTP 요청의 `X-Rhwp-User` 헤더에
   * 박힘. 미지정 시 서버 `RHWP_DEFAULT_USER` 환경변수 폴백.
   *
   * 브라우저 native WebSocket 은 커스텀 헤더를 못 박는다 — WS 연결 자리는 별도 사안.
   */
  userId?: string;
}

const DEFAULT_BACKOFF = [500, 1000, 2000, 5000, 10000];

export class SessionClient implements MirrorSink {
  private readonly baseUrlHttp: string;
  private readonly baseUrlWs: string;
  private readonly fileId: string;
  private readonly getSnapshotBytes: () => Uint8Array | null;
  private readonly format: string;
  private readonly debounceMs: number;
  private readonly onServerEvent?: (ev: ServerEvent) => void;
  private readonly reconnectDelaysMs: number[];
  private readonly userId?: string;

  // [Sub-6] *이 SessionClient 인스턴스의 고유 식별자*. 서버가 broadcast 페이로드의
  // `origin_client_id` 에 그대로 실어 — 자기 발신을 echo 로 받으면 main.ts 가 skip.
  private readonly clientId: string = crypto.randomUUID();

  private ws: WebSocket | null = null;
  private disposed = false;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectIdx = 0;
  private connected = false;
  private sendBuffer: string[] = []; // WS 닫혀 있을 때 큐
  private queue: EditOperation[] = [];
  private opTimer: ReturnType<typeof setTimeout> | null = null;
  private unloadHandler: (() => void) | null = null;

  constructor(opts: SessionClientOptions) {
    this.baseUrlHttp = opts.baseUrl.replace(/\/$/, '');
    this.baseUrlWs = this.baseUrlHttp.replace(/^http/, 'ws');
    this.fileId = opts.fileId;
    this.getSnapshotBytes = opts.getSnapshotBytes;
    this.format = opts.format ?? 'hwpx';
    this.debounceMs = opts.debounceMs ?? 600;
    this.onServerEvent = opts.onServerEvent;
    this.reconnectDelaysMs = opts.reconnectDelaysMs ?? DEFAULT_BACKOFF;
    this.userId = opts.userId;
  }

  /** fileId + 원본 바이트로 서버 세션을 생성/재생성. 이후 WS 연결. */
  async createSession(bytes: Uint8Array): Promise<void> {
    const body = JSON.stringify({
      fileId: this.fileId,
      format: this.format,
      fileBase64: bytesToBase64(bytes),
    });
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.userId) headers['X-Rhwp-User'] = this.userId;
    const res = await fetch(this.baseUrlHttp + '/sessions', {
      method: 'POST',
      headers,
      body,
    });
    if (!res.ok) throw new Error(`세션 생성 실패: HTTP ${res.status}`);
    this.openWs();
    this.installUnloadFlush();
  }

  /** 이미 서버에 존재하는 세션에 WS만 연결. createSession을 호출하면 ops 초기화. */
  attach(): void {
    this.openWs();
    this.installUnloadFlush();
  }

  /** [Sub-6] main.ts 의 onServerEvent self-echo skip 가드가 비교용으로 읽음. */
  getClientId(): string {
    return this.clientId;
  }

  queueOp(op: EditOperation): void {
    this.queue.push(op);
    this.scheduleOpFlush();
  }

  requestSnapshot(): void {
    const bytes = this.getSnapshotBytes();
    if (!bytes) return;
    const msg = JSON.stringify({
      kind: 'snapshot',
      file_base64: bytesToBase64(bytes),
    });
    this.sendOrBuffer(msg);
  }

  private scheduleOpFlush(): void {
    if (this.opTimer) clearTimeout(this.opTimer);
    this.opTimer = setTimeout(() => this.flushOps(), this.debounceMs);
  }

  /** 큐에 쌓인 연산을 즉시 WS로 전송한다. main.ts의 saveToServer 경로에서 await로 호출. */
  flushOps(): Promise<void> {
    if (this.queue.length === 0) return Promise.resolve();
    const ops = this.queue;
    this.queue = [];
    // [Sub-6] client_id 동봉 — 서버가 broadcast 의 origin_client_id 에 그대로 실음.
    const msg = JSON.stringify({ kind: 'ops', client_id: this.clientId, ops });
    this.sendOrBuffer(msg);
    return Promise.resolve();
  }

  private sendOrBuffer(msg: string): void {
    if (this.ws && this.connected) {
      try {
        this.ws.send(msg);
      } catch {
        this.sendBuffer.push(msg);
      }
    } else {
      this.sendBuffer.push(msg);
    }
  }

  private openWs(): void {
    if (this.disposed) return;
    if (this.ws) return;
    const url = `${this.baseUrlWs}/sessions/${encodeURIComponent(this.fileId)}/ws`;
    this.ws = new WebSocket(url);

    this.ws.addEventListener('open', () => {
      this.connected = true;
      this.reconnectIdx = 0;
      while (this.sendBuffer.length > 0) {
        const m = this.sendBuffer.shift()!;
        try {
          this.ws!.send(m);
        } catch {
          this.sendBuffer.unshift(m);
          break;
        }
      }
    });

    this.ws.addEventListener('message', (e) => {
      let parsed: ServerEvent;
      try {
        parsed = JSON.parse(e.data) as ServerEvent;
      } catch {
        console.warn('[session-client] WS 메시지 JSON 파싱 실패:', e.data);
        return;
      }
      if (this.onServerEvent) {
        try {
          this.onServerEvent(parsed);
        } catch (err) {
          console.error('[session-client] onServerEvent 예외:', err);
        }
      }
    });

    this.ws.addEventListener('close', () => {
      this.connected = false;
      this.ws = null;
      this.scheduleReconnect();
    });

    this.ws.addEventListener('error', (e) => {
      console.warn('[session-client] WS 에러:', e);
      // close 이벤트가 뒤따라 옴 → 거기서 재연결
    });
  }

  private scheduleReconnect(): void {
    if (this.disposed) return;
    const delay =
      this.reconnectDelaysMs[
        Math.min(this.reconnectIdx, this.reconnectDelaysMs.length - 1)
      ];
    this.reconnectIdx += 1;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.openWs();
    }, delay);
  }

  private installUnloadFlush(): void {
    if (this.unloadHandler) return;
    this.unloadHandler = () => {
      if (this.opTimer) clearTimeout(this.opTimer);
      void this.flushOps();
    };
    window.addEventListener('beforeunload', this.unloadHandler);
  }

  /** WS 연결 해제 + beforeunload 리스너 제거 + 잔여 큐 flush. */
  dispose(): void {
    this.disposed = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.unloadHandler) {
      window.removeEventListener('beforeunload', this.unloadHandler);
      this.unloadHandler = null;
    }
    if (this.opTimer) {
      clearTimeout(this.opTimer);
      this.opTimer = null;
    }
    void this.flushOps();
    if (this.ws) {
      // close 이벤트가 scheduleReconnect를 트리거하지 않도록 ws를 null로 먼저.
      const ws = this.ws;
      this.ws = null;
      this.connected = false;
      ws.close();
    }
  }
}
