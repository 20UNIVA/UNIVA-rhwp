/**
 * Sub-2 e2e: merge_cells.
 *
 * 시나리오:
 *   1. 3x3 표 삽입 — 표 control 의 rows/cols 는 변하지 않지만 cells 수 감소
 *   2. (0,0)-(0,1) 두 셀 병합
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
  const fileId = newFileId('sub2-merge-cells');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  await postWorkbench(fileId, 'insert_table', {
    section: 0,
    insert_after_para: 0,
    rows: 3,
    cols: 3,
  });

  const resp = await postWorkbench(fileId, 'merge_cells', {
    section: 0,
    table_para: 1,
    row_start: 0,
    col_start: 0,
    row_end: 0,
    col_end: 1,
  });
  if (resp.status !== 200) {
    throw new Error(`merge_cells 실패: ${JSON.stringify(resp)}`);
  }
  if (resp.body.applied !== 'ops') {
    throw new Error(`applied !== 'ops': ${JSON.stringify(resp.body)}`);
  }

  await wait(500);
  const opsEv = findEvent(
    received,
    'ops',
    (ev) => Array.isArray(ev.ops) && ev.ops.some((o) => o.op === 'merge_cells'),
  );
  if (!opsEv) {
    throw new Error(
      `merge_cells broadcast 미수신: received=${JSON.stringify(received)}`,
    );
  }

  console.log('=== Sub-2 merge_cells e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
