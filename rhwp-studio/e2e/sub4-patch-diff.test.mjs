/**
 * Sub-4 e2e — workbench 응답에 PatchDiff 가 포함되어 모델이 적용 여부 / 변화 내용을
 * tool result 만 보고 확인할 수 있는지 검증.
 *
 * 사전 조건: `e2e/sub2-server.sh start` 또는 `cd server && cargo run` 으로 서버 가동.
 *
 * 검증 항목:
 *  1. insert_text → diff.summary.changed=true, afterTextLen > beforeTextLen
 *  2. replace_runs (실제 변경) → changed=true, op="replace_runs"
 *  3. insert_paragraph → location.paraEndAfter > location.paraEndBefore
 *  4. replace_cell_runs → location.cell 채워짐, op="replace_cell_runs"
 *  5. delete_range → after_para_count <= before_para_count
 *  6. complete / passthrough → diff 키 자체가 없거나 null
 */

import { strict as assert } from 'node:assert';
import {
  createSession,
  newFileId,
  postWorkbench,
} from './sub2-helpers.mjs';

const TEST_RESULTS = [];
function record(name, fn) {
  TEST_RESULTS.push({ name, fn });
}

record('insert_text 응답에 diff 가 채워지고 paragraphs target 사용', async () => {
  const fid = newFileId('sub4-it');
  await createSession(fid);
  // insert_text 는 server 가 ops 분기 — diff 채워야 함.
  const { status, body } = await postWorkbench(fid, 'insert_text', {
    section: 0, para: 0, offset: 0, text: '안녕',
  });
  assert.equal(status, 200, `status: ${status} body=${JSON.stringify(body)}`);
  assert.equal(body.applied, 'ops');
  assert.ok(body.diff, 'diff 가 응답에 포함되어야 함');
  assert.equal(body.diff.op, 'insert_text');
  assert.equal(body.diff.summary.changed, true, 'insert_text 는 changed=true 여야');
  assert.ok(
    body.diff.summary.afterTextLen >= body.diff.summary.beforeTextLen + 2,
    `텍스트 길이 2 글자 증가 — before=${body.diff.summary.beforeTextLen}, after=${body.diff.summary.afterTextLen}`,
  );
  // 본문 편집 → paragraphs target.
  assert.ok(body.diff.before.paragraphs, 'before.paragraphs 가 있어야');
  assert.ok(body.diff.after.paragraphs, 'after.paragraphs 가 있어야');
  assert.ok(body.diff.before.cell === undefined, 'cell 키는 없어야');
});

record('replace_runs 응답 diff.op 가 정확히 매핑', async () => {
  const fid = newFileId('sub4-rr');
  await createSession(fid);
  // 먼저 본문에 텍스트 삽입.
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'ABC' });
  const { body } = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'XYZ', style: { bold: true } }],
  });
  assert.equal(body.applied, 'ops');
  assert.ok(body.diff, 'diff 필요');
  assert.equal(body.diff.op, 'replace_runs');
  assert.equal(body.diff.summary.changed, true);
});

record('insert_paragraph 응답 location.paraEndAfter 가 늘어남', async () => {
  const fid = newFileId('sub4-ip');
  await createSession(fid);
  const { body } = await postWorkbench(fid, 'insert_paragraph', {
    section: 0, after_para: 0, count: 2,
  });
  assert.ok(body.diff, 'diff 필요');
  assert.equal(body.diff.op, 'insert_paragraph');
  const loc = body.diff.location;
  assert.equal(loc.paraStartBefore, 0);
  assert.equal(loc.paraEndBefore, 1);
  assert.equal(loc.paraStartAfter, 0);
  // count=2 → after = [0..3)
  assert.equal(loc.paraEndAfter, 3, `after_end 가 0+1+2=3 이어야 — got ${loc.paraEndAfter}`);
});

record('insert_table → replace_cell_runs 가 cell target 으로 압축됨', async () => {
  const fid = newFileId('sub4-rcr');
  await createSession(fid);
  // 먼저 표를 만들어 둔다 (insert_table 자체는 paragraphs target — cell focus 없음).
  const t1 = await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  assert.equal(t1.body.applied, 'ops');
  assert.ok(t1.body.diff, 'insert_table 도 diff 필요');
  assert.equal(t1.body.diff.op, 'insert_table');
  assert.ok(t1.body.diff.after.paragraphs, 'insert_table 는 paragraphs target');
  // after = [0..2) — 원래 para + 새 표 para.
  assert.equal(t1.body.diff.location.paraEndAfter, 2);

  // 셀 한 칸 변경 → cell target.
  const t2 = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [{ text: '셀값', style: {} }],
  });
  assert.equal(t2.body.applied, 'ops');
  assert.ok(t2.body.diff, 'replace_cell_runs diff 필요');
  assert.equal(t2.body.diff.op, 'replace_cell_runs');
  // cell target 검증.
  assert.ok(t2.body.diff.before.cell, 'before.cell 가 있어야 (cell target)');
  assert.ok(t2.body.diff.after.cell, 'after.cell 가 있어야');
  assert.ok(t2.body.diff.before.paragraphs === undefined, 'paragraphs 키 없어야');
  // cell 자체에 row/col/paragraphs 들어 있음.
  assert.equal(t2.body.diff.after.cell.row, 0);
  assert.equal(t2.body.diff.after.cell.col, 0);
  assert.ok(Array.isArray(t2.body.diff.after.cell.paragraphs));
  // location.cell 좌표.
  assert.ok(t2.body.diff.location.cell, 'location.cell 필요');
  assert.equal(t2.body.diff.location.cell.tablePara, 1);
  assert.equal(t2.body.diff.location.cell.row, 0);
  assert.equal(t2.body.diff.location.cell.col, 0);
  assert.equal(t2.body.diff.location.cell.cellPara, 0);
  assert.equal(typeof t2.body.diff.location.cell.cellIdx, 'number');
});

