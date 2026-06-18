/**
 * Sub-2 e2e: delete_range.
 *
 * 시나리오:
 *   1. 동문단 — 'ABCDE' 삽입 → char_start=1, char_end=3 삭제 → 'ADE' 확인
 *   2. 다문단 — 두 문단 만든 뒤 para_start=0 char_start=2, para_end=1 char_end=2 삭제
 *      → 첫 문단 앞 2자 + 두 번째 문단 뒤 부분만 남는지 확인
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIr,
} from './sub2-helpers.mjs';

function paraText(ir, idx) {
  return ir.paragraphs?.[idx]?.text ?? ir.sections?.[0]?.paragraphs?.[idx]?.text;
}

async function main() {
  // === 시나리오 1: 동문단 ===
  const fileId1 = newFileId('sub2-delete-range-single');
  await createSession(fileId1);
  const sub1 = subscribeWs(fileId1);
  await sub1.opened;
  console.log(`WS 연결 OK — ${fileId1}`);

  await postWorkbench(fileId1, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'ABCDE',
  });
  const r1 = await postWorkbench(fileId1, 'delete_range', {
    section: 0,
    para_start: 0,
    char_start: 1,
    para_end: 0,
    char_end: 3,
  });
  if (r1.status !== 200) throw new Error(`동문단 delete_range 실패: ${JSON.stringify(r1)}`);
  const ir1 = await getIr(fileId1);
  const t1 = paraText(ir1, 0);
  if (t1 !== 'ADE') throw new Error(`동문단 결과 mismatch: '${t1}' (expected 'ADE')`);
  sub1.ws.close();

  // === 시나리오 2: 다문단 ===
  const fileId2 = newFileId('sub2-delete-range-multi');
  await createSession(fileId2);
  const sub2 = subscribeWs(fileId2);
  await sub2.opened;
  console.log(`WS 연결 OK — ${fileId2}`);

  // 문단 0 'ABCDE', 문단 1 'FGHIJ'
  // 주의: insert_paragraph 는 after_para 값을 *삽입 위치 인덱스* 로 사용한다
  // (insert_paragraph_native 의 para_idx 로 그대로 전달). 기존 문단 다음에
  // 새 문단을 두려면 *paragraph_count* 위치(= 1)를 지정해야 한다.
  await postWorkbench(fileId2, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'ABCDE',
  });
  await postWorkbench(fileId2, 'insert_paragraph', {
    section: 0,
    after_para: 1,
    count: 1,
  });
  await postWorkbench(fileId2, 'insert_text', {
    section: 0,
    para: 1,
    offset: 0,
    text: 'FGHIJ',
  });

  // 문단 0 의 char 2 부터 문단 1 의 char 2 까지 삭제 → 'AB' + 'HIJ' 가 같은 문단으로 합쳐짐
  const r2 = await postWorkbench(fileId2, 'delete_range', {
    section: 0,
    para_start: 0,
    char_start: 2,
    para_end: 1,
    char_end: 2,
  });
  if (r2.status !== 200) throw new Error(`다문단 delete_range 실패: ${JSON.stringify(r2)}`);
  const ir2 = await getIr(fileId2);
  const t2 = paraText(ir2, 0);
  if (t2 !== 'ABHIJ') {
    throw new Error(`다문단 결과 mismatch: '${t2}' (expected 'ABHIJ')`);
  }
  sub2.ws.close();

  console.log('=== Sub-2 delete_range e2e 통과 ===');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
