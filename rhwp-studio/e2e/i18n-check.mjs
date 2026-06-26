/**
 * E2E: i18n 자체 자체 자체 lang 자체 자체 자체 자체 박힌 자체 자체 자체 검증.
 *
 * 박는 자체:
 *   1. ?sysLang=ko 박은 자체 자체 자체 메뉴바 "파일" 박힌 자체
 *   2. ?sysLang=en 박은 자체 자체 자체 메뉴바 "File" 박힌 자체
 *   3. ?sysLang=ja 박은 자체 자체 자체 메뉴바 "ファイル" 박힌 자체
 *   4. postMessage 박은 자체 실시간 교체 자체
 *
 * 사용 방법:
 *   npx vite --host 0.0.0.0 --port 7700 &
 *   node e2e/i18n-check.mjs
 */
import puppeteer from 'puppeteer-core';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const CHROME_PATH = process.env.CHROME_PATH
  || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const VITE_URL = process.env.VITE_URL || 'http://localhost:7700';
const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../../output/e2e/i18n-check');

const EXPECT = {
  ko: { fileMenu: '파일', editMenu: '편집', insertMode: '삽입', tbCut: '오려 두기', tbCharShape: '글자 모양', mdFileSave: '저장', mdEditUndo: '되돌리기' },
  en: { fileMenu: 'File', editMenu: 'Edit', insertMode: 'Insert', tbCut: 'Cut', tbCharShape: 'Font', mdFileSave: 'Save', mdEditUndo: 'Undo' },
  ja: { fileMenu: 'ファイル', editMenu: '編集', insertMode: '挿入', tbCut: '切り取り', tbCharShape: 'フォント', mdFileSave: '保存', mdEditUndo: '元に戻す' },
};

async function getMenuTexts(page) {
  return page.evaluate(() => {
    const menu = (key) => {
      const el = document.querySelector(`[data-i18n="${key}"]`);
      return el ? el.textContent.trim() : null;
    };
    return {
      fileMenu: menu('menu.file.label'),
      editMenu: menu('menu.edit.label'),
      insertMode: menu('statusbar.insert_mode'),
      // m700-13 자체 자체 도구상자·드롭다운 자체 자체
      tbCut: menu('toolbar.cut'),
      tbCharShape: menu('toolbar.char_shape'),
      mdFileSave: menu('menu.file.save'),
      mdEditUndo: menu('menu.edit.undo'),
    };
  });
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: CHROME_PATH,
    headless: 'new',
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  const results = [];
  let passCount = 0;
  let failCount = 0;

  for (const lang of ['ko', 'en', 'ja']) {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 800 });
    const url = `${VITE_URL}/hwp/?sysLang=${lang}`;
    console.log(`\n[${lang}] ${url} 로드 자체 자체 ...`);
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
    // 자체 자체 자체 자체 자체 applyStaticTexts 박힐 자체 자체 자체 자체 잠시
    await new Promise((r) => setTimeout(r, 2000));

    const actual = await getMenuTexts(page);
    const expected = EXPECT[lang];
    const pass =
      actual.fileMenu === expected.fileMenu
      && actual.editMenu === expected.editMenu
      && actual.insertMode === expected.insertMode
      && actual.tbCut === expected.tbCut
      && actual.tbCharShape === expected.tbCharShape
      && actual.mdFileSave === expected.mdFileSave
      && actual.mdEditUndo === expected.mdEditUndo;
    if (pass) {
      passCount++;
      console.log(`  ✓ ${lang}: file=${actual.fileMenu} edit=${actual.editMenu} mode=${actual.insertMode}`);
    } else {
      failCount++;
      console.log(`  ✗ ${lang}:`);
      console.log(`    expected: ${JSON.stringify(expected)}`);
      console.log(`    actual:   ${JSON.stringify(actual)}`);
    }
    await page.screenshot({ path: `${OUT_DIR}/sysLang-${lang}.png`, fullPage: false });
    results.push({ lang, expected, actual, pass });
    await page.close();
  }

  // postMessage 실시간 교체 자체
  console.log(`\n[postMessage] ko → en 자체 자체 자체 자체`);
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 800 });
  await page.goto(`${VITE_URL}/hwp/?sysLang=ko`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await new Promise((r) => setTimeout(r, 2000));
  const before = await getMenuTexts(page);
  console.log(`  before: ${JSON.stringify(before)}`);
  await page.evaluate(() => {
    window.postMessage({ type: 'rhwp:set-locale', sysLang: 'en' }, '*');
  });
  await new Promise((r) => setTimeout(r, 500));
  const after = await getMenuTexts(page);
  console.log(`  after:  ${JSON.stringify(after)}`);
  const liveSwapPass =
    before.fileMenu === '파일'
    && after.fileMenu === 'File'
    && after.editMenu === 'Edit';
  if (liveSwapPass) {
    passCount++;
    console.log(`  ✓ postMessage 실시간 교체 통과`);
  } else {
    failCount++;
    console.log(`  ✗ postMessage 실시간 교체 실패`);
  }
  await page.screenshot({ path: `${OUT_DIR}/postMessage-after.png`, fullPage: false });
  results.push({ name: 'postMessage', before, after, pass: liveSwapPass });

  await browser.close();

  console.log(`\n=== 결과: ${passCount} 통과 / ${failCount} 실패 ===`);
  await writeFile(`${OUT_DIR}/results.json`, JSON.stringify(results, null, 2));
  console.log(`자체 자체: ${OUT_DIR}/`);
  if (failCount > 0) process.exit(1);
}

main().catch((e) => {
  console.error('error:', e);
  process.exit(1);
});
