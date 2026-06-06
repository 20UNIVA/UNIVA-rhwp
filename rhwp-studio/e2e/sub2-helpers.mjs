/**
 * Sub-2 e2e 공통 헬퍼.
 *
 * 서버는 외부에서 미리 가동 (예: `e2e/sub2-server.sh start` 또는
 * `cd server && cargo run`) 후 e2e 가 호출하는 가정.
 *
 * Node 22+ 표준 globalThis.WebSocket 사용 — 추가 패키지 의존성 없음.
 * 이는 Sub-1 의 ws-bridge.test.mjs 와 동일한 방식.
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { Buffer } from 'node:buffer';

export const BASE = 'http://127.0.0.1:7710';
export const WS_BASE = 'ws://127.0.0.1:7710';

/**
 * 빈 hwpx 샘플 경로.
 * Sub-1 ws-bridge.test.mjs 와 동일한 파일을 가리킨다.
 */
export const BLANK_HWPX_PATH = resolve(
  import.meta.dirname ?? '.',
  '..',
  '..',
  'samples',
  'hwpx',
  'blank_hwpx.hwpx',
);

/**
 * 빈 hwpx base64. 첫 호출 시 파일을 읽어 캐시 — e2e 가 createSession 을
 * 여러 번 부르더라도 디스크는 한 번만 읽는다.
 */
let _blankHwpxBase64Cache = null;
export function blankHwpxBase64() {
  if (_blankHwpxBase64Cache === null) {
    const bytes = readFileSync(BLANK_HWPX_PATH);
    _blankHwpxBase64Cache = Buffer.from(bytes).toString('base64');
  }
  return _blankHwpxBase64Cache;
}

/** UUID 스러운 fileId 생성. */
export function newFileId(prefix = 'sub2') {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
}

/**
 * 세션 생성 — POST /sessions.
 * base64 미지정 시 blank_hwpx.hwpx 를 자동으로 읽어 사용.
 */
export async function createSession(fileId, base64 = null) {
  const fileBase64 = base64 ?? blankHwpxBase64();
  const resp = await fetch(`${BASE}/sessions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ fileId, format: 'hwpx', fileBase64 }),
  });
  if (!resp.ok) {
    const t = await resp.text();
    throw new Error(`createSession 실패 ${resp.status}: ${t}`);
  }
  return resp.json();
}

/**
 * WebSocket 구독 시작. 받은 메시지를 메모리 array 에 누적.
 *
 * 반환:
 *   - ws         : WebSocket 인스턴스
 *   - received   : 수신 메시지 누적 (JSON 파싱된 객체)
 *   - opened     : 'open' 까지 기다리는 Promise
 *   - close()    : 명시적 종료 함수
 */
export function subscribeWs(fileId) {
  const ws = new WebSocket(`${WS_BASE}/sessions/${encodeURIComponent(fileId)}/ws`);
  const received = [];
  ws.addEventListener('message', (event) => {
    try {
      received.push(JSON.parse(event.data));
    } catch (e) {
      received.push({ raw: String(event.data), parseError: e.message });
    }
  });
  const opened = new Promise((resolveOpen, rejectOpen) => {
    const onOpen = () => {
      ws.removeEventListener('error', onError);
      resolveOpen();
    };
    const onError = () => {
      ws.removeEventListener('open', onOpen);
      rejectOpen(new Error('WS error during open'));
    };
    ws.addEventListener('open', onOpen, { once: true });
    ws.addEventListener('error', onError, { once: true });
    setTimeout(() => rejectOpen(new Error('WS open timeout (5s)')), 5000);
  });
  return {
    ws,
    received,
    opened,
    close: () => {
      try { ws.close(); } catch (_) { /* ignore */ }
    },
  };
}

/**
 * POST /sessions/{fid}/workbench.
 * 반환: { status, body } — 4xx 도 throw 하지 않고 그대로 노출 (negative test 지원).
 */
export async function postWorkbench(fileId, action, payload) {
  const resp = await fetch(`${BASE}/sessions/${encodeURIComponent(fileId)}/workbench`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ action, payload }),
  });
  let body;
  const text = await resp.text();
  try { body = text ? JSON.parse(text) : null; }
  catch (_) { body = { raw: text }; }
  return { status: resp.status, body };
}

/** GET /sessions/{fid}/ir?page=N. */
export async function getIr(fileId, page = 0) {
  const resp = await fetch(`${BASE}/sessions/${encodeURIComponent(fileId)}/ir?page=${page}`);
  if (!resp.ok) throw new Error(`getIr 실패 ${resp.status}: ${await resp.text()}`);
  return resp.json();
}

/**
 * GET /sessions/{fid}/ir-slice?sec=&para_start=&para_end=&mode=.
 * para_end null 이면 단일 문단 요청.
 */
export async function getIrSlice(fileId, sec = 0, paraStart = 0, paraEnd = null, mode = 'auto') {
  const params = new URLSearchParams({
    sec: String(sec),
    para_start: String(paraStart),
    mode,
  });
  if (paraEnd !== null) params.set('para_end', String(paraEnd));
  const resp = await fetch(
    `${BASE}/sessions/${encodeURIComponent(fileId)}/ir-slice?${params}`,
  );
  if (!resp.ok) throw new Error(`getIrSlice 실패 ${resp.status}: ${await resp.text()}`);
  return resp.json();
}

/** GET /sessions/{fid}/audit?seq_from=&seq_to=. */
export async function getAudit(fileId, seqFrom, seqTo) {
  const resp = await fetch(
    `${BASE}/sessions/${encodeURIComponent(fileId)}/audit?seq_from=${seqFrom}&seq_to=${seqTo}`,
  );
  if (!resp.ok) throw new Error(`getAudit 실패 ${resp.status}: ${await resp.text()}`);
  return resp.json();
}

/** GET /sessions/{fid}/diff?seq=N. */
export async function getDiff(fileId, seq) {
  const resp = await fetch(`${BASE}/sessions/${encodeURIComponent(fileId)}/diff?seq=${seq}`);
  if (!resp.ok) throw new Error(`getDiff 실패 ${resp.status}: ${await resp.text()}`);
  return resp.json();
}

/**
 * POST /sessions/{fid}/undo.
 * 반환: { status, body } — postWorkbench 와 동일 형식.
 */
export async function postUndo(fileId) {
  const resp = await fetch(
    `${BASE}/sessions/${encodeURIComponent(fileId)}/undo`,
    { method: 'POST' },
  );
  let body;
  const text = await resp.text();
  try { body = text ? JSON.parse(text) : null; }
  catch (_) { body = { raw: text }; }
  return { status: resp.status, body };
}

/** 짧은 sleep. */
export const wait = (ms) => new Promise((r) => setTimeout(r, ms));

/**
 * 수신 메시지 array 에서 특정 kind 첫 매치를 찾는다.
 * predicate 추가 시 둘 다 만족하는 첫 항목 반환.
 */
export function findEvent(received, kind, predicate = null) {
  return received.find(
    (ev) => ev && ev.kind === kind && (!predicate || predicate(ev)),
  );
}

/**
 * 수신 메시지 array 에서 특정 kind 가 등장할 때까지 polling.
 * timeoutMs 안에 못 보면 Error.
 */
export async function waitForEvent(received, kind, predicate = null, timeoutMs = 5000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const hit = findEvent(received, kind, predicate);
    if (hit) return hit;
    await wait(50);
  }
  throw new Error(`waitForEvent: kind="${kind}" timeout (${timeoutMs}ms)`);
}
