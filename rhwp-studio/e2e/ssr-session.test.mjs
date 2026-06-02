/**
 * E2E 테스트: SSR 세션 미러링 (Task #999)
 *
 * 시나리오:
 *   1. rhwp-server 기동 (별도 포트)
 *   2. studio를 ?fileId=&ssrBase= 로 로드
 *   3. postMessage loadFile({fileId}) → 서버 세션 생성
 *   4. 편집(typeText) → 디바운스 배치로 서버에 op 미러링
 *   5. 서버 GET /ir 조회 → 입력 텍스트가 서버 세션에 반영되었는지 확인
 *   6. (닫힘 시뮬레이션) 페이지 종료 후에도 서버 상태 유지 확인
 *
 * 사전 조건: vite dev server(VITE_URL, 기본 :7700)가 떠 있어야 함.
 */
import { spawn } from 'node:child_process';
import { existsSync, unlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { runTest, assert, clickEditArea, typeText } from './helpers.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SERVER_BIN = resolve(__dirname, '../../server/target/debug/rhwp-server');
const SERVER_PORT = 7720;
const SERVER_BASE = `http://127.0.0.1:${SERVER_PORT}`;
const VITE_URL = process.env.VITE_URL || 'http://localhost:7700';
const FILE_ID = 'SSRE2E';
const DB = '/tmp/rhwp-e2e-session.db';
const MARKER = 'E2EMIRROR';

// 서버는 기본적으로 외부에서 기동(RHWP_SERVER_ADDR=127.0.0.1:7720)된 것을 사용한다.
// E2E_SPAWN_SERVER=1 이면 테스트가 직접 spawn한다.
let server = null;
const cleanup = () => { try { server?.kill(); } catch {} };
if (process.env.E2E_SPAWN_SERVER) {
  if (existsSync(DB)) unlinkSync(DB);
  server = spawn(SERVER_BIN, [], {
    env: { ...process.env, RHWP_SERVER_ADDR: `127.0.0.1:${SERVER_PORT}`, RHWP_SERVER_DB: DB },
    stdio: 'ignore',
  });
  process.on('exit', cleanup);
}

// 서버 health 대기
async function waitServer() {
  for (let i = 0; i < 30; i++) {
    try {
      const r = await fetch(`${SERVER_BASE}/health`);
      if (r.ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error('rhwp-server 기동 실패');
}

await waitServer();

runTest('SSR 세션 미러링 E2E', async ({ page, browser }) => {
  // 2. studio 로드 (fileId + ssrBase query)
  const url = `${VITE_URL}/?fileId=${FILE_ID}&ssrBase=${encodeURIComponent(SERVER_BASE)}`;
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 30000 });
  await page.waitForFunction(() => !!window.__wasm, { timeout: 15000 });
  await page.evaluate(() => new Promise((r) => setTimeout(r, 500)));

  // 3. postMessage loadFile({fileId}) → loadBytes → connectSsrSession
  const loadRes = await page.evaluate(async ({ fileId }) => {
    const resp = await fetch('/samples/re-align-center-hancom.hwp');
    if (!resp.ok) return { error: `fetch HTTP ${resp.status}` };
    const data = Array.from(new Uint8Array(await resp.arrayBuffer()));
    return await new Promise((resolve) => {
      const onMsg = (e) => {
        if (e.data?.type === 'rhwp-response' && e.data.id === 99) {
          window.removeEventListener('message', onMsg);
          resolve(e.data);
        }
      };
      window.addEventListener('message', onMsg);
      window.postMessage(
        { type: 'rhwp-request', id: 99, method: 'loadFile', params: { data, fileName: 'e2e.hwp', fileId } },
        '*',
      );
    });
  }, { fileId: FILE_ID });
  assert(!loadRes.error, `loadFile 응답: ${loadRes.error || 'ok (' + JSON.stringify(loadRes.result) + ')'}`);

  // 세션 생성 완료 대기
  await page.evaluate(() => new Promise((r) => setTimeout(r, 700)));

  // 세션이 서버에 생성되었는지 확인
  const created = await fetch(`${SERVER_BASE}/sessions/${FILE_ID}/ir`).then((r) => r.ok);
  assert(created, '서버 세션 생성 확인 (GET /ir 200)');

  // 4. 편집 → 미러링
  await clickEditArea(page);
  await typeText(page, MARKER);

  // 5. 디바운스(600ms) 초과 대기 후 서버 IR 조회
  await page.evaluate(() => new Promise((r) => setTimeout(r, 1200)));
  const ir = await fetch(`${SERVER_BASE}/sessions/${FILE_ID}/ir`).then((r) => r.json());
  const allText = ir.sections.flatMap((s) => s.paragraphs.map((p) => p.text)).join('');
  console.log(`  서버 IR 텍스트(앞부분): "${allText.slice(0, 50)}"`);
  assert(allText.includes(MARKER), `편집이 서버 세션에 미러링됨 (marker "${MARKER}" 포함)`);

  // 6. 페이지 종료 후에도 서버 상태 유지 (DELETE = 연결 끊김 시뮬레이션)
  await fetch(`${SERVER_BASE}/sessions/${FILE_ID}`, { method: 'DELETE' });
  const irAfter = await fetch(`${SERVER_BASE}/sessions/${FILE_ID}/ir`).then((r) => r.json());
  const textAfter = irAfter.sections.flatMap((s) => s.paragraphs.map((p) => p.text)).join('');
  assert(textAfter.includes(MARKER), `연결 끊김(DELETE) 후 sqlite 복원으로 편집 유지`);

  // 7. 재진입 복원: 새 탭에서 파일 없이 같은 fileId URL로 진입 → 서버 상태가 화면에 떠야 함
  //    (same-URL reload는 PWA SW/history 때문에 navigation이 멈출 수 있어 새 탭 사용)
  const page2 = await browser.newPage();
  await page2.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page2.waitForFunction(() => !!window.__wasm, { timeout: 15000 });
  // restoreSsrSessionIfNeeded() 비동기 완료 대기 (export fetch + loadDocument)
  await page2.waitForFunction(
    () => (window.__wasm?.pageCount ?? 0) > 0,
    { timeout: 10000 },
  ).catch(() => {});
  const restoredPages = await page2.evaluate(() => window.__wasm?.pageCount ?? 0);
  const restoredText = await page2.evaluate(() => {
    const c = window.__wasm;
    if (!c || !c.pageCount) return '';
    let t = '';
    const n = c.getParagraphCount?.(0) ?? 0;
    for (let i = 0; i < n; i++) t += c.getTextRange?.(0, i, 0, 200) ?? '';
    return t;
  });
  console.log(`  재진입 후 pageCount=${restoredPages}, marker 포함=${restoredText.includes(MARKER)}`);
  assert(restoredPages > 0, `재진입 시 파일 없이 서버에서 문서 복원 로드 (pageCount=${restoredPages})`);
  assert(restoredText.includes(MARKER), `재진입 복원 문서에 이전 편집("${MARKER}") 포함`);
}, { skipLoadApp: true });

cleanup();
