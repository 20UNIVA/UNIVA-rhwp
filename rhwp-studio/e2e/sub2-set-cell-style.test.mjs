/**
 * Sub-2 e2e: set_cell_style.
 *
 * 시나리오:
 *   1. 2x2 표 삽입 (빈 문서 → table_para = 1, Phase 2a.3 발견)
 *   2. (row=0, col=0) 셀에 vertical_align='middle' 적용
 *   3. 응답 applied === 'ops' + WS broadcast 수신
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  findEvent,
  wait,
} from './sub2-helpers.mjs';

async function main() {
  const fileId = newFileId('sub2-set-cell-style');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  await postWorkbench(fileId, 'insert_table', {
    section: 0,
    insert_after_para: 0,
    rows: 2,
    cols: 2,
  });

  const resp = await postWorkbench(fileId, 'set_cell_style', {
    section: 0,
    table_para: 1,
    row: 0,
    col: 0,
    style: { vertical_align: 'middle' },
  });
  if (resp.status !== 200) {
    throw new Error(`set_cell_style 실패: ${JSON.stringify(resp)}`);
  }
  if (resp.body.applied !== 'ops') {
    throw new Error(`applied !== 'ops': ${JSON.stringify(resp.body)}`);
  }

  await wait(500);
  const opsEv = findEvent(
    received,
    'ops',
    (ev) => Array.isArray(ev.ops) && ev.ops.some((o) => o.op === 'set_cell_style'),
  );
  if (!opsEv) {
    throw new Error(
      `set_cell_style broadcast 미수신: received=${JSON.stringify(received)}`,
    );
  }

  console.log('=== Sub-2 set_cell_style e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
