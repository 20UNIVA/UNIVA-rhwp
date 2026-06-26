/**
 * E2E: 동적 자료(드롭다운·다이얼로그·우클릭 메뉴) 열어 잔존 한국어 박는다.
 *
 * 박은 자료:
 *   1. 메뉴바 드롭다운 *모두 열기*
 *   2. Tools > Preferences 다이얼로그 열기
 *   3. 도구상자 스타일 콤보 자료 박기
 *   4. 새 문서 박은 후 우클릭 메뉴 열기
 */
import puppeteer from 'puppeteer-core';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const CHROME_PATH = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const VITE_URL = 'http://localhost:7700';
const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../../output/e2e/i18n-deep');

async function scanKorean(page, label) {
  return page.evaluate((label) => {
    const KOREAN = /[가-힣]/;
    const results = [];
    const seen = new Set();

    const isVisible = (el) => {
      const style = window.getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden') return false;
      const rect = el.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    };

    const getPath = (el) => {
      const parts = [];
      let cur = el;
      while (cur && cur !== document.body && parts.length < 4) {
        const id = cur.id ? '#' + cur.id : '';
        const cls = (cur.className || '').toString().split(' ').filter(Boolean).slice(0, 2).join('.');
        const tag = cur.tagName.toLowerCase();
        parts.unshift(id || (cls ? `${tag}.${cls}` : tag));
        cur = cur.parentElement;
      }
      return parts.join(' > ');
    };

    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
      acceptNode: (node) => {
        const text = node.textContent.trim();
        if (!text || !KOREAN.test(text)) return NodeFilter.FILTER_REJECT;
        const parent = node.parentElement;
        if (!parent || ['SCRIPT', 'STYLE'].includes(parent.tagName)) return NodeFilter.FILTER_REJECT;
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
      results.push({ scenario: label, text, path });
    }

    const attrs = ['title', 'placeholder', 'aria-label', 'value', 'alt'];
    document.querySelectorAll('*').forEach((el) => {
      if (!isVisible(el)) return;
      for (const attr of attrs) {
        const v = el.getAttribute(attr);
        if (v && KOREAN.test(v)) {
          const key = `attr:${attr}::${getPath(el)}::${v}`;
          if (seen.has(key)) continue;
          else { seen.add(key); results.push({ scenario: label, text: v, path: getPath(el), attr }); }
        }
      }
    });

    // select 안 option 자료
    document.querySelectorAll('select').forEach((sel) => {
      if (!isVisible(sel)) return;
      sel.querySelectorAll('option').forEach((opt) => {
        const text = opt.textContent.trim();
        if (text && KOREAN.test(text)) {
          const key = `option::${getPath(sel)}::${text}`;
          if (seen.has(key)) return;
          seen.add(key);
          results.push({ scenario: label, text, path: getPath(sel) + ' > option' });
        }
      });
    });

    return results;
  }, label);
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

    const collected = [];

    // 시나리오 1: 초기 화면
    collected.push(...await scanKorean(page, 'initial'));

    // 시나리오 2: 모든 드롭다운 열기 + 스타일 콤보
    await page.evaluate(() => {
      document.querySelectorAll('.menu-item').forEach((m) => {
        m.classList.add('open');
        const dropdown = m.querySelector('.menu-dropdown');
        if (dropdown) dropdown.style.display = 'block';
      });
      // 서브메뉴도 열기
      document.querySelectorAll('.md-sub').forEach((sub) => {
        const submenu = sub.querySelector('.md-submenu');
        if (submenu) submenu.style.display = 'block';
      });
    });
    await new Promise((r) => setTimeout(r, 300));
    collected.push(...await scanKorean(page, 'menus_open'));

    // 시나리오 3: 새 문서 박은 후 우클릭 메뉴 열기
    try {
      // 새 문서 메뉴 박는다 (file:new-doc)
      await page.evaluate(() => {
        const newDoc = document.querySelector('[data-cmd="file:new-doc"]');
        if (newDoc && !newDoc.classList.contains('disabled')) newDoc.click();
      });
      await new Promise((r) => setTimeout(r, 1500));
      // 편집 영역 박은 후 우클릭
      const editArea = await page.$('#scroll-container');
      if (editArea) {
        const box = await editArea.boundingBox();
        if (box) {
          await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
          await page.keyboard.type('테스트');
          await page.keyboard.down('Shift');
          for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowLeft');
          await page.keyboard.up('Shift');
          await page.mouse.click(box.x + box.width / 2, box.y + 50, { button: 'right' });
          await new Promise((r) => setTimeout(r, 500));
          collected.push(...await scanKorean(page, 'contextmenu'));
        }
      }
    } catch (e) {
      console.log(`  [${lang}] 우클릭 시나리오 실패: ${e.message}`);
    }

    all[lang] = collected;
    const unique = new Set(collected.map((r) => r.text));
    console.log(`\n[${lang}] 잔존 한국어: ${unique.size} 자리 (총 ${collected.length} 매칭)`);
    Array.from(unique).slice(0, 40).forEach((t) => console.log(`  "${t}"`));

    await page.screenshot({ path: `${OUT_DIR}/${lang}-final.png`, fullPage: false });
    await page.close();
  }

  await writeFile(`${OUT_DIR}/deep-scan.json`, JSON.stringify(all, null, 2));
  await browser.close();
  console.log(`\n자료: ${OUT_DIR}/deep-scan.json`);
}

main().catch((e) => { console.error(e); process.exit(1); });
