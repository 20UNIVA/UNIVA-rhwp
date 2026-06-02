/**
 * E2E 테스트: SSR 빈 문서 업로드 + 열기 분기 (Task #999 Stage 9)
 *
 * 사전 조건:
 *   - vite dev server (VITE_URL, 기본 :7700)
 *   - rhwp-server (127.0.0.1:7720) + UPLOAD_URL/DOWNLOAD_URL 환경변수로 minio 연동
 *
 * 시나리오:
 *   A) fileId 없이 ?ssrBase= 로 진입 → 빈 문서 자동 업로드(fileId 발급) + 미러링
 *   B) 빈 문서 미편집 상태에서 다른 문서 열기 → 새 fileId로 전환(빈 세션 닫힘)
 *   C) 빈 문서 편집 후 다른 문서 열기 → 빈 세션은 서버에 편집 보존(유지) + 새 fileId
 */
import { runTest, assert, clickEditArea, typeText } from './helpers.mjs';

const SERVER_BASE = process.env.SSR_SERVER || 'http://127.0.0.1:7720';
const VITE_URL = process.env.VITE_URL || 'http://localhost:7700';
const ENTRY = `${VITE_URL}/?ssrBase=${encodeURIComponent(SERVER_BASE)}`;

async function irText(fileId) {
  const r = await fetch(`${SERVER_BASE}/sessions/${encodeURIComponent(fileId)}/ir`);
  if (!r.ok) return null;
  const j = await r.json();
  return j.sections.flatMap((s) => s.paragraphs.map((p) => p.text)).join('');
}

runTest('SSR 빈문서 업로드 + 열기 분기 E2E', async ({ page, browser }) => {
  // ── 시나리오 A: 빈 문서 진입 → 자동 업로드 ──
  await page.goto(ENTRY, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForFunction(() => !!window.__wasm, { timeout: 15000 });
  await page.waitForFunction(() => window.__ssr?.fileId != null, { timeout: 15000 });
  const blankFid = await page.evaluate(() => window.__ssr.fileId);
  const isBlank = await page.evaluate(() => window.__ssr.isBlank);
  console.log(`  [A] 빈문서 fileId=${blankFid}, isBlank=${isBlank}`);
  assert(!!blankFid, '빈 문서 진입 시 fileId 자동 발급');
  assert(isBlank === true, '빈 문서 세션 isBlank=true');
  assert((await irText(blankFid)) != null, '서버에 빈문서 세션 생성됨');

  // ── 시나리오 C: 빈 문서를 편집한 뒤 다른 문서 열기 → 빈 세션 보존 ──
  await clickEditArea(page);
  await typeText(page, 'KEEPME');
  await page.evaluate(() => new Promise((r) => setTimeout(r, 1200))); // 디바운스 flush
  // 다른 문서를 fileId 없이 로컬 열기(postMessage loadFile, fileId 미지정 → SSR 업로드 발급)
  const openResult = await page.evaluate(async () => {
    const resp = await fetch('/samples/re-align-center-hancom.hwp');
    const data = Array.from(new Uint8Array(await resp.arrayBuffer()));
    return await new Promise((resolve) => {
      const onMsg = (e) => {
        if (e.data?.type === 'rhwp-response' && e.data.id === 77) {
          window.removeEventListener('message', onMsg);
          resolve(e.data);
        }
      };
      window.addEventListener('message', onMsg);
      window.postMessage({ type: 'rhwp-request', id: 77, method: 'loadFile', params: { data, fileName: 'opened.hwp', skipUnsavedGuard: true } }, '*');
    });
  });
  assert(!openResult.error, `열기 응답: ${openResult.error || 'ok'}`);
  await page.waitForFunction((prev) => window.__ssr?.fileId && window.__ssr.fileId !== prev, { timeout: 15000 }, blankFid);
  const openedFid = await page.evaluate(() => window.__ssr.fileId);
  console.log(`  [C] 열기 후 fileId=${openedFid} (blank=${blankFid})`);
  assert(openedFid && openedFid !== blankFid, '열기 시 새 fileId로 전환');
  assert((await page.evaluate(() => window.__ssr.isBlank)) === false, '전환 후 isBlank=false');
  // 편집했던 빈 문서 세션은 서버에 편집("KEEPME") 보존
  const keptText = await irText(blankFid);
  console.log(`  [C] 편집했던 빈문서 서버 상태: ${keptText ? keptText.slice(0, 20) : '(없음)'}`);
  assert(keptText != null && keptText.includes('KEEPME'), '편집한 빈문서 세션은 서버에 보존됨');

  // ── 시나리오 B: 빈 문서 미편집 → 다른 문서 열기 → 빈 세션 전환(닫힘) ──
  const page2 = await browser.newPage();
  await page2.goto(ENTRY, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page2.waitForFunction(() => !!window.__wasm, { timeout: 15000 });
  await page2.waitForFunction(() => window.__ssr?.fileId != null, { timeout: 15000 });
  const blank2 = await page2.evaluate(() => window.__ssr.fileId);
  // 편집 없이 바로 다른 문서 열기
  await page2.evaluate(async () => {
    const resp = await fetch('/samples/re-align-center-hancom.hwp');
    const data = Array.from(new Uint8Array(await resp.arrayBuffer()));
    await new Promise((resolve) => {
      const onMsg = (e) => { if (e.data?.type === 'rhwp-response' && e.data.id === 88) { window.removeEventListener('message', onMsg); resolve(); } };
      window.addEventListener('message', onMsg);
      window.postMessage({ type: 'rhwp-request', id: 88, method: 'loadFile', params: { data, fileName: 'opened2.hwp', skipUnsavedGuard: true } }, '*');
    });
  });
  await page2.waitForFunction((prev) => window.__ssr?.fileId && window.__ssr.fileId !== prev, { timeout: 15000 }, blank2);
  const opened2 = await page2.evaluate(() => window.__ssr.fileId);
  console.log(`  [B] 미편집 열기: blank=${blank2} → opened=${opened2}`);
  assert(opened2 && opened2 !== blank2, '미편집 후 열기 시 새 fileId로 전환');
}, { skipLoadApp: true });
