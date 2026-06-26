/**
 * i18n 헬퍼 — rhwp-studio 의 한국어·영어·일본어 사전 접근.
 *
 * 사용 자리:
 *   t('menu.file.save')                     → "저장" (ko) | "Save" (en) | "保存" (ja)
 *   t('compare.detail_title_pair', { left, right }) → 치환된 문장
 *
 * 변수명 규약:
 *   - 경계 (URL · postMessage): sysLang
 *   - 내부 (이 모듈·코드 안): lang
 */

import { messages_ko } from './messages.ko';
import { messages_en } from './messages.en';
import { messages_ja } from './messages.ja';

export type Lang = 'ko' | 'en' | 'ja';
export type MessageKey = keyof typeof messages_ko;

const bundles: Record<Lang, Record<MessageKey, string>> = {
  ko: messages_ko,
  en: messages_en,
  ja: messages_ja,
};

let currentLang: Lang = 'ko';
const listeners = new Set<() => void>();

export function setLang(lang: Lang): void {
  if (lang === currentLang) return;
  currentLang = lang;
  listeners.forEach((fn) => fn());
}

export function getLang(): Lang {
  return currentLang;
}

/** lang 바뀔 때마다 호출될 콜백 등록. unsubscribe 반환. */
export function onLangChange(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

export function t(
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  const bundle = bundles[currentLang];
  // fallback: 현재 lang → ko 원본 → 키 자체 (개발 중 누락 가시화)
  const template = bundle[key] ?? messages_ko[key] ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, k) => {
    const v = params[k];
    return v === undefined ? `{${k}}` : String(v);
  });
}
