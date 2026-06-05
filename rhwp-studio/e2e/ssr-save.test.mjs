/**
 * E2E 테스트: SSR 저장(서버 minio 덮어쓰기) — Task #999 Stage 10
 *
 * 사전 조건: vite dev server(:7700) + rhwp-server(:7720, UPLOAD/DOWNLOAD env)
 *
 * 시나리오:
 *   빈 문서 진입 → 편집 → file:save → 서버 /save 호출(덮어쓰기) → dirty 해제 확인
 *   + 서버 IR에 편집 반영 확인
 */
import { runTest, assert, clickEditArea, typeText } from './helpers.mjs';

const SERVER_BASE = process.env.SSR_SERVER || 'http://127.0.0.1:7720';
const VITE_URL = process.env.VITE_URL || 'http://localhost:7700';
const ENTRY = `${VITE_URL}/?ssrBase=${encodeURIComponent(SERVER_BASE)}`;
const MARK = 'SAVEME';

runTest('SSR 저장(서버 덮어쓰기) E2E', async ({ page }) => {
  await page.goto(ENTRY, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForFunction(() => !!window.__wasm && !!window.__dispatcher, { timeout: 15000 });
  await page.waitForFunction(() => window.__ssr?.fileId != null, { timeout: 15000 });
  const fid = await page.evaluate(() => window.__ssr.fileId);
  console.log(`  빈문서 fileId=${fid}`);

  // 편집 → dirty=true
  await clickEditArea(page);
  await typeText(page, MARK);
  await page.evaluate(() => new Promise((r) => setTimeout(r, 400)));
  const dirtyBefore = await page.evaluate(() => window.__documentState.isDirty());
  assert(dirtyBefore === true, '편집 후 dirty=true');

  // file:save → SSR 서버 저장 라우팅
  await page.evaluate(() => window.__dispatcher.dispatch('file:save'));
  await page.waitForFunction(() => window.__documentState.isDirty() === false, { timeout: 10000 });
  const dirtyAfter = await page.evaluate(() => window.__documentState.isDirty());
  console.log(`  file:save 후 dirty=${dirtyAfter}`);
  assert(dirtyAfter === false, 'file:save(서버 저장) 후 dirty 해제');

  // 서버 IR에 편집 반영(미러링+저장)
  const ir = await fetch(`${SERVER_BASE}/sessions/${fid}/ir`).then((r) => r.json());
  const txt = ir.sections.flatMap((s) => s.paragraphs.map((p) => p.text)).join('');
  console.log(`  서버 IR: ${txt.slice(0, 20)}`);
  assert(txt.includes(MARK), `서버 세션에 편집 반영(${MARK})`);
}, { skipLoadApp: true });
