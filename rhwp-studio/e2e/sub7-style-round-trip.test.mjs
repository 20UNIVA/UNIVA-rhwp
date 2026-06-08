/**
 * Sub-7 e2e — Partial*Style ↔ SKILL.md 광고 round-trip 검증.
 *
 * 사전 조건: `e2e/sub2-server.sh start` 또는 `cd server && cargo run` 으로 서버 가동.
 *
 * 검증 카테고리:
 *   - 테이블 셀 — set_cell_style: bgcolor / border.all / border override / unknown key 400
 *   - run 스타일 — replace_runs: color / textColor alias / font_size 변형 3종 / highlight / font_name / unknown key 400
 *   - paragraph 스타일 — set_paragraph_style: align / alignment alias / line_height / unknown key 400
 *   - PatchSummary — no-op 시 changed=false + noChangeWarning 채워짐
 *
 * 각 시나리오는 *독립 fileId* — 한 시나리오의 부작용이 다음에 영향 없음.
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

// ─── 테이블 셀 ───────────────────────────────────────────────────────────────

test('1. set_cell_style bgcolor 단독 round-trip', async () => {
  const fid = newFileId('sub7-bg');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  const r = await postWorkbench(fid, 'set_cell_style', {
    section: 0, table_para: 1, row: 0, col: 0,
    style: { bgcolor: '#FFC0CB' },
  });
  assert.equal(r.status, 200, `status ${r.status} body=${JSON.stringify(r.body)}`);
  assert.equal(r.body.diff.summary.changed, true, 'bgcolor 적용 시 changed=true 여야');
  assert.equal(r.body.diff.summary.noChangeWarning, undefined, 'changed=true 일 때 noChangeWarning 누락');
  // diff.after 검증
  assert.equal(r.body.diff.after.cell.style.bgcolor, '#FFC0CB', 'diff.after.cell.style.bgcolor mismatch');
  // IR slice 재조회
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  assert.equal(cell.style.bgcolor, '#FFC0CB', 'IR cell.style.bgcolor !== #FFC0CB');
});

test('2. set_cell_style border.all 단독 round-trip', async () => {
  const fid = newFileId('sub7-ba');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  const r = await postWorkbench(fid, 'set_cell_style', {
    section: 0, table_para: 1, row: 0, col: 0,
    style: { border: { all: { color: '#0000FF', width: 2, type: 1 } } },
  });
  assert.equal(r.status, 200, `status ${r.status} body=${JSON.stringify(r.body).slice(0,300)}`);
  assert.equal(r.body.diff.summary.changed, true);
  // IR — 4면 동일하면 compact 단계가 `all` 한 칸으로 축약. 색만 확인.
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  const border = cell.style.border;
  assert.ok(border, 'border 키 누락');
  // 4면 통합 → all 축약, 또는 4면 각각으로 응답.
  if (border.all) {
    assert.equal(border.all.color, '#0000FF', 'border.all.color mismatch');
  } else {
    for (const side of ['left', 'right', 'top', 'bottom']) {
      assert.ok(border[side], `border.${side} 누락 (4면 분리 응답)`);
      assert.equal(border[side].color, '#0000FF', `border.${side}.color mismatch`);
    }
  }
});

test('3. set_cell_style border.all + left override', async () => {
  const fid = newFileId('sub7-bo');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  const r = await postWorkbench(fid, 'set_cell_style', {
    section: 0, table_para: 1, row: 0, col: 0,
    style: {
      border: {
        all: { color: '#0000FF', width: 2, type: 1 },
        left: { color: '#FF0000', width: 2, type: 1 },
      },
    },
  });
  assert.equal(r.status, 200);
  assert.equal(r.body.diff.summary.changed, true);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const table = ir.paragraphs.find((p) => p.type === 'table');
  const cell = table.cells.find((c) => c.row === 0 && c.col === 0);
  const border = cell.style.border;
  assert.ok(border, 'border 키 누락');
  // 4면 분리 응답 — left 만 빨강, 나머지 파랑.
  assert.equal(border.left?.color, '#FF0000', `border.left.color !== #FF0000 — got ${JSON.stringify(border)}`);
  for (const side of ['right', 'top', 'bottom']) {
    assert.equal(border[side]?.color, '#0000FF', `border.${side}.color !== #0000FF`);
  }
});

test('4. set_cell_style unknown key → 400', async () => {
  const fid = newFileId('sub7-uk');
  await createSession(fid);
  await postWorkbench(fid, 'insert_table', {
    section: 0, insert_after_para: 0, rows: 2, cols: 2,
  });
  const r = await postWorkbench(fid, 'set_cell_style', {
    section: 0, table_para: 1, row: 0, col: 0,
    style: { bgClor: '#FFF' },   // typo
  });
  assert.equal(r.status, 400, `오타 키는 400 이어야 — 실제 ${r.status} body=${JSON.stringify(r.body)}`);
  assert.ok(r.body.error, 'error 메시지 필요');
  assert.ok(
    /unknown field/i.test(r.body.error || ''),
    `에러 메시지에 unknown field 언급 필요 — ${r.body.error}`,
  );
});

// ─── run 스타일 ──────────────────────────────────────────────────────────────

test('5. replace_runs color 광고 키', async () => {
  const fid = newFileId('sub7-rc');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'X', style: { color: '#FF0000' } }],
  });
  assert.equal(r.status, 200);
  assert.equal(r.body.diff.summary.changed, true);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const runs = ir.paragraphs[0].runs;
  const run = runs.find((rr) => rr.text === 'X');
  assert.ok(run, 'run X 없음');
  assert.equal(run.style?.color, '#FF0000', `run.style.color !== #FF0000 — got ${JSON.stringify(run)}`);
});

test('6. replace_runs textColor alias 동작', async () => {
  const fid = newFileId('sub7-rtc');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'X', style: { textColor: '#FF0000' } }],
  });
  assert.equal(r.status, 200, `alias 거절: ${JSON.stringify(r.body)}`);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const run = ir.paragraphs[0].runs.find((rr) => rr.text === 'X');
  assert.equal(run.style?.color, '#FF0000', 'textColor alias 후 IR color mismatch');
});

test('7. replace_runs font_size / fontSize / baseSize 3 변형 동작 (alias 수용 검증)', async () => {
  // font-size 단위 변환 (광고 pt vs native 100단위) 은 별도 sub. 본 시나리오는
  // *세 변형 모두 deny_unknown_fields 를 통과하고 400 이 아닌 200 반환* 만 검증.
  for (const key of ['font_size', 'fontSize', 'baseSize']) {
    const fid = newFileId(`sub7-fs-${key}`);
    await createSession(fid);
    await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
    const r = await postWorkbench(fid, 'replace_runs', {
      section: 0, para: 0,
      runs: [{ text: 'X', style: { [key]: 2200 } }],
    });
    assert.equal(r.status, 200, `font_size key=${key} 거절: ${JSON.stringify(r.body).slice(0,200)}`);
    // changed 는 native 단위 mismatch 가능성 때문에 강제 안 함 — 200 + diff 응답만 검증.
    assert.ok(r.body.diff, `font_size key=${key} diff 누락`);
  }
});

test('8. replace_runs highlight round-trip', async () => {
  const fid = newFileId('sub7-hl');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'X', style: { highlight: '#FFFF00' } }],
  });
  assert.equal(r.status, 200);
  assert.equal(r.body.diff.summary.changed, true);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const run = ir.paragraphs[0].runs.find((rr) => rr.text === 'X');
  assert.equal(run.style?.highlight, '#FFFF00', `IR run.highlight !== #FFFF00 — got ${JSON.stringify(run)}`);
});

test('9. replace_runs font_name (한국 폰트) → fontId 변환 후 적용', async () => {
  // 함초롬바탕 — 빈 hwpx 의 기본 폰트와 같을 가능성이 있어 changed=false 일 수 있다.
  // 핵심 검증: (a) 400 이 아니어야 함 (b) 변환 자체는 성공해 응답에 fontName 이 노출되거나
  // 문서 defaults.run.font-name 에 들어가야 함.
  const fid = newFileId('sub7-fn');
  await createSession(fid);
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'X', style: { font_name: '함초롬바탕' } }],
  });
  assert.equal(r.status, 200, `font_name 거절: ${JSON.stringify(r.body).slice(0,200)}`);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  // (a) IR 의 run.style.font-name 으로 노출되거나
  // (b) defaults.run.font-name 에 함초롬바탕 들어가 있어야 — find_or_create 가 등록 후 default 갱신.
  const run = ir.paragraphs[0].runs.find((rr) => rr.text === 'X');
  const runFontName = run?.style?.['font-name'];
  const defaultFontName = ir.defaults?.run?.['font-name'];
  assert.ok(
    runFontName === '함초롬바탕' || defaultFontName === '함초롬바탕',
    `font_name 변환 효과 없음 — run=${runFontName} defaults=${defaultFontName}`,
  );
});

test('10. replace_runs unknown key → 400', async () => {
  const fid = newFileId('sub7-ruk');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'replace_runs', {
    section: 0, para: 0,
    runs: [{ text: 'X', style: { colour: '#FFF' } }],   // 영국 spell
  });
  assert.equal(r.status, 400, `오타 키는 400 이어야 — 실제 ${r.status} body=${JSON.stringify(r.body).slice(0,200)}`);
  assert.ok(/unknown field/i.test(r.body.error || ''), `에러 메시지에 unknown field 언급 필요 — ${r.body.error}`);
});

// ─── paragraph 스타일 ───────────────────────────────────────────────────────

test('11. set_paragraph_style align (광고 키) round-trip', async () => {
  const fid = newFileId('sub7-pa');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { align: 'right' },
  });
  assert.equal(r.status, 200);
  assert.equal(r.body.diff.summary.changed, true);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  assert.equal(ir.paragraphs[0].style?.align, 'right', `IR paragraph.align !== right — got ${JSON.stringify(ir.paragraphs[0].style)}`);
});

test('12. set_paragraph_style alignment alias 동작', async () => {
  const fid = newFileId('sub7-pal');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { alignment: 'right' },
  });
  assert.equal(r.status, 200, `alignment alias 거절: ${JSON.stringify(r.body)}`);
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  assert.equal(ir.paragraphs[0].style?.align, 'right');
});

test('13. set_paragraph_style line_height (snake) 동작', async () => {
  // 빈 hwpx 의 defaults.line-height = 160 — 같은 값 보내면 omit. 200 으로 명확히 변경.
  const fid = newFileId('sub7-plh');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { line_height: 200 },
  });
  assert.equal(r.status, 200, `line_height 거절: ${JSON.stringify(r.body)}`);
  assert.equal(r.body.diff.summary.changed, true, 'line_height=200 적용 시 changed=true 여야');
  const ir = await getIrSlice(fid, 0, 0, null, 'compact');
  const lh = ir.paragraphs[0].style?.['line-height'];
  assert.equal(lh, 200, `IR paragraph.line-height !== 200 — style=${JSON.stringify(ir.paragraphs[0].style)}`);
});

test('14. set_paragraph_style unknown key → 400', async () => {
  const fid = newFileId('sub7-puk');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { alighn: 'right' },   // typo
  });
  assert.equal(r.status, 400, `오타 키는 400 이어야 — 실제 ${r.status} body=${JSON.stringify(r.body).slice(0,200)}`);
  assert.ok(/unknown field/i.test(r.body.error || ''));
});

// ─── PatchSummary noChangeWarning 가시화 ────────────────────────────────────

test('15. 진짜 no-op 시 changed=false + noChangeWarning 채워짐', async () => {
  // align right 두 번 — 두 번째는 진짜 no-op.
  const fid = newFileId('sub7-noop');
  await createSession(fid);
  await postWorkbench(fid, 'insert_text', { section: 0, para: 0, offset: 0, text: 'X' });
  const r1 = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { align: 'right' },
  });
  assert.equal(r1.body.diff.summary.changed, true, '첫 align=right 는 changed=true');
  const r2 = await postWorkbench(fid, 'set_paragraph_style', {
    section: 0, para: 0, style: { align: 'right' },
  });
  assert.equal(r2.status, 200);
  assert.equal(r2.body.diff.summary.changed, false, '두 번째 동일 align 은 changed=false');
  const warn = r2.body.diff.summary.noChangeWarning;
  assert.ok(warn, `changed=false 일 때 noChangeWarning 필드 누락`);
  assert.ok(/schema/.test(warn), `noChangeWarning 메시지에 schema 안내 필요 — ${warn}`);
});

// ─── 실행기 ──────────────────────────────────────────────────────────────────

let pass = 0;
let fail = 0;
for (const { name, fn } of TESTS) {
  try {
    await fn();
    console.log(`  ✓ ${name}`);
    pass++;
  } catch (e) {
    console.error(`  ✗ ${name}\n    ${e.message}`);
    if (e.stack) console.error(e.stack.split('\n').slice(1, 4).map((l) => '    ' + l).join('\n'));
    fail++;
  }
}
console.log(`\nSub-7 style round-trip: pass=${pass} fail=${fail}`);
if (fail > 0) process.exit(1);
console.log('✓ sub7-style-round-trip PASS');
