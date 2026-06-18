/**
 * Sub-3 IR compact 응답 검증.
 *
 * 텍스트 + bold + 표 + 셀 텍스트가 init.md spec §2 의 평탄 형식으로
 * 직렬화되는지를 워크벤치 호출 후 GET /ir-slice?mode=compact 응답으로
 * 확인한다.
 *
 * 시나리오:
 *   1. replace_runs 로 문단 0 에 'A' (bold) 삽입
 *   2. insert_paragraph 로 문단 1 추가
 *   3. insert_table 로 문단 2 위치에 2x2 표 삽입 (insert_after_para=1)
 *   4. replace_cell_runs 로 (0,0) 셀에 'CELL_TEXT' 작성
 *
 * 검증:
 *   1. defaults 박스 (run/paragraph)
 *   2. 첫 문단의 bold run
 *   3. type=table 의 rows/cols/cells
 *   4. 셀 (0,0) 의 'CELL_TEXT'
 *   5. nested 안 cell_locator (셀 (0,0) paragraph 의 4 좌표)
 *   5-1. paragraphs[] top-level 에 평탄 cell_locator entry 가 *없음*
 *   6. raw 모드 호환 (Sub-2 회귀 0)
 */

import {
  newFileId,
  createSession,
  postWorkbench,
  getIrSlice,
} from './sub2-helpers.mjs';
import assert from 'node:assert/strict';

