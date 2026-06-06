/**
 * Sub-2 e2e: insert_paragraph.
 *
 * 시나리오:
 *   1. count=1 — after_para=0 → paragraphs.length === 2
 *   2. count=3 — after_para=1 → paragraphs.length === 5
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIr,
} from './sub2-helpers.mjs';

function paraCount(ir) {
  // getIr(page=0) 는 *현재 페이지만* paragraphs 에 담는다. 전체 개수는
  // sections[0].paragraph_count 로 봐야 한다. fallback 으로 paragraphs.length.
  return (
    ir.sections?.[0]?.paragraph_count
      ?? ir.paragraphs?.length
      ?? ir.sections?.[0]?.paragraphs?.length
  );
}

async function main() {
  const fileId = newFileId('sub2-insert-paragraph');
  await createSession(fileId);
  const { ws, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  const ir0 = await getIr(fileId);
  const before = paraCount(ir0);

  // 1. count=1
  const r1 = await postWorkbench(fileId, 'insert_paragraph', {
    section: 0,
    after_para: 0,
    count: 1,
  });
  if (r1.status !== 200) throw new Error(`insert_paragraph count=1 실패: ${JSON.stringify(r1)}`);

  const ir1 = await getIr(fileId);
  const after1 = paraCount(ir1);
  if (after1 !== before + 1) {
    throw new Error(`count=1 후 paragraphs 수 mismatch: before=${before} after=${after1}`);
  }

  // 2. count=3
  const r2 = await postWorkbench(fileId, 'insert_paragraph', {
    section: 0,
    after_para: 1,
    count: 3,
  });
  if (r2.status !== 200) throw new Error(`insert_paragraph count=3 실패: ${JSON.stringify(r2)}`);

  const ir2 = await getIr(fileId);
  const after2 = paraCount(ir2);
  if (after2 !== before + 1 + 3) {
    throw new Error(`count=3 후 paragraphs 수 mismatch: 기대=${before + 4} got=${after2}`);
  }

  console.log('=== Sub-2 insert_paragraph e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
