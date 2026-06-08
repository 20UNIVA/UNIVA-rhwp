/**
 * E2E (Sub-6): WS broadcast 의 self-echo 라벨링 — origin_client_id 전파 검증.
 *
 * 배경: 브라우저가 빠르게 타이핑하면 600ms 디바운스 단위마다 같은 입력이 *복제* 되는
 * 사고가 있었다. 원인은 서버 broadcast 가 발신자 자신에게도 echo 되는 구조.
 * 본 e2e 는 *프로토콜 단* 에서 다음을 검증한다.
 *
 *   1. `{kind:'ops', client_id:'A', ops:[...]}` 으로 보낸 메시지는
 *      *모든 구독자* 에게 `origin_client_id:'A'` 가 박힌 broadcast 로 echo 된다.
 *      → 발신자 A 도 받지만, A 가 비교해 *자기 것* 임을 알고 skip 할 수 있다.
 *      → 다른 구독자 B 는 *남이 보낸 것* 으로 보고 적용 대상.
 *
 *   2. `client_id` 누락 메시지 (구 클라 / HTTP 경로 시뮬레이션) 는 broadcast 의
 *      `origin_client_id` 가 *직렬화에서 누락* — 즉 키 자체가 없음
 *      (서버 `skip_serializing_if = "Option::is_none"` 동작 회귀 방지).
 *
 * 사전 조건: 서버가 `127.0.0.1:7710` 에서 가동 중. 다른 sub2-* e2e 와 동일.
 * Node 22+ 의 빌트인 `globalThis.WebSocket` 사용 — 추가 패키지 없음.
 */

import {
  BASE,
  WS_BASE,
  createSession,
  newFileId,
  postWorkbench,
  wait,
  waitForEvent,
} from './sub2-helpers.mjs';

/**
 * client_id 를 명시한 WebSocket 구독자.
 * 받은 메시지를 `received` 에 누적. `opened` 로 open 까지 대기.
 */
function openLabeledWs(fileId, label) {
  const ws = new WebSocket(`${WS_BASE}/sessions/${encodeURIComponent(fileId)}/ws`);
  const received = [];
  ws.addEventListener('message', (ev) => {
    try {
      received.push(JSON.parse(ev.data));
    } catch (e) {
      received.push({ raw: String(ev.data), parseError: e.message });
    }
  });
  const opened = new Promise((res, rej) => {
    const onOpen = () => {
      ws.removeEventListener('error', onError);
      res();
    };
    const onError = () => {
      ws.removeEventListener('open', onOpen);
      rej(new Error(`WS(${label}) error during open`));
    };
    ws.addEventListener('open', onOpen, { once: true });
    ws.addEventListener('error', onError, { once: true });
    setTimeout(() => rej(new Error(`WS(${label}) open timeout (5s)`)), 5000);
  });
  return {
    ws,
    received,
    opened,
    sendOps(clientId, ops) {
      const payload = clientId == null
        ? { kind: 'ops', ops }
        : { kind: 'ops', client_id: clientId, ops };
      ws.send(JSON.stringify(payload));
    },
    close: () => { try { ws.close(); } catch (_) { /* ignore */ } },
  };
}

function assert(cond, msg) {
  if (!cond) {
    throw new Error(`ASSERT 실패: ${msg}`);
  }
}

async function checkServerAlive() {
  try {
    const r = await fetch(`${BASE}/healthz`).catch(() => null);
    if (r && r.ok) return true;
    // healthz 가 없을 수도 있으므로 sessions 엔드포인트라도 살아 있는지 확인.
    const r2 = await fetch(`${BASE}/sessions/nonexistent/info`).catch(() => null);
    return r2 != null;
  } catch {
    return false;
  }
}

