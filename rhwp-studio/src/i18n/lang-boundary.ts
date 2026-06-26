/**
 * 부모 시스템(agent) 으로부터 *언어 설정* 을 받아 내부 `lang` 으로 갈아끼우는 *경계 모듈*.
 *
 * 경계엔 `sysLang` (시스템이 내려 박은 값), 내부엔 `lang` 으로 정착.
 *
 * 두 입력 경로:
 *   (1) iframe URL `?sysLang=en|ja|ko` — 초기값
 *   (2) `postMessage({ type: 'rhwp:set-locale', sysLang: 'en' })` — 실시간 교체
 *
 * 두 경로 모두 *유효한 sysLang 일 때만* setLang 호출. 무효·미지정이면 기본 `ko` 유지.
 */

import { setLang, type Lang } from './t';

const VALID_LANGS: readonly Lang[] = ['ko', 'en', 'ja'];

function isValidLang(v: unknown): v is Lang {
  return typeof v === 'string' && (VALID_LANGS as readonly string[]).includes(v);
}

/** iframe URL `?sysLang=...` 읽어 초기 lang 설정. 없거나 무효면 ko (기본값) 유지. */
export function applyInitialLangFromUrl(): void {
  const sysLang = new URLSearchParams(location.search).get('sysLang');
  if (isValidLang(sysLang)) setLang(sysLang);
}

/**
 * 부모창의 `postMessage` 수신해 lang 교체.
 * @param expectedOrigin — `*` 면 origin 검증 skip (개발용). 그 외는 정확히 일치해야 함.
 */
export function attachLangPostMessageListener(expectedOrigin: string): void {
  window.addEventListener('message', (e) => {
    if (expectedOrigin !== '*' && e.origin !== expectedOrigin) return;
    const data = e.data;
    if (!data || typeof data !== 'object') return;
    if ((data as { type?: unknown }).type !== 'rhwp:set-locale') return;
    const sysLang = (data as { sysLang?: unknown }).sysLang;
    if (isValidLang(sysLang)) setLang(sysLang);
  });
}
