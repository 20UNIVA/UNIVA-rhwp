/**
 * E2E: WS 양방향 — 서버→클라 push와 클라→서버 미러링 둘 다 검증.
 *
 * 시나리오:
 *   1) 빈 hwpx로 POST /sessions → fileId
 *   2) Puppeteer로 ?fileId 진입 → WS 연결됨
 *   3) curl로 POST /workbench (insert_text "FROM-LLM") → 서버 push → DOM에 "FROM-LLM"
 *   4) page.evaluate로 InputHandler 시뮬 — 직접 키 입력은 어려우니
 *      *WS 직접 호출* 방식: page에서 new WebSocket(...) 만들어 ClientMessage::Ops 발사,
 *      그 결과가 sqlite에 반영되었는지 GET /ir로 확인
 */

import puppeteer from 'puppeteer-core';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SERVER = 'http://127.0.0.1:7710';
const WS_BASE = 'ws://127.0.0.1:7710';
const BLANK_HWPX = resolve(import.meta.dirname ?? '.', '..', '..', 'samples', 'hwpx', 'blank_hwpx.hwpx');
const CHROMIUM = process.env.CHROMIUM ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

async function http(method, path, body) {
  const r = await fetch(`${SERVER}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await r.text();
  if (!r.ok) throw new Error(`HTTP ${r.status} ${path}: ${text}`);
  return text ? JSON.parse(text) : {};
}

async function main() {
  // 1) 세션
  const fileId = `e2e-ws-${Date.now()}`;
  const bytes = readFileSync(BLANK_HWPX);
  await http('POST', '/sessions', {
    fileId,
    format: 'hwpx',
    fileBase64: Buffer.from(bytes).toString('base64'),
  });
  console.log('세션:', fileId);

  // 2) 브라우저
  const browser = await puppeteer.launch({
    executablePath: CHROMIUM,
    headless: 'new',
    args: ['--no-sandbox'],
  });
  const page = await browser.newPage();
  page.on('console', (msg) => console.log('[browser]', msg.text()));
  await page.goto(`${SERVER}/?fileId=${fileId}`, { waitUntil: 'networkidle0', timeout: 30000 });

  // WS 연결 대기 — 콘솔에 "WS open" 로그가 없으면 잠시 대기
  await new Promise((r) => setTimeout(r, 1000));

  // 3) 서버→클라: workbench로 발사
  await http('POST', `/sessions/${fileId}/workbench`, {
    action: 'insert_text',
    payload: { section: 0, para: 0, offset: 0, text: 'FROM-LLM' },
  });

  let appeared = false;
  for (let i = 0; i < 50; i++) {
    if (await page.evaluate(() => document.body.innerText.includes('FROM-LLM'))) {
      appeared = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  if (!appeared) {
    await browser.close();
    throw new Error('서버→클라 push가 5초 안에 DOM에 반영 안 됨');
  }
  console.log('OK 1: 서버→클라 push로 "FROM-LLM" 반영');

  // 4) 클라→서버: page 안에서 WS로 ops 발사
  await page.evaluate(
    async (url) => {
      const ws = new WebSocket(url);
      await new Promise((resolve, reject) => {
        ws.addEventListener('open', () => resolve());
        ws.addEventListener('error', (e) => reject(e));
        setTimeout(() => reject(new Error('WS open timeout')), 5000);
      });
      ws.send(
        JSON.stringify({
          kind: 'ops',
          ops: [
            {
              op: 'insert_text',
              section: 0,
              para: 0,
              offset: 0,
              text: 'FROM-CLIENT',
            },
          ],
        })
      );
      await new Promise((r) => setTimeout(r, 500));
      ws.close();
    },
    `${WS_BASE}/sessions/${fileId}/ws`
  );

  // 5) 서버 IR 확인 — FROM-CLIENT가 sqlite에 영속됐는지
  const ir = await http('GET', `/sessions/${fileId}/ir?page=0`);
  const irText = JSON.stringify(ir);
  if (!irText.includes('FROM-CLIENT')) {
    await browser.close();
    throw new Error(`클라→서버 ops가 서버 IR에 반영 안 됨. IR=${irText.slice(0, 500)}`);
  }
  console.log('OK 2: 클라→서버 ops로 "FROM-CLIENT"가 서버 IR에 영속');

  await browser.close();
  console.log('\n=== 양방향 WS bridge 검증 통과 ===');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
