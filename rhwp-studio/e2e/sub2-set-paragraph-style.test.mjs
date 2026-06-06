/**
 * Sub-2 e2e: set_paragraph_style.
 *
 * 시나리오:
 *   1. insert_text 로 문단 0 에 'hello' 삽입 → 초기 para_shape_id 기록
 *   2. set_paragraph_style alignment='right' 적용
 *   3. ir-slice raw 모드 — paragraphs[0].para_shape_id 가 초기값과 달라졌는지 검증
 *      (raw 모드에 alignment 직접 노출 없음. 새 shape id 부여로 변경 확인)
 *   4. WS broadcast 에 set_paragraph_style op 수신
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIrSlice,
  findEvent,
  wait,
} from './sub2-helpers.mjs';

async function main() {
  const fileId = newFileId('sub2-set-paragraph-style');
  await createSession(fileId);
  const { ws, received, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  // 1. 텍스트 삽입
  const ins = await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'hello',
  });
  if (ins.status !== 200) throw new Error(`insert_text 실패: ${JSON.stringify(ins)}`);

  const before = await getIrSlice(fileId, 0, 0, 1, 'raw');
  const beforeShapeId = before.paragraphs[0].para_shape_id;

  // 2. set_paragraph_style — alignment 만
  const resp = await postWorkbench(fileId, 'set_paragraph_style', {
    section: 0,
    para: 0,
    style: { alignment: 'right' },
  });
  if (resp.status !== 200) {
    throw new Error(`set_paragraph_style 실패: ${JSON.stringify(resp)}`);
  }
  if (resp.body.applied !== 'ops') {
    throw new Error(`applied !== 'ops': ${JSON.stringify(resp.body)}`);
  }

  // 3. ir-slice — para_shape_id 변화 확인
  const after = await getIrSlice(fileId, 0, 0, 1, 'raw');
  const afterShapeId = after.paragraphs[0].para_shape_id;
  if (afterShapeId === beforeShapeId) {
    throw new Error(
      `para_shape_id 변경 없음: before=${beforeShapeId} after=${afterShapeId}`,
    );
  }

  // 4. WS broadcast
  await wait(500);
  const opsEv = findEvent(
    received,
    'ops',
    (ev) => Array.isArray(ev.ops) && ev.ops.some((o) => o.op === 'set_paragraph_style'),
  );
  if (!opsEv) {
    throw new Error(
      `set_paragraph_style broadcast 미수신: received=${JSON.stringify(received)}`,
    );
  }

  console.log('=== Sub-2 set_paragraph_style e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
