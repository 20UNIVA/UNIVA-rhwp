/**
 * Sub-2 e2e: undo.
 *
 * 시나리오:
 *   1. insert_text 'A' + replace_runs 'B' + replace_runs 'C'
 *      → op_stash 는 Sub-2 신규 12 액션만 적재 (insert_text 는 append_op 사용 → stash 미적재).
 *        결과적으로 stash 2 entry (replace_runs B, replace_runs C).
 *   2. 초기 텍스트 'C' 확인
 *   3. undo 1회 → 'B' (C → B 역적용)
 *   4. undo 2회 → 'A' (B → A 역적용; replace_runs B 의 before_blob 은 insert_text 직후 상태)
 *   5. undo 3회 → 409 NO_UNDO_AVAILABLE (빈 stash)
 *   각 undo 후 ServerEvent::SnapshotRestored broadcast 수신
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  postUndo,
  getIr,
  findEvent,
  wait,
} from './sub2-helpers.mjs';

function paraText(ir, idx = 0) {
  return ir.paragraphs?.[idx]?.text ?? ir.sections?.[0]?.paragraphs?.[idx]?.text;
}

async function main() {
  const fileId = newFileId('sub2-undo');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  // 1. 변경 누적 — A → B → C
  let r = await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'A',
  });
  if (r.status !== 200) throw new Error(`insert_text A 실패: ${JSON.stringify(r)}`);

  r = await postWorkbench(fileId, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'B' }],
  });
  if (r.status !== 200) throw new Error(`replace_runs B 실패: ${JSON.stringify(r)}`);

  r = await postWorkbench(fileId, 'replace_runs', {
    section: 0,
    para: 0,
    runs: [{ text: 'C' }],
  });
  if (r.status !== 200) throw new Error(`replace_runs C 실패: ${JSON.stringify(r)}`);

  // 2. 초기 상태 — 'C'
  let ir = await getIr(fileId);
  if (paraText(ir) !== 'C') {
    throw new Error(`초기 상태 mismatch: '${paraText(ir)}'`);
  }

  // 3. undo 1 — 'B'
  let beforeLen = received.length;
  let u = await postUndo(fileId);
  if (u.status !== 200) throw new Error(`undo 1: ${JSON.stringify(u)}`);
  await wait(500);
  const restoredEv1 = received
    .slice(beforeLen)
    .find((ev) => ev && ev.kind === 'snapshot_restored');
  if (!restoredEv1) {
    throw new Error(
      `undo 1 SnapshotRestored 미수신: received=${JSON.stringify(received.slice(beforeLen))}`,
    );
  }
  ir = await getIr(fileId);
  const t1 = paraText(ir);
  if (t1 !== 'B') throw new Error(`undo 1 후 text mismatch: '${t1}'`);

  // 4. undo 2 — 'A' (replace_runs B 의 before_blob 은 insert_text 직후 'A' 상태)
  u = await postUndo(fileId);
  if (u.status !== 200) throw new Error(`undo 2: ${JSON.stringify(u)}`);
  ir = await getIr(fileId);
  const t2 = paraText(ir);
  if (t2 !== 'A') throw new Error(`undo 2 후 text mismatch: '${t2}'`);

  // 5. undo 3 — 빈 stash 409 (insert_text 는 stash 미적재)
  u = await postUndo(fileId);
  if (u.status !== 409) {
    throw new Error(`빈 stash 409 기대 — got status=${u.status} body=${JSON.stringify(u.body)}`);
  }

  console.log('=== Sub-2 undo e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
