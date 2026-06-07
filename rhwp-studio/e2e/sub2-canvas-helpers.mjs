/**
 * Sub-2 Canvas 시각 검증 헬퍼.
 *
 * 목적 — Sub-1 의 Critical fix #2 류 사고 (`eventBus.emit('document-changed')`
 * 누락 → 서버 IR 은 바뀌었으나 Canvas 안 바뀜) 를 e2e 가 사전에 잡아낸다.
 * 기존 14 e2e 는 서버 IR + WS broadcast 만 검증 — Canvas pixel 자체는 검증 못 함.
 *
 * 의존 — puppeteer-core (프로젝트 기존 devDependencies) + pngjs + pixelmatch.
 * 프로젝트 helpers.mjs 와 같은 방식 — 시스템 Chrome 또는 PUPPETEER_EXECUTABLE_PATH.
 *
 * 서버 측 — sub2-server.sh 가 RHWP_STUDIO_DIR 를 dist 로 지정 → 같은 포트 (7710) 에서
 * studio 정적 자산 서빙. Puppeteer 가 `http://127.0.0.1:7710/hwp/?fileId=...` 로 진입.
 */

import puppeteer from 'puppeteer-core';
import { PNG } from 'pngjs';
import pixelmatch from 'pixelmatch';
import { writeFileSync } from 'node:fs';
import { Buffer } from 'node:buffer';

/** 시스템 Chrome 경로 — macOS / Linux 환경 모두 환경변수 우선. */
const CHROME_PATH =
  process.env.CHROME_PATH
  || process.env.PUPPETEER_EXECUTABLE_PATH
  || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

/** Studio 가 7710 포트의 `/hwp` 아래 같은 origin 으로 서빙된다는 전제. */
export const STUDIO_BASE = process.env.STUDIO_BASE || 'http://127.0.0.1:7710/hwp';

/**
 * Puppeteer 브라우저 가동 (headless).
 * 캐시·SW 관련 문제 회피 위해 매 호출마다 사용자 데이터 디렉터리 분리.
 */
export async function launchBrowser() {
  return await puppeteer.launch({
    headless: true,
    executablePath: CHROME_PATH,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-gpu',
      '--disable-web-security',
      // PWA Service Worker 가 dist 에 동봉되어 있음 — workbox precache 가 옛 번들을
      // 잡고 있을 수 있어 매 e2e 마다 SW 캐시·등록 모두 비활성화.
      '--disable-features=ServiceWorker,InterestCohort',
    ],
  });
}

/**
 * fileId 로 세션 페이지 열기.
 * 1) `?fileId=<id>` 진입 → studio 가 SSR 모드 활성, /sessions/{id}/export 로 복원.
 * 2) wasm 초기화 + Canvas DOM 생성 + 첫 페이지 렌더 완료까지 대기.
 *
 * 반환: page (Puppeteer Page 인스턴스). browser.close() 가 청소 책임.
 */
