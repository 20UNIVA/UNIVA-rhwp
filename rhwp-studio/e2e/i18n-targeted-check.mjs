/**
 * 사용자 지적 3 자리 박힌 자체 직접 검증:
 *   1. Tools > Preferences 다이얼로그 본문 — 한국어 가이드
 *   2. 우클릭 컨텍스트 메뉴 — 글자/문단 모양 등
 *   3. 도구상자 스타일 콤보 — 바탕글
 */
import puppeteer from 'puppeteer-core';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const CHROME_PATH = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const VITE_URL = 'http://localhost:7700';
const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../../output/e2e/i18n-targeted');

async function getKoreanIn(page, selector) {
  return page.evaluate((sel) => {
    const KOREAN = /[가-힣]/;
    const root = document.querySelector(sel);
    if (!root) return { error: 'selector not found', selector: sel };
    const result = [];
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) {
      const text = node.textContent.trim();
      if (text && KOREAN.test(text)) result.push(text);
    }
    return { found: result };
  }, selector);
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: CHROME_PATH,
    headless: 'new',
    args: ['--no-sandbox'],
  });

  for (const lang of ['en', 'ja']) {
    const page = await browser.newPage();
    await page.setViewport({ width: 1600, height: 1000 });
    await page.goto(`${VITE_URL}/hwp/?sysLang=${lang}`, { waitUntil: 'domcontentloaded' });
    await new Promise((r) => setTimeout(r, 2500));

    console.log(`\n=== [${lang}] ===`);

    // 1. 스타일 콤보 — 바탕글 박혔나
    // 문서 자체 박은 후 박힌다. 새 문서 박는다.
    await page.evaluate(() => {
      const newDoc = document.querySelector('[data-cmd="file:new-doc"]');
      if (newDoc) newDoc.click();
    });
    await new Promise((r) => setTimeout(r, 2000));

    const styleCombo = await page.evaluate(() => {
      const sel = document.querySelector('#style-name');
      if (!sel) return { error: 'no #style-name' };
      const opts = Array.from(sel.querySelectorAll('option')).map((o) => o.textContent.trim());
      return { options: opts };
    });
    console.log(`  스타일 콤보: ${JSON.stringify(styleCombo)}`);

    // 2. Tools > Preferences 박기
    await page.evaluate(() => {
      const tools = document.querySelector('[data-menu="tool"]');
      if (tools) tools.click();
    });
    await new Promise((r) => setTimeout(r, 500));
    await page.evaluate(() => {
      // Preferences 박은 자료
      const items = document.querySelectorAll('.md-item');
      items.forEach((i) => {
        const k = i.dataset?.i18n;
        if (k === 'prefs.dialog_title' || k?.includes('options')) i.click();
      });
    });
    await new Promise((r) => setTimeout(r, 1000));
    const prefsDialog = await getKoreanIn(page, '.dialog-overlay, .opt-dialog, [class*="options"]');
    console.log(`  Preferences 다이얼로그 한국어: ${JSON.stringify(prefsDialog)}`);
    await page.screenshot({ path: `${OUT_DIR}/${lang}-prefs.png` });

    // 다이얼로그 닫음
    await page.keyboard.press('Escape');
    await new Promise((r) => setTimeout(r, 300));

    // 3. 본문 박은 후 우클릭
    const editArea = await page.$('#scroll-container');
    if (editArea) {
      const box = await editArea.boundingBox();
      if (box) {
        await page.mouse.click(box.x + box.width / 2, box.y + 100);
        await new Promise((r) => setTimeout(r, 300));
        await page.keyboard.type('test');
        await new Promise((r) => setTimeout(r, 300));
        await page.mouse.click(box.x + box.width / 2, box.y + 100, { button: 'right' });
        await new Promise((r) => setTimeout(r, 500));
      }
    }
    const ctxMenu = await getKoreanIn(page, '.context-menu, [class*="context"]');
    console.log(`  우클릭 메뉴 한국어: ${JSON.stringify(ctxMenu)}`);
    await page.screenshot({ path: `${OUT_DIR}/${lang}-ctx.png` });

    await page.close();
  }
  await browser.close();
}

main().catch((e) => { console.error(e); process.exit(1); });
