/**
 * Sub-2 e2e: replace_runs.
 *
 * 시나리오:
 *   1. insert_text 로 문단 0 에 '원본' 삽입
 *   2. replace_runs 로 같은 문단 runs 를 'E2E-RUN' (bold) 으로 교체
 *   3. WS broadcast 채널에 ServerEvent::Ops 가 replace_runs 포함하여 수신되는지 확인
 *   4. GET /ir 로 문단 0 텍스트가 'E2E-RUN' 인지 검증
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIr,
  findEvent,
  wait,
} from './sub2-helpers.mjs';

async function main() {
  const fileId = newFileId('sub2-replace-runs');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  // 1. 사전 텍스트 삽입
  const insertResp = await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: '원본',
  });
  if (insertResp.status !== 200) {
    throw new Error(`사전 insert_text 실패: ${JSON.stringify(insertResp)}`);
  }

  // 2. replace_runs
  const resp = await postWorkbench(fileId, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'E2E-RUN', style: { bold: true } }],
  });
  if (resp.status !== 200) {
    throw new Error(`replace_runs 실패: ${JSON.stringify(resp)}`);
  }
  if (resp.body.applied !== 'ops') {
    throw new Error(`applied !== 'ops': ${JSON.stringify(resp.body)}`);
  }

  // 3. broadcast 수신 확인
  await wait(500);
  const opsEv = findEvent(
    received,
    'ops',
    (ev) => Array.isArray(ev.ops) && ev.ops.some((o) => o.op === 'replace_runs'),
  );
  if (!opsEv) {
    throw new Error(
      `ServerEvent::Ops replace_runs 미수신: received=${JSON.stringify(received)}`,
    );
  }

  // 4. IR 검증 — sections[0].paragraphs[0].text 가 'E2E-RUN'
  const ir = await getIr(fileId);
  const para0Text =
    ir.paragraphs?.[0]?.text ?? ir.sections?.[0]?.paragraphs?.[0]?.text;
  if (para0Text !== 'E2E-RUN') {
    throw new Error(`IR text mismatch: got '${para0Text}', expected 'E2E-RUN'`);
  }

  console.log('=== Sub-2 replace_runs e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
