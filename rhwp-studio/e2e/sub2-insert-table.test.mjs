/**
 * Sub-2 e2e: insert_table.
 *
 * 시나리오:
 *   1. rows=2 cols=3 표 삽입
 *   2. IR controls 에 table (rows=2 cols=3) 한 개 이상 등장
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIr,
} from './sub2-helpers.mjs';

function paragraphs(ir) {
  return ir.paragraphs ?? ir.sections?.[0]?.paragraphs ?? [];
}

async function main() {
  const fileId = newFileId('sub2-insert-table');
  await createSession(fileId);
  const { ws, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  const resp = await postWorkbench(fileId, 'insert_table', {
    section: 0,
    insert_after_para: 0,
    rows: 2,
    cols: 3,
  });
  if (resp.status !== 200) throw new Error(`insert_table 실패: ${JSON.stringify(resp)}`);

  const ir = await getIr(fileId);
  const ps = paragraphs(ir);
  const table = ps
    .flatMap((p) => p.controls ?? [])
    .find((c) => c.kind === 'table');
  if (!table) {
    throw new Error(`IR 에 table control 없음: ${JSON.stringify(ps)}`);
  }
  if (table.rows !== 2 || table.cols !== 3) {
    throw new Error(`table rows/cols mismatch: ${JSON.stringify(table)}`);
  }

  console.log('=== Sub-2 insert_table e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