async function main() {
  if (!(await checkServerAlive())) {
    throw new Error(
      `서버가 ${BASE} 에서 응답 없음. 먼저 'cd server && cargo run' 또는 ./dev_run.sh 로 띄울 것.`,
    );
  }

  const fileId = newFileId('sub6');
  await createSession(fileId);

  // ── 시나리오 1: A 가 client_id 동봉 → A·B 모두 origin_client_id='A' 를 받음
  const A = openLabeledWs(fileId, 'A');
  const B = openLabeledWs(fileId, 'B');
  await Promise.all([A.opened, B.opened]);

  const CLIENT_A = 'cli-A-test';
  const insertOp = {
    op: 'insert_text',
    section: 0,
    para: 0,
    offset: 0,
    text: '가',
  };
  A.sendOps(CLIENT_A, [insertOp]);

  // A 도 자기 broadcast 를 받는다(self-echo). origin_client_id 가 자기 것임을 보고
  // 브라우저는 skip 하지만, *프로토콜 단* 에서는 이 메시지가 도달함을 검증.
  const onA = await waitForEvent(A.received, 'ops', (ev) => ev.origin_client_id === CLIENT_A);
  assert(onA.origin_client_id === CLIENT_A,
    `A 수신 origin_client_id 가 '${CLIENT_A}' 여야 함, 실제=${onA.origin_client_id}`);
  assert(Array.isArray(onA.ops) && onA.ops.length === 1,
    `A 수신 ops 길이 1 이어야 함, 실제=${onA.ops?.length}`);

  const onB = await waitForEvent(B.received, 'ops', (ev) => ev.origin_client_id === CLIENT_A);
  assert(onB.origin_client_id === CLIENT_A,
    `B 수신 origin_client_id 가 '${CLIENT_A}' 여야 함, 실제=${onB.origin_client_id}`);
  assert(Array.isArray(onB.ops) && onB.ops.length === 1,
    `B 수신 ops 길이 1 이어야 함, 실제=${onB.ops?.length}`);

  // ── 시나리오 2: client_id 누락 메시지 → broadcast 의 origin_client_id 가 키 자체 없음
  // (서버 skip_serializing_if 동작 회귀 방지)
  const beforeCount = A.received.length;
  A.sendOps(null, [{ ...insertOp, text: '나', offset: 1 }]);

  // 새로 들어온 ops 이벤트 중 *seq* 가 첫 이벤트보다 큰 것을 잡는다.
  const firstSeq = onA.seq;
  const onA2 = await waitForEvent(
    A.received.slice(beforeCount),
    'ops',
    (ev) => typeof ev.seq === 'number' && ev.seq > firstSeq,
  ).catch(async () => {
    // slice 한 사본이라 polling 효과 미흡 → 새로 polling.
    return await waitForEvent(
      A.received,
      'ops',
      (ev) => typeof ev.seq === 'number' && ev.seq > firstSeq,
    );
  });
  assert(!('origin_client_id' in onA2),
    `client_id 누락 시 broadcast 에 origin_client_id 키가 없어야 함. 실제 keys=${Object.keys(onA2).join(',')}`);

  // ── 시나리오 3: HTTP /workbench 경로 — broadcast 의 origin_client_id 부재 확인
  const beforeCount2 = A.received.length;
  const wb = await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: '다',
  });
  assert(wb.status === 200, `workbench 200 이어야 함, 실제=${wb.status}`);

  // 새 ops 이벤트(앞 두 개보다 seq 큰 것) 도착 대기.
  const lastSeq = onA2.seq;
  const onA3 = await waitForEvent(
    A.received,
    'ops',
    (ev) => typeof ev.seq === 'number' && ev.seq > lastSeq,
  );
  assert(!('origin_client_id' in onA3),
    `HTTP /workbench broadcast 에 origin_client_id 키가 없어야 함. 실제 keys=${Object.keys(onA3).join(',')}`);

  // 정리
  A.close(); B.close();
  await wait(50);
  console.log('[sub6-ws-echo-skip] PASS — 3 시나리오 통과');
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error('[sub6-ws-echo-skip] FAIL:', err);
    process.exit(1);
  });