export async function openSession(browser, fileId, timeoutMs = 30000) {
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 900 });

  // SSR 미러링 연결 완료 시점을 콘솔 메시지로 감지한다.
  // dist 빌드는 window.__ssr 같은 전역을 노출 안 함 (DEV 전용 분기) — 콘솔 message 만이 신뢰 지표.
  let mirrorReady = false;
  page.on('console', (msg) => {
    const t = msg.type();
    const text = msg.text();
    if (t === 'error' || t === 'warning') console.error(`[browser ${t}]`, text);
    else if (process.env.SUB2_CANVAS_VERBOSE) console.log(`[browser ${t}]`, text);
    if (text.includes('[SSR] 세션 미러링 연결됨')) {
      mirrorReady = true;
    }
  });
  page.on('pageerror', (err) => {
    console.error('[browser pageerror]', err.message);
  });
  page.on('requestfailed', (req) => {
    console.error('[browser requestfailed]', req.url(), req.failure()?.errorText);
  });
  // ssrBase 미지정 시 SessionClient 는 baseUrlWs="" 로 잘못된 ws:/// URL 생성 → ws open silent fail.
  // STUDIO_BASE (`/hwp` 포함) 그대로 ssrBase 로 전달 — fetch 와 ws 가 prefix 포함 origin 사용.
  const url = `${STUDIO_BASE}/?fileId=${encodeURIComponent(fileId)}&ssrBase=${encodeURIComponent(STUDIO_BASE)}`;

  // 디버깅 (SUB2_CANVAS_VERBOSE=1 일 때) — WebSocket 트래픽을 console 로 가시화.
  if (process.env.SUB2_CANVAS_VERBOSE) {
    await page.evaluateOnNewDocument(() => {
      const OriginalWS = window.WebSocket;
      function PatchedWS(url, protocols) {
        const ws = new OriginalWS(url, protocols);
        console.log('[WS-DEBUG] new WebSocket', url);
        ws.addEventListener('open', () => console.log('[WS-DEBUG] open', url));
        ws.addEventListener('close', (e) => console.log('[WS-DEBUG] close', url, e.code));
        ws.addEventListener('error', () => console.log('[WS-DEBUG] error', url));
        ws.addEventListener('message', (e) => {
          const d = typeof e.data === 'string' ? e.data : '[binary]';
          console.log('[WS-DEBUG] message', d.slice(0, 200));
        });
        return ws;
      }
      PatchedWS.prototype = OriginalWS.prototype;
      PatchedWS.CONNECTING = OriginalWS.CONNECTING;
      PatchedWS.OPEN = OriginalWS.OPEN;
      PatchedWS.CLOSING = OriginalWS.CLOSING;
      PatchedWS.CLOSED = OriginalWS.CLOSED;
      window.WebSocket = PatchedWS;
    });
  }
  await page.goto(url, { waitUntil: 'networkidle2', timeout: timeoutMs });
  // Canvas 렌더 완료 대기.
  await page.waitForFunction(
    () => {
      const c = document.querySelector('#scroll-container canvas');
      return c && c.width > 0 && c.height > 0;
    },
    { timeout: timeoutMs },
  );
  // SSR 미러링 (WS) 연결 완료 대기 — broadcast 수신 가능해야 함.
  const mirrorDeadline = Date.now() + timeoutMs;
  while (!mirrorReady && Date.now() < mirrorDeadline) {
    await new Promise((r) => setTimeout(r, 100));
  }
  if (!mirrorReady) throw new Error('SSR 미러링 연결 timeout — broadcast 수신 불가 상태');
  // 추가 안정화 — WS open 후 broadcast 채널 등록 여유.
  await new Promise((r) => setTimeout(r, 500));
  return page;
}

/**
 * 편집 영역 캔버스를 PNG buffer 로 캡처.
 * `#scroll-container canvas` 셀렉터 — helpers.mjs 와 동일.
 */
export async function snapCanvas(page) {
  const canvas = await page.$('#scroll-container canvas');
  if (!canvas) throw new Error('canvas 요소 없음 (#scroll-container canvas)');
  const raw = await canvas.screenshot({ type: 'png' });
  // puppeteer-core 25.x 는 Uint8Array 반환 — pngjs 는 Node Buffer 가 필요.
  return Buffer.isBuffer(raw) ? raw : Buffer.from(raw);
}

/**
 * 두 PNG buffer 의 픽셀 차이 계산.
 * threshold 0.1 — pixelmatch 기본값 (0~1 사이, 낮을수록 민감).
 *
 * 반환:
 *   - diff   : 차이가 난 픽셀 수
 *   - total  : 전체 픽셀 수
 *   - ratio  : diff / total (0~1)
 *   - note   : size mismatch 등 비교 불가 사유 (정상이면 undefined)
 */
export function diffPixels(beforeBuf, afterBuf) {
  const before = PNG.sync.read(beforeBuf);
  const after = PNG.sync.read(afterBuf);
  if (before.width !== after.width || before.height !== after.height) {
    return {
      diff: -1,
      total: 0,
      ratio: -1,
      note: `size mismatch ${before.width}x${before.height} vs ${after.width}x${after.height}`,
    };
  }
  const total = before.width * before.height;
  const diffOut = new PNG({ width: before.width, height: before.height });
  // includeAA: true 로 글자 anti-aliasing 픽셀까지 차이로 카운트한다.
  // threshold 0.1 — pixelmatch 권장값 (0~1, 낮을수록 민감).
  const diff = pixelmatch(
    before.data,
    after.data,
    diffOut.data,
    before.width,
    before.height,
    { threshold: 0.1, includeAA: true },
  );
  return { diff, total, ratio: diff / total };
}

/**
 * 디버깅용 — before·after·diff 세 PNG 를 디스크에 기록한다.
 * 실패 시 산출물 분석에 사용.
 */
export function saveDiffArtifacts(beforeBuf, afterBuf, outDir, baseName) {
  writeFileSync(`${outDir}/${baseName}-before.png`, beforeBuf);
  writeFileSync(`${outDir}/${baseName}-after.png`, afterBuf);
  const before = PNG.sync.read(beforeBuf);
  const after = PNG.sync.read(afterBuf);
  if (before.width !== after.width || before.height !== after.height) return;
  const diffOut = new PNG({ width: before.width, height: before.height });
  pixelmatch(
    before.data,
    after.data,
    diffOut.data,
    before.width,
    before.height,
    { threshold: 0.1, includeAA: true },
  );
  writeFileSync(`${outDir}/${baseName}-diff.png`, PNG.sync.write(diffOut));
}
