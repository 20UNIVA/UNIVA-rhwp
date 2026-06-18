/**
 * Sub-2 e2e: delete_element.
 *
 * 시나리오:
 *   1. paragraph 분기 — 두 문단 만든 뒤 첫 문단 삭제 → 두 번째 문단의 텍스트가 [0] 으로 이동
 *   2. table 분기 — 표 삽입 후 table_para 의 element_type='table' 삭제
 *      삭제 후 해당 문단의 controls 에 table 이 사라졌는지 확인
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
  // === 1. paragraph 분기 ===
  const fileId1 = newFileId('sub2-delete-element-para');
  await createSession(fileId1);
  const s1 = subscribeWs(fileId1);
  await s1.opened;
  console.log(`WS 연결 OK — ${fileId1}`);

  // insert_paragraph 의 after_para 는 *삽입 위치 인덱스* 로 동작한다.
  // [first, second] 배치를 만들려면 paragraph_count(=1) 를 지정해 끝에 붙인다.
  await postWorkbench(fileId1, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'first',
  });
  await postWorkbench(fileId1, 'insert_paragraph', {
    section: 0,
    after_para: 1,
    count: 1,
  });
  await postWorkbench(fileId1, 'insert_text', {
    section: 0,
    para: 1,
    offset: 0,
    text: 'second',
  });

  const r1 = await postWorkbench(fileId1, 'delete_element', {
    section: 0,
    para: 0,
    element_type: 'paragraph',
  });
  if (r1.status !== 200) throw new Error(`delete_element paragraph 실패: ${JSON.stringify(r1)}`);

  const ir1 = await getIr(fileId1);
  const ps1 = paragraphs(ir1);
  if (ps1[0]?.text !== 'second') {
    throw new Error(`para 삭제 후 [0].text mismatch: '${ps1[0]?.text}' (expected 'second')`);
  }
  s1.ws.close();

  // === 2. table 분기 ===
  const fileId2 = newFileId('sub2-delete-element-table');
  await createSession(fileId2);
  const s2 = subscribeWs(fileId2);
  await s2.opened;
  console.log(`WS 연결 OK — ${fileId2}`);

  // 빈 문서에 표 삽입 — table_para = 1 (Phase 2a.3 발견)
  await postWorkbench(fileId2, 'insert_table', {
    section: 0,
    insert_after_para: 0,
    rows: 2,
    cols: 2,
  });
  const irBeforeDel = await getIr(fileId2);
  const psBefore = paragraphs(irBeforeDel);
  // 표가 있는 문단 인덱스 식별
  let tablePara = -1;
  for (let i = 0; i < psBefore.length; i++) {
    const ctrls = psBefore[i].controls ?? [];
    if (ctrls.some((c) => c.kind === 'table')) {
      tablePara = i;
      break;
    }
  }
  if (tablePara < 0) {
    throw new Error(`표가 있는 문단을 찾지 못함: ${JSON.stringify(psBefore)}`);
  }

  const r2 = await postWorkbench(fileId2, 'delete_element', {
    section: 0,
    para: tablePara,
    element_type: 'table',
  });
  if (r2.status !== 200) throw new Error(`delete_element table 실패: ${JSON.stringify(r2)}`);

  const irAfter = await getIr(fileId2);
  const psAfter = paragraphs(irAfter);
  const stillTable = psAfter.some((p) =>
    (p.controls ?? []).some((c) => c.kind === 'table'),
  );
  if (stillTable) {
    throw new Error(`table 삭제 후에도 table control 잔존: ${JSON.stringify(psAfter)}`);
  }
  s2.ws.close();

  console.log('=== Sub-2 delete_element e2e 통과 ===');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
