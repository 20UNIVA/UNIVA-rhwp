/**
 * Sub-2 e2e: audit + diff + ir-slice 세 엔드포인트 한꺼번에.
 *
 * 시나리오:
 *   1. insert_text 'foo' + replace_runs 'bar' + replace_runs 'baz'
 *      — insert_text 는 op_stash 미적재 (Sub-2 [2f.17a fix] 확정 사실),
 *      replace_runs 두 번만 op_stash 에 들어간다.
 *   2. GET /audit?seq_from=1&seq_to=100 — replace_runs row 최소 2개
 *   3. GET /diff?seq=<last> — before_paragraphs / after_paragraphs 존재
 *   4. GET /ir-slice?sec=0&para_start=0&para_end=1&mode=auto — paragraphs 1개
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getAudit,
  getDiff,
  getIrSlice,
} from './sub2-helpers.mjs';

async function main() {
  const fileId = newFileId('sub2-audit-diff-slice');
  await createSession(fileId);
  const { ws, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'foo',
  });
  await postWorkbench(fileId, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'bar' }],
  });
  await postWorkbench(fileId, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'baz' }],
  });

  // audit — insert_text 는 op_stash 미적재이므로 replace_runs 2 row 만 기대
  const audit = await getAudit(fileId, 1, 100);
  if (!Array.isArray(audit) || audit.length < 2) {
    throw new Error(`audit 결과 부족: ${JSON.stringify(audit)}`);
  }

  // diff — 마지막 seq
  const lastSeq = audit[audit.length - 1].seq;
  const diff = await getDiff(fileId, lastSeq);
  // camelCase 응답 — beforeParagraphs / afterParagraphs
  const before = diff.before_paragraphs ?? diff.beforeParagraphs;
  const after = diff.after_paragraphs ?? diff.afterParagraphs;
  if (!Array.isArray(before) || !Array.isArray(after)) {
    throw new Error(`diff shape: ${JSON.stringify(diff)}`);
  }

  // ir-slice — auto mode, 단일 문단
  const slice = await getIrSlice(fileId, 0, 0, 1, 'auto');
  if (!Array.isArray(slice.paragraphs) || slice.paragraphs.length !== 1) {
    throw new Error(`ir-slice paragraphs 1 기대: ${JSON.stringify(slice)}`);
  }

  console.log('=== Sub-2 audit/diff/ir-slice e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