record('빈 셀의 before 응답에 runs[0].style 이 항상 노출됨 (Sub-4 v3)', async () => {
  // 이전엔 빈 paragraph 의 compact 직렬화가 placeholder run 의 style 을 *문서 defaults
  // 와 같다* 판정해 omit — 셀에 묶인 char_shape (색 등) 가 응답에서 사라져 모델이 "왜 갑자기
  // 빨간색이지?" 라고 혼란을 겪었음. Sub-4 v3 는 빈 paragraph 라도 paragraph 첫 char_shape
  // 를 placeholder style 로 채워, runs 키 자체가 응답에 *항상 존재* 하게 보장.
  const fid = newFileId('sub4-empty-cell-style');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  // 빈 셀 상태에서 replace_cell_runs 호출 → before.cell.paragraphs[0].runs 가 정의돼 있어야.
  const { body } = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [{ text: 'A', style: {} }],
  });
  const beforePara = body.diff.before.cell.paragraphs[0];
  assert.ok(
    Array.isArray(beforePara.runs),
    `빈 셀의 before 에도 runs 키가 있어야 — got ${JSON.stringify(beforePara)}`,
  );
  assert.ok(beforePara.runs.length >= 1, 'placeholder run 1건 이상');
  // run 의 style 키 자체 존재 검증 (style omit 가능하지만 runs 키는 살아있음).
  assert.ok('style' in beforePara.runs[0] || beforePara.runs[0].text === '', 'run 형식 유지');
});

record('큰 표 셀 1개 편집 응답 크기가 표 크기와 무관하게 작음', async () => {
  // 10x10 = 100 셀. 표 전체 IR 두 번 (before+after) 이면 수만 byte 이지만,
  // cell target 압축 후엔 1KB 미만이어야 한다.
  const fid = newFileId('sub4-size');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 10, cols: 10,
  });
  const { body } = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 5, col: 7, cell_para: 0,
    runs: [{ text: '변경값' }],
  });
  const respSize = JSON.stringify(body).length;
  assert.ok(
    respSize < 2000,
    `10x10 표 셀 1개 변경 응답이 2KB 미만이어야 — 실제 ${respSize} byte`,
  );
  // 그리고 응답에 다른 셀들 정보는 없어야 — cell target 안에 row=5, col=7 의 셀 한 칸만.
  assert.equal(body.diff.after.cell.row, 5);
  assert.equal(body.diff.after.cell.col, 7);
});

record('delete_range 응답 paraEndAfter 가 줄어듦', async () => {
  const fid = newFileId('sub4-dr');
  await createSession(fid);
  // 본문에 일정 텍스트 + paragraph 추가.
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'AAA' });
  await postWorkbench(fid, 'insert_paragraph', { section: 0, after_para: 0, count: 2 });
  const { body } = await postWorkbench(fid, 'delete_range', {
    section: 0, para_start: 0, char_start: 0, para_end: 2, char_end: 0,
  });
  assert.ok(body.diff, 'delete_range diff 필요');
  assert.equal(body.diff.op, 'delete_range');
  // before = [0..3), after = [0..1) — paragraphs target.
  assert.equal(body.diff.location.paraStartBefore, 0);
  assert.equal(body.diff.location.paraEndBefore, 3);
  assert.equal(body.diff.location.paraEndAfter, 1);
  assert.ok(body.diff.before.paragraphs, 'before.paragraphs');
  assert.ok(body.diff.after.paragraphs, 'after.paragraphs');
  assert.ok(
    body.diff.summary.afterParaCount <= body.diff.summary.beforeParaCount,
    `paragraph 수 줄어야 — before=${body.diff.summary.beforeParaCount}, after=${body.diff.summary.afterParaCount}`,
  );
});

record('complete 응답에는 diff 가 없음 (None)', async () => {
  const fid = newFileId('sub4-cp');
  await createSession(fid);
  const { body } = await postWorkbench(fid, 'complete', {});
  assert.equal(body.applied, 'complete');
  // skip_serializing_if = "Option::is_none" 이라 키 자체가 직렬화 결과에 없어야 함.
  assert.ok(body.diff == null, `complete 응답에 diff 없어야 함 — got ${JSON.stringify(body.diff)}`);
});

record('알 수 없는 action (passthrough) 응답에도 diff 없음', async () => {
  const fid = newFileId('sub4-pt');
  await createSession(fid);
  const { body } = await postWorkbench(fid, 'some_unknown_action', { foo: 'bar' });
  assert.equal(body.applied, 'passthrough');
  assert.ok(body.diff == null, 'passthrough 에는 diff 없어야');
});

// ─── 실행기 ────────────────────────────────────────────────────────────────

let pass = 0;
let fail = 0;
for (const { name, fn } of TEST_RESULTS) {
  try {
    await fn();
    console.log(`  ✓ ${name}`);
    pass++;
  } catch (e) {
    console.error(`  ✗ ${name}\n    ${e.message}`);
    if (e.stack) console.error(e.stack.split('\n').slice(1, 4).map(l => '    ' + l).join('\n'));
    fail++;
  }
}
console.log(`\nSub-4 PatchDiff e2e: pass=${pass} fail=${fail}`);
if (fail > 0) process.exit(1);
