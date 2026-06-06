/**
 * Sub-2 Canvas 시각 검증 e2e — insert_text.
 *
 * 시나리오:
 *   1) 빈 hwpx 세션 생성 + Puppeteer 로 페이지 열기 (`?fileId=<id>`).
 *   2) Canvas before screenshot.
 *   3) /workbench insert_text 호출 → 서버가 IR 수정 + ServerEvent::Ops 발행.
 *   4) 브라우저가 ws 로 ops 수신 + wasm 미러 + `document-changed` emit → Canvas 재렌더.
 *   5) Canvas after screenshot + pixel diff.
 *
 * 검증 — diff ratio > 1%. 0 에 가까우면 *Critical fix #2 류 사고* 신호:
 *   - 서버 broadcast 안 됨,
 *   - 또는 main.ts 에서 ops 적용 후 eventBus.emit('document-changed') 누락,
 *   - 또는 wasm 미러 자체 실패.
 *
 * Sub-1 의 ws-bridge.test.mjs 가 못 잡는 사고 — 서버 broadcast 만 검증하므로
 * 클라이언트 측 렌더 누락은 비가시. 본 e2e 가 그 마지막 한 단계를 보강한다.
 */

import { newFileId, createSession, postWorkbench, wait } from './sub2-helpers.mjs';
import {
  launchBrowser,
  openSession,
  snapCanvas,
  diffPixels,
  saveDiffArtifacts,
} from './sub2-canvas-helpers.mjs';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ARTIFACT_DIR = `${__dirname}/screenshots/sub2-canvas`;
// 빈 문서에 짧은 문자열 ("CANVAS-TEST" 11 자) 삽입 — 화면 전체 대비 글자 영역 비율은 0.05~0.1%.
// 임계값 — Canvas 가 *전혀 안 그려진 경우* 와 *글자 한 줄 들어간 경우* 만 구별하면 충분.
// 0.02% = 약 180 픽셀. 일반 e2e 가 "0 % vs 글자 한 줄" 사이 결정에 사용.
const DIFF_THRESHOLD = 0.0002;

async function main() {
  mkdirSync(ARTIFACT_DIR, { recursive: true });

  const fileId = newFileId('sub2-canvas-insert-text');
  await createSession(fileId);
  console.log(`세션 생성: ${fileId}`);

  console.log('Puppeteer 브라우저 가동...');
  const browser = await launchBrowser();
  try {
    const page = await openSession(browser, fileId);
    console.log('세션 페이지 로드 완료');

    const before = await snapCanvas(page);
    console.log(`before screenshot: ${before.length} bytes`);

    const resp = await postWorkbench(fileId, 'insert_text', {
      section: 0,
      para: 0,
      offset: 0,
      text: 'CANVAS-TEST',
    });
    if (resp.status !== 200) {
      throw new Error(`insert_text 실패: ${JSON.stringify(resp)}`);
    }
    console.log('insert_text 워크벤치 호출 성공');

    // ws broadcast 수신 + wasm 미러 + Canvas refresh 대기.
    await wait(2500);

    const after = await snapCanvas(page);
    console.log(`after screenshot: ${after.length} bytes`);

    const d = diffPixels(before, after);
    if (d.note) {
      saveDiffArtifacts(before, after, ARTIFACT_DIR, 'insert-text-sizemismatch');
      throw new Error(`Canvas 비교 불가 — ${d.note}`);
    }
    const pct = (d.ratio * 100).toFixed(3);
    console.log(`pixel diff: ${d.diff}/${d.total} (${pct}%)`);

    if (d.ratio < DIFF_THRESHOLD) {
      saveDiffArtifacts(before, after, ARTIFACT_DIR, 'insert-text-fail');
      throw new Error(
        `Canvas 갱신 없음 — ratio=${d.ratio.toFixed(6)} < ${DIFF_THRESHOLD}. `
          + `Critical fix #2 류 사고 신호 — document-changed emit 누락 또는 ws 미러 실패. `
          + `산출물: ${ARTIFACT_DIR}/insert-text-fail-*.png`,
      );
    }

    console.log('=== Sub-2 Canvas insert_text 시각 검증 통과 ===');
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
