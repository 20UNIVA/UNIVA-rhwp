/**
 * Sub-7 v2 e2e — 셀 내 char_format 적용 + replace_runs cascade 회피 검증.
 *
 * 사전 조건: `e2e/sub2-server.sh start` 또는 `cd server && cargo run` 으로 서버 가동.
 *
 * Sub-7 v2 의 두 사고를 HTTP 수준에서 잠금:
 *   A. 셀 내 char_format — replace_cell_runs / insert_text_in_cell + style 이 *실제로* IR
 *      응답에 노출되는지. 옛 버그는 셀 paragraph 의 char_shapes 가 비어 있어 apply_char_shape_range
 *      가 no-op 으로 끝나면서 모든 run 이 default style 로만 보였다.
 *   B. 본문 replace_runs cascade 회피 — 각 run 의 미지정 style 키가 *직전 run 의 값을 상속*
 *      하지 않고 paragraph 의 원래 default 에서 출발하는지.
 *
 * 각 시나리오는 *독립 fileId*.
 */

import { strict as assert } from 'node:assert';
import {
  createSession,
  newFileId,
  postWorkbench,
  getIrSlice,
} from './sub2-helpers.mjs';

const TESTS = [];
function test(name, fn) {
  TESTS.push({ name, fn });
}

// ─── A. 셀 내 char_format ───────────────────────────────────────────────────

test('A1. replace_cell_runs + {bold:true} 1 run → diff.after.cell.runs[0].style.bold === true', async () => {
  const fid = newFileId('sub7v2-cell-bold');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 1, cols: 2,
  });
  const r = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [{ text: '굵게', style: { bold: true } }],
  });
  assert.equal(r.status, 200, `status ${r.status} body=${JSON.stringify(r.body).slice(0,400)}`);
  const after = r.body.diff?.after?.cell;
  assert.ok(after, `diff.after.cell 누락 — ${JSON.stringify(r.body).slice(0,400)}`);
  const runs = after.paragraphs[0].runs;
  assert.equal(runs.length, 1, `runs.length 1 기대 (실제 ${runs.length}) — ${JSON.stringify(runs)}`);
  assert.equal(runs[0].text, '굵게');
  assert.equal(runs[0].style.bold, true, `bold:true 누락 — style=${JSON.stringify(runs[0].style)}`);
});

test('A2. replace_cell_runs 3 run (color/bold/highlight) 분리 — diff.after.cell.runs 가 3개', async () => {
  const fid = newFileId('sub7v2-cell-3runs');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 1, cols: 2,
  });
  const r = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [
      { text: '빨강', style: { color: '#FF0000' } },
      { text: '굵게', style: { bold: true } },
      { text: '노랑', style: { highlight: '#FFFF00' } },
    ],
  });
  assert.equal(r.status, 200, `status ${r.status}`);
  const runs = r.body.diff?.after?.cell?.paragraphs?.[0]?.runs;
  assert.ok(runs, `diff.after.cell.paragraphs[0].runs 누락 — ${JSON.stringify(r.body).slice(0,400)}`);
  assert.equal(runs.length, 3, `runs 3개 기대 (실제 ${runs.length}) — ${JSON.stringify(runs)}`);
  assert.equal(runs[0].text, '빨강');
  assert.equal(runs[0].style.color, '#FF0000', `run[0] color #FF0000 누락 — ${JSON.stringify(runs[0])}`);
  assert.equal(runs[1].text, '굵게');
  assert.equal(runs[1].style.bold, true, `run[1] bold 누락 — ${JSON.stringify(runs[1])}`);
  assert.equal(runs[2].text, '노랑');
  assert.equal(runs[2].style.highlight, '#FFFF00', `run[2] highlight #FFFF00 누락 — ${JSON.stringify(runs[2])}`);
  // IR slice 재조회로도 확인
  const ir = await getIrSlice(fid, 0, 1, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  const irRuns = cell.paragraphs[0].runs;
  assert.equal(irRuns.length, 3, `IR runs 3개 기대 (실제 ${irRuns.length})`);
});

