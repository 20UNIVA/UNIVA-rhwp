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

record('insert_text 응답에 diff 가 채워지고 changed=true', async () => {
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
    `텍스트 길이 2 글자 증가해야 — before=${body.diff.summary.beforeTextLen}, after=${body.diff.summary.afterTextLen}`,
  );
  assert.equal(body.diff.location.section, 0);
  assert.equal(body.diff.location.paraStartBefore, 0);
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

record('insert_table → replace_cell_runs 로 cell focus 동작', async () => {
  const fid = newFileId('sub4-rcr');
  await createSession(fid);
  // 먼저 표를 만들어 둔다.
  const t1 = await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  assert.equal(t1.body.applied, 'ops');
  assert.ok(t1.body.diff, 'insert_table 도 diff 필요');
  assert.equal(t1.body.diff.op, 'insert_table');
  // after = [0..2) — 원래 para + 새 표 para.
  assert.equal(t1.body.diff.location.paraEndAfter, 2);

  // 표는 para index 1 에 위치 (insert_after_para=0 → para 1 추가).
  const t2 = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [{ text: '셀값', style: {} }],
  });
  assert.equal(t2.body.applied, 'ops');
  assert.ok(t2.body.diff, 'replace_cell_runs diff 필요');
  assert.equal(t2.body.diff.op, 'replace_cell_runs');
  assert.ok(t2.body.diff.location.cell, 'cell focus 필요');
  assert.equal(t2.body.diff.location.cell.tablePara, 1);
  assert.equal(t2.body.diff.location.cell.row, 0);
  assert.equal(t2.body.diff.location.cell.col, 0);
  assert.equal(t2.body.diff.location.cell.cellPara, 0);
  // cellIdx 는 서버가 미리 변환해 채워줌.
  assert.equal(typeof t2.body.diff.location.cell.cellIdx, 'number');
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
  // before = [0..3), after = [0..1) — paragraphCount 정확히 검증.
  assert.equal(body.diff.location.paraStartBefore, 0);
  assert.equal(body.diff.location.paraEndBefore, 3);
  assert.equal(body.diff.location.paraEndAfter, 1);
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
