/**
 * Sub-2 e2e: partial update — set_paragraph_style 부분 필드 보존.
 *
 * 시나리오:
 *   1. insert_text 'partial' 삽입 → 초기 para_shape_id 기록
 *   2. set_paragraph_style alignment 만 변경 → para_shape_id 변화
 *   3. line_spacing 만 변경 → para_shape_id 또 변화 (이전 alignment 는 유지된 채 합쳐진 새 shape)
 *
 * ir-slice raw 모드는 alignment/line_spacing 을 직접 노출하지 않으므로,
 * 변경 적용은 para_shape_id 변화로 간접 확인한다.
 * 정확한 필드 유지 검사는 다음 단계 (직접 IR fetch 또는 별도 endpoint) 에서 강화.
 */

import {
  newFileId,
  createSession,
  subscribeWs,
  postWorkbench,
  getIrSlice,
} from './sub2-helpers.mjs';

async function main() {
  const fileId = newFileId('sub2-partial-update');
  await createSession(fileId);
  const { ws, opened } = subscribeWs(fileId);
  await opened;
  console.log(`WS 연결 OK — ${fileId}`);

  // 1. 텍스트 삽입
  await postWorkbench(fileId, 'insert_text', {
    section: 0,
    para: 0,
    offset: 0,
    text: 'partial',
  });
  const slice0 = await getIrSlice(fileId, 0, 0, 1, 'raw');
  const shape0 = slice0.paragraphs[0].para_shape_id;

  // 2. alignment 만 변경
  const r1 = await postWorkbench(fileId, 'set_paragraph_style', {
    section: 0,
    para: 0,
    style: { alignment: 'right' },
  });
  if (r1.status !== 200) throw new Error(`alignment 변경 실패: ${JSON.stringify(r1)}`);

  const slice1 = await getIrSlice(fileId, 0, 0, 1, 'raw');
  const shape1 = slice1.paragraphs[0].para_shape_id;
  if (shape1 === shape0) {
    throw new Error(`alignment 변경 후 para_shape_id 동일: shape0=${shape0}`);
  }

  // 3. line_spacing 만 변경
  const r2 = await postWorkbench(fileId, 'set_paragraph_style', {
    section: 0,
    para: 0,
    style: { line_spacing: 200.0 },
  });
  if (r2.status !== 200) throw new Error(`line_spacing 변경 실패: ${JSON.stringify(r2)}`);

  const slice2 = await getIrSlice(fileId, 0, 0, 1, 'raw');
  const shape2 = slice2.paragraphs[0].para_shape_id;
  if (shape2 === shape1) {
    throw new Error(`line_spacing 변경 후 para_shape_id 동일: shape1=${shape1}`);
  }

  console.log('=== Sub-2 partial update e2e 통과 ===');
  ws.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