test('A3. insert_text_in_cell + apply_char_format → IR 의 새 run 에 color 노출', async () => {
  const fid = newFileId('sub7v2-insert-cell-style');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 1, cols: 2,
  });
  await postWorkbench(fid, 'insert_text_in_cell', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    offset: 0, text: '굵게',
  });
  // replace_cell_runs 로 style 지정해도 동일 결과 (insert + style 의 simpler path)
  const r = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [{ text: '빨강', style: { color: '#FF0000' } }],
  });
  assert.equal(r.status, 200);
  const ir = await getIrSlice(fid, 0, 1, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  const irRuns = cell.paragraphs[0].runs;
  assert.equal(irRuns.length, 1);
  assert.equal(irRuns[0].text, '빨강');
  assert.equal(irRuns[0].style.color, '#FF0000', `IR run.style.color #FF0000 누락 — ${JSON.stringify(irRuns[0])}`);
});

// ─── B. 본문 replace_runs cascade 회피 ───────────────────────────────────────

test('B1. 본문 replace_runs — run[1] 미지정 color 가 run[0] color 를 상속하지 않음', async () => {
  const fid = newFileId('sub7v2-body-no-cascade');
  await createSession(fid);
  // 본문 첫 문단에 텍스트가 있어야 함 — 빈 paragraph 에 replace 도 가능하지만 안전하게 텍스트 시드
  await postWorkbench(fid, 'insert_text', {
    section: 0, para: 0, offset: 0, text: '초기',
  });
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [
      { text: '빨강', style: { color: '#FF0000' } },
      { text: '노랑', style: { highlight: '#FFFF00' } },  // color 미지정
      { text: '파랑', style: { color: '#0000FF', bold: true } },
    ],
  });
  assert.equal(r.status, 200, `status ${r.status} body=${JSON.stringify(r.body).slice(0,400)}`);
  // IR slice 로 확인
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const para = ir.paragraphs[0];
  const runs = para.runs;
  assert.equal(runs.length, 3, `runs 3개 기대 (실제 ${runs.length}) — ${JSON.stringify(runs)}`);
  assert.equal(runs[0].text, '빨강');
  assert.equal(runs[0].style.color, '#FF0000');
  // run[1] color 는 #FF0000 (cascade 옛 동작) 이 아니어야 함
  assert.equal(runs[1].text, '노랑');
  assert.notEqual(
    runs[1].style.color, '#FF0000',
    `run[1] color 가 run[0] 의 #FF0000 을 상속 — cascade 사고 재발 (style=${JSON.stringify(runs[1].style)})`,
  );
  assert.equal(runs[1].style.highlight, '#FFFF00', `run[1] highlight 미반영 — ${JSON.stringify(runs[1].style)}`);
  // run[2] 도 cascade 검증 — bold 와 color 둘 다 정확히 자기 값
  assert.equal(runs[2].text, '파랑');
  assert.equal(runs[2].style.color, '#0000FF');
  assert.equal(runs[2].style.bold, true);
});

test('B2. 셀 replace_cell_runs — run[1] 미지정 color 가 run[0] color 를 상속하지 않음', async () => {
  const fid = newFileId('sub7v2-cell-no-cascade');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 1, cols: 2,
  });
  const r = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0, table_para: 1, row: 0, col: 0, cell_para: 0,
    runs: [
      { text: '빨강', style: { color: '#FF0000' } },
      { text: '노랑', style: { highlight: '#FFFF00' } },  // color 미지정
    ],
  });
  assert.equal(r.status, 200);
  const ir = await getIrSlice(fid, 0, 1, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  const runs = cell.paragraphs[0].runs;
  assert.equal(runs.length, 2);
  assert.equal(runs[0].style.color, '#FF0000');
  assert.notEqual(
    runs[1].style.color, '#FF0000',
    `셀 run[1] color 가 run[0] 의 #FF0000 을 상속 — 셀 경로에서 cascade 사고 재발 (style=${JSON.stringify(runs[1].style)})`,
  );
});

// ─── 실행 ───────────────────────────────────────────────────────────────────

(async () => {
  let pass = 0;
  let fail = 0;
  for (const t of TESTS) {
    try {
      await t.fn();
      console.log(`  ✓ ${t.name}`);
      pass++;
    } catch (e) {
      console.log(`  ✗ ${t.name}\n      ${e.message}`);
      fail++;
    }
  }
  console.log(`\nSub-7 v2 cell-style round-trip: pass=${pass} fail=${fail}`);
  if (fail > 0) {
    console.log(`✗ sub7v2-cell-style-round-trip FAIL`);
    process.exit(1);
  }
  console.log(`✓ sub7v2-cell-style-round-trip PASS`);
})();
