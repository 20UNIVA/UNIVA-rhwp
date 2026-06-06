/**
 * Sub-2 e2e: replace_cell_runs.
 *
 * 시나리오:
 *   1. 1x2 표 삽입 (table_para = 1)
 *   2. (0,0) 셀 cell_para 0 에 '원본' 삽입
 *   3. replace_cell_runs 로 같은 셀 runs 를 '변경' 으로 교체
 *   4. 응답 applied === 'ops' + WS broadcast 수신
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
  const fileId = newFileId('sub2-replace-cell-runs');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  await postWorkbench(fileId, 'insert_table', {
    section: 0,
    insert_after_para: 0,
    rows: 1,
    cols: 2,
  });
  await postWorkbench(fileId, 'insert_text_in_cell', {
    section: 0,
    table_para: 1,
    row: 0,
    col: 0,
    cell_para: 0,
    offset: 0,
    text: '원본',
  });

  const resp = await postWorkbench(fileId, 'replace_cell_runs', {
    section: 0,
    table_para: 1,
    row: 0,
    col: 0,
    cell_para: 0,
    runs: [{ text: '변경' }],
  });
  if (resp.status !== 200) {
    throw new Error(`replace_cell_runs 실패: ${JSON.stringify(resp)}`);
  }
  if (resp.body.applied !== 'ops') {
    throw new Error(`applied !== 'ops': ${JSON.stringify(resp.body)}`);
  }

  await wait(500);
  const opsEv = findEvent(
    received,
    'ops',
    (ev) => Array.isArray(ev.ops) && ev.ops.some((o) => o.op === 'replace_cell_runs'),
  );
  if (!opsEv) {
    throw new Error(
      `replace_cell_runs broadcast 미수신: received=${JSON.stringify(received)}`,
    );
  }

  console.log('=== Sub-2 replace_cell_runs e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
