/**
 * E2E: WS 양방향 — 서버 측 broadcast + 클라 ops 미러링 검증.
 *
 * Sub-1 자동 검증 범위:
 *   - workbench POST 후 broadcast 채널에 ServerEvent::Ops 발행 (e2e WS 구독자 수신)
 *   - 클라 ClientMessage::Ops 송신 후 서버가 apply_edit_ops_json 호출 + sqlite 영속
 *
 * Puppeteer/브라우저 측 main.ts onServerEvent 적용·Canvas 렌더링은 *수동 시나리오*에서 검증
 * (Canvas 기반 렌더링이라 DOM innerText로 자동 검증이 부정확).
 * Node 22+ 표준 globalThis.WebSocket을 사용하므로 추가 패키지 의존성 없음.
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SERVER = 'http://127.0.0.1:7710/hwp';
const WS_BASE = 'ws://127.0.0.1:7710/hwp';
const BLANK_HWPX = resolve(import.meta.dirname ?? '.', '..', '..', 'samples', 'hwpx', 'blank_hwpx.hwpx');

async function http(method, path, body) {
  const r = await fetch(`${SERVER}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await r.text();
  if (!r.ok) throw new Error(`HTTP ${r.status} ${path}: ${text}`);
  return text ? JSON.parse(text) : {};
}

function awaitMessage(ws, predicate, timeoutMs = 5000) {
  return new Promise((resolveMatch, rejectTimeout) => {
    const timer = setTimeout(
      () => rejectTimeout(new Error(`WS 메시지 timeout (${timeoutMs}ms)`)),
      timeoutMs,
    );
    const onMsg = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (predicate(msg)) {
          clearTimeout(timer);
          ws.removeEventListener('message', onMsg);
          resolveMatch(msg);
        }
      } catch {
        // skip non-JSON frames
      }
    };
    ws.addEventListener('message', onMsg);
  });
}

async function main() {
  // 1) 세션 생성
  const fileId = `e2e-ws-${Date.now()}`;
  const bytes = readFileSync(BLANK_HWPX);
  await http('POST', '/sessions', {
    fileId,
    format: 'hwpx',
    fileBase64: Buffer.from(bytes).toString('base64'),
  });
  console.log('세션:', fileId);

  // 2) WS 구독 (e2e가 직접 클라이언트가 됨)
  const ws = new WebSocket(`${WS_BASE}/sessions/${fileId}/ws`);
  await new Promise((resolveOpen, rejectOpen) => {
    ws.addEventListener('open', () => resolveOpen(), { once: true });
    ws.addEventListener('error', () => rejectOpen(new Error('WS error')), { once: true });
    setTimeout(() => rejectOpen(new Error('WS open timeout')), 5000);
  });
  console.log('WS 연결 OK');

  // 3) 서버→클라 방향: workbench로 insert_text 발사 → e2e WS가 ServerEvent::Ops 받음
  const opsPromise = awaitMessage(ws, (m) => m.kind === 'ops');
  await http('POST', `/sessions/${fileId}/workbench`, {
    action: 'insert_text',
    payload: { section: 0, para: 0, offset: 0, text: 'FROM-LLM' },
  });
  const opsMsg = await opsPromise;
  if (
    !Array.isArray(opsMsg.ops) ||
    !opsMsg.ops.some((op) => op.text === 'FROM-LLM')
  ) {
    ws.close();
    throw new Error(
      `workbench broadcast 메시지에 'FROM-LLM' 없음. msg=${JSON.stringify(opsMsg).slice(0, 300)}`,
    );
  }
  console.log('OK 1: 서버→클라 broadcast로 ServerEvent::Ops 수신, ops에 "FROM-LLM" 포함');

  // 4) 클라→서버 방향: WS로 ClientMessage::Ops 송신
  ws.send(
    JSON.stringify({
      kind: 'ops',
      ops: [
        {
          op: 'insert_text',
          section: 0,
          para: 0,
          offset: 0,
          text: 'FROM-CLIENT',
        },
      ],
    }),
  );
  await new Promise((r) => setTimeout(r, 500));
  ws.close();

  // 5) 서버 IR 확인 — 두 텍스트 모두 영속
  const ir = await http('GET', `/sessions/${fileId}/ir?page=0`);
  const irText = JSON.stringify(ir);
  if (!irText.includes('FROM-LLM')) {
    throw new Error(`서버 IR에 'FROM-LLM' 없음. IR=${irText.slice(0, 500)}`);
  }
  if (!irText.includes('FROM-CLIENT')) {
    throw new Error(`서버 IR에 'FROM-CLIENT' 없음. IR=${irText.slice(0, 500)}`);
  }
  console.log('OK 2: 서버 IR에 "FROM-LLM"·"FROM-CLIENT" 둘 다 영속');

  console.log('\n=== 양방향 WS bridge 검증 통과 ===');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