async function main() {
  const fid = newFileId('sub3-ir-compact');
  await createSession(fid);

  // 1) 문단 0 에 bold 'A'
  const r1 = await postWorkbench(fid, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'A', style: { bold: true } }],
  });
  if (r1.status !== 200) throw new Error(`replace_runs 실패: ${JSON.stringify(r1)}`);

  // 2) 문단 1 삽입
  const r2 = await postWorkbench(fid, 'insert_paragraph', {
    section: 0,
    after_para: 0,
    count: 1,
  });
  if (r2.status !== 200) throw new Error(`insert_paragraph 실패: ${JSON.stringify(r2)}`);

  // 3) 문단 2 위치에 2x2 표
  const r3 = await postWorkbench(fid, 'insert_table', {
    section: 0,
    insert_after_para: 1,
    rows: 2,
    cols: 2,
  });
  if (r3.status !== 200) throw new Error(`insert_table 실패: ${JSON.stringify(r3)}`);

  // 4) (0,0) 셀에 'CELL_TEXT'
  const r4 = await postWorkbench(fid, 'replace_cell_runs', {
    section: 0,
    table_para: 2,
    row: 0,
    col: 0,
    cell_para: 0,
    runs: [{ text: 'CELL_TEXT' }],
  });
  if (r4.status !== 200) throw new Error(`replace_cell_runs 실패: ${JSON.stringify(r4)}`);

  // === compact 모드 응답 ===
  const compact = await getIrSlice(fid, 0, 0, null, 'compact');
  console.log('compact response (head):', JSON.stringify(compact, null, 2).slice(0, 800));

  // 1) defaults 박스
  assert.ok(compact.defaults, 'defaults 박스 누락');
  assert.equal(compact.defaults.run.bold, false, 'defaults.run.bold 기본값 false');
  assert.equal(compact.defaults.run.color, '#000000', 'defaults.run.color 기본값 #000000');
  assert.equal(compact.defaults.paragraph.align, 'left', 'defaults.paragraph.align 기본값 left');

  // 2) bold 'A' 가 들어간 문단 찾기 (insert_paragraph 가 좌표를 미는 case 대비
  // — para 0 또는 para 1 어디든 'A' 를 가진 첫 text 문단을 찾는다)
  // Sub-3 v2 Phase 3 — type:"text" 는 omit. type 부재 시 기본 'text', 'table' 만 명시.
  assert.ok(Array.isArray(compact.paragraphs), 'paragraphs 배열 없음');
  const boldPara = compact.paragraphs.find((p) => {
    const isText = (p.type ?? 'text') === 'text';
    return isText && Array.isArray(p.runs) && p.runs.some((r) => r.text === 'A');
  });
  assert.ok(boldPara, `bold 'A' 가 들어간 문단 없음: ${JSON.stringify(compact.paragraphs)}`);
  const boldRun = boldPara.runs.find((r) => r.text === 'A');
  assert.equal(boldRun.style?.bold, true, `bold 누락: ${JSON.stringify(boldRun)}`);

  // 3) 표 문단
  const table = compact.paragraphs.find((p) => p.type === 'table');
  assert.ok(table, '표 문단 없음');
  assert.equal(table.rows, 2, 'table.rows !== 2');
  assert.equal(table.cols, 2, 'table.cols !== 2');
  assert.equal(table.cells.length, 4, 'table.cells.length !== 4');

  // 4) 셀 (0,0) 'CELL_TEXT'
  const cell00 = table.cells.find((c) => c.row === 0 && c.col === 0);
  assert.ok(cell00, '셀 (0,0) 없음');
  const cellPara = cell00.paragraphs[0];
  const cellText = cellPara.text ?? cellPara.runs?.map((r) => r.text).join('');
  assert.equal(cellText, 'CELL_TEXT', `셀 텍스트 불일치: ${cellText}`);

  // 5) Sub-3 v2: nested 안 cell_locator — 셀 (0,0) 의 paragraph 가 cell_locator 4 좌표 보유.
  const cellPara0 = cell00.paragraphs[0];
  assert.ok(cellPara0.cell_locator, 'nested cell_locator 누락');
  assert.equal(cellPara0.cell_locator.table_para, table.para, 'cell_locator.table_para 불일치');
  assert.equal(cellPara0.cell_locator.row, 0, 'cell_locator.row !== 0');
  assert.equal(cellPara0.cell_locator.col, 0, 'cell_locator.col !== 0');
  assert.equal(cellPara0.cell_locator.cell_para, 0, 'cell_locator.cell_para !== 0');

  // 5-1) Sub-3 v2: paragraphs[] top-level 에는 cell_locator 평탄 entry 가 *없어야* 함.
  const flatCellEntries = compact.paragraphs.filter((p) => p.cell_locator);
  assert.equal(
    flatCellEntries.length,
    0,
    `평탄 cell_locator entry 가 남음: ${flatCellEntries.length} 건`,
  );

  // 5-2) Sub-3 v2 Phase 3: 구조 키 omit 검증.
  //   - id 항상 omit (text/table 모두)
  //   - 단일 sec 응답이므로 paragraph 마다의 sec 키 omit
  //   - type:"text" 는 omit (기본값), table 만 type:"table" 명시
  for (const p of compact.paragraphs) {
    assert.equal(p.id, undefined, `paragraph id 잔존: ${JSON.stringify(p).slice(0, 120)}`);
    assert.equal(p.sec, undefined, `단일 sec 응답에서 paragraph sec 잔존: ${JSON.stringify(p).slice(0, 120)}`);
    if (p.type !== undefined) {
      assert.equal(p.type, 'table', `text 는 type 부재여야 함, table 만 명시: ${p.type}`);
    }
  }
  // doc_meta.anchor.sec 는 그대로 유지 — 응답 전체의 sec 진실.
  assert.equal(typeof compact.doc_meta?.anchor?.sec, 'number', 'doc_meta.anchor.sec 누락');

  // 6) raw 모드 호환
  const raw = await getIrSlice(fid, 0, 0, null, 'raw');
  assert.equal(raw.mode, 'raw', 'raw.mode !== raw');
  assert.ok(Array.isArray(raw.paragraphs), 'raw.paragraphs 배열 없음');

  // 7) Sub-3 v2 — page query 시나리오.
  //   본 e2e 문서는 짧아 보통 1 페이지지만, page=0 은 응답 형식만 동일하면 OK.
  //   getIrSlice helper 가 page 파라미터를 모르므로 직접 fetch.
  const BASE_URL = process.env.RHWP_SERVER_URL || 'http://127.0.0.1:7710/hwp';
  const getWithPage = async (page) => {
    const resp = await fetch(
      `${BASE_URL}/sessions/${encodeURIComponent(fid)}/ir-slice?mode=compact&page=${page}`,
    );
    if (!resp.ok) throw new Error(`getWithPage 실패 ${resp.status}: ${await resp.text()}`);
    return resp.json();
  };

  const page0 = await getWithPage(0);
  assert.ok(page0.defaults, 'page=0 응답에 defaults 누락');
  assert.ok(Array.isArray(page0.paragraphs), 'page=0 응답의 paragraphs 배열 아님');

  // 8) page=999 (범위 외) — 서버 측 fallback 으로 sec/para_start/para_end 폴백 (응답 형식 동일).
  const pageOOB = await getWithPage(999);
  assert.ok(pageOOB.defaults, 'page=999 응답에 defaults 누락 (fallback 동작 일관성)');
  assert.ok(Array.isArray(pageOOB.paragraphs), 'page=999 응답의 paragraphs 배열 아님');

  console.log('✓ sub3-ir-compact PASS');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
