/**
 * E2E: en/ja 화면에서 남은 한국어 텍스트 추출.
 *
 * 화면 안 모든 가시 요소를 순회해 한국어 [가-힣] 박힌 텍스트를 박는다.
 * 코드 안 어디서 박혔는지 추적할 수 있도록 selector 도 같이 박는다.
 */
import puppeteer from 'puppeteer-core';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const CHROME_PATH = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const VITE_URL = 'http://localhost:7700';
const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../../output/e2e/i18n-residual');

async function scanResidualKorean(page, lang) {
  return page.evaluate(() => {
    const KOREAN = /[가-힣]/;
    const results = [];
    const seen = new Set();

    function isVisible(el) {
      const style = window.getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
      const rect = el.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    }

    function getSelector(el) {
      if (el.id) return '#' + el.id;
      const cls = (el.className || '').toString().split(' ').filter(Boolean).slice(0, 2).join('.');
      const tag = el.tagName.toLowerCase();
      return cls ? `${tag}.${cls}` : tag;
    }

    function getPath(el) {
      const parts = [];
      let cur = el;
      while (cur && cur !== document.body && parts.length < 5) {
        parts.unshift(getSelector(cur));
        cur = cur.parentElement;
      }
      return parts.join(' > ');
    }

    // 모든 텍스트 노드 박는다
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
      acceptNode: (node) => {
        const text = node.textContent.trim();
        if (!text || !KOREAN.test(text)) return NodeFilter.FILTER_REJECT;
        const parent = node.parentElement;
        if (!parent) return NodeFilter.FILTER_REJECT;
        // script/style 안 박힌 텍스트 제외
        if (['SCRIPT', 'STYLE', 'NOSCRIPT'].includes(parent.tagName)) return NodeFilter.FILTER_REJECT;
        if (!isVisible(parent)) return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      },
    });

    let node;
    while ((node = walker.nextNode())) {
      const text = node.textContent.trim();
      const parent = node.parentElement;
      const path = getPath(parent);
      const key = `${path}::${text}`;
      if (seen.has(key)) continue;
      seen.add(key);
      results.push({ text, path });
    }

    // 속성 자체 한국어 박힌 자리 (title, placeholder, aria-label, value)
    const attrs = ['title', 'placeholder', 'aria-label', 'value', 'alt'];
    document.querySelectorAll('*').forEach((el) => {
      if (!isVisible(el)) return;
      for (const attr of attrs) {
        const v = el.getAttribute(attr);
        if (v && KOREAN.test(v)) {
          const path = getPath(el);
          const key = `attr:${attr}::${path}::${v}`;
          if (seen.has(key)) continue;
          seen.add(key);
          results.push({ text: v, path, attr });
        }
      }
    });

    return results;
  });
}

async function expandMenus(page) {
  // 메뉴바 드롭다운 박은 후 한국어 확인
  await page.evaluate(() => {
    document.querySelectorAll('.menu-item').forEach((m) => {
      m.classList.add('open');
      const dropdown = m.querySelector('.menu-dropdown');
      if (dropdown) dropdown.style.display = 'block';
    });
  });
  await new Promise((r) => setTimeout(r, 300));
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: CHROME_PATH,
    headless: 'new',
    args: ['--no-sandbox'],
  });

  const all = {};
  for (const lang of ['en', 'ja']) {
    const page = await browser.newPage();
    await page.setViewport({ width: 1600, height: 1000 });
    await page.goto(`${VITE_URL}/hwp/?sysLang=${lang}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await new Promise((r) => setTimeout(r, 2500));

    // 기본 화면 박은 한국어
    const baseline = await scanResidualKorean(page, lang);

    // 메뉴 드롭다운 박은 후 다시 박음
    await expandMenus(page);
    const withMenus = await scanResidualKorean(page, lang);

    all[lang] = withMenus;
    console.log(`\n[${lang}] 남은 한국어 자리: ${withMenus.length} 개`);
    withMenus.slice(0, 30).forEach((r) => {
      const attrStr = r.attr ? ` [${r.attr}]` : '';
      console.log(`  ${r.path}${attrStr}: "${r.text}"`);
    });
    if (withMenus.length > 30) console.log(`  ... ${withMenus.length - 30} 더`);

    await page.screenshot({ path: `${OUT_DIR}/${lang}-fullscreen.png`, fullPage: false });
    await page.close();
  }

  await writeFile(`${OUT_DIR}/residual.json`, JSON.stringify(all, null, 2));
  await browser.close();
  console.log(`\n자료 박힘: ${OUT_DIR}/residual.json`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
