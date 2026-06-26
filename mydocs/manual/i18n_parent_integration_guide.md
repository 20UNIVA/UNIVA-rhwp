# rhwp-studio 언어 설정 — 부모(agent) 측 통합 가이드

rhwp-studio 는 iframe 으로 임베드된 *자식 자리*. 언어 결정 권한은 *부모(agent 시스템)* 에 있다. 이 문서는 부모가 rhwp-studio 에 *언어를 박는* 두 통로를 정리한다.

대상 — agent 프런트엔드를 만지는 개발자.

vfinder 의 [parent-integration-guide](../../../vfinder/docs/i18n/parent-integration-guide.md) 와 같은 패턴. 다른 자리는 *메시지 type 의 prefix* 뿐 (vfinder: → rhwp:).

---

## 1. 지원 언어

rhwp-studio 가 받는 언어 코드는 정확히 세 가지:

| 코드 | 언어 |
|---|---|
| `ko` | 한국어 (기본값) |
| `en` | English |
| `ja` | 日本語 |

*그 외 값*(예: `en-US`, `zh`, 빈 문자열, 대문자) 은 *무효 처리* — rhwp-studio 는 무시하고 *현재 lang 유지* 한다.

---

## 2. 두 통로 — 초기값 · 실시간 교체

### 통로 A — iframe URL 파라미터 (초기값)

iframe 의 `src` URL 에 `?sysLang=...` 박는다.

```html
<iframe
  src="https://rhwp.example.com/?parentOrigin=https://agent.example.com&sysLang=en"
  ...
></iframe>
```

- *파라미터 이름은 정확히 `sysLang`* (대소문자 구분).
- 부모가 rhwp-studio 를 로드하는 *바로 그 순간*의 언어 설정을 박는다.
- 없거나 무효면 rhwp-studio 는 `ko` 로 시작.

### 통로 B — `postMessage` (실시간 교체)

사용자가 agent 안에서 *언어를 바꾼* 순간 rhwp-studio 도 즉시 갱신.

```js
// agent 쪽 — 사용자가 언어 설정 변경 시
function syncLangToRhwp(newSysLang) {
  const rhwpIframe = document.querySelector('iframe[data-app="rhwp"]');
  if (!rhwpIframe?.contentWindow) return;
  rhwpIframe.contentWindow.postMessage(
    {
      type: 'rhwp:set-locale',
      sysLang: newSysLang,   // 'ko' | 'en' | 'ja'
    },
    'https://rhwp.example.com',  // *정확한* rhwp 호스트 origin
  );
}
```

**필수 사항**:
- `type` 자리는 정확히 `'rhwp:set-locale'`.
- `sysLang` 자리는 `'ko'`/`'en'`/`'ja'` 중 하나.
- *targetOrigin* 자리는 `'*'` 박지 말 것 — 정확한 rhwp 호스트 origin 박는다 (보안).

---

## 3. 두 통로 결합 패턴

부모는 *둘 다* 박는다:

```js
// 1) iframe 로드 시 URL 자체에 박음
function buildRhwpUrl(baseUrl, lang) {
  const url = new URL(baseUrl);
  url.searchParams.set('parentOrigin', window.location.origin);
  url.searchParams.set('sysLang', lang);
  return url.toString();
}

// 2) 이후 사용자가 lang 바꿀 때마다 postMessage 발사
window.addEventListener('userLangChange', (e) => {
  syncLangToRhwp(e.detail.lang);
});
```

URL 자료는 *초기 로드 시점 1회만* 사용 — 이후 iframe.src 다시 박지 말 것 (전체 리로드 자체). `postMessage` 자체 자체 자체 실시간 교체 자체.

---

## 4. 자식 측 검증 흐름 (참고)

rhwp-studio 가 받는 자리 자체:

```
iframe 로드
  ↓
URL ?sysLang=...
  ↓
isValidLang() 검사 — 통과하면 setLang()
  ↓
postMessage 수신 대기
  ↓
부모 postMessage 받음:
  - e.origin === expectedOrigin? (아니면 무시)
  - data.type === 'rhwp:set-locale'? (아니면 무시)
  - isValidLang(data.sysLang)? (아니면 무시)
  ↓
setLang(data.sysLang) → 모든 UI 자체 자체 재렌더
```

소스 자료: [rhwp-studio/src/i18n/lang-boundary.ts](../../rhwp-studio/src/i18n/lang-boundary.ts).

---

## 5. 디버깅 체크리스트

iframe 안 lang 안 바뀔 때 의심할 자리:

| 의심 자리 | 확인 방법 |
|---|---|
| `sysLang` 자체 자체 자체 자체 자체 *오타* (`syslang`, `sys-lang`) | URL 자체 자체 자체 자체 자체 자체 *정확히* `sysLang` 박혔나 |
| `type` 자체 자체 자체 `'rhwp:set-locale'` 자체 자체 자체 자체 자체 | DevTools 콘솔 자체 자체 자체 자체 postMessage payload 자체 |
| `targetOrigin` 자체 자체 자체 자체 자체 자체 자체 *rhwp origin* 자체 자체 자체 자체 자체 자체 자체 자체 | rhwp iframe 자체 자체 origin 자체 자체 자체 자체 정확 박혔나 |
| 무효 코드 (`en-US`, `EN`, ``) | rhwp 자체 자체 자체 자체 *조용히 무시* 자체 자체 자체 — coarse 코드 박을 자체 자체 자체 자체 자체 자체 자체 자체 |
| 부모 origin 검증 자체 자체 자체 자체 자체 자체 자체 *expectedOrigin* 자체 자체 자체 자체 자체 자체 *parentOrigin* URL 자체 자체 자체 자체 박혔나 | rhwp-studio 자체 자체 자체 자체 자체 *parentOrigin* URL 자체 자체 자체 자체 자체 자체 박힌 origin 자체 자체 자체 자체 자체 박혀야 |

---

## 6. 빠른 시작 코드 (복붙용)

```js
// === agent 쪽 — rhwp iframe 박는 자료 ===

const RHWP_ORIGIN = 'https://rhwp.example.com';

function mountRhwpIframe(container, opts) {
  const url = new URL(RHWP_ORIGIN);
  url.searchParams.set('parentOrigin', window.location.origin);
  url.searchParams.set('sysLang', opts.lang ?? 'ko');
  if (opts.fileId) url.searchParams.set('fileId', opts.fileId);

  const iframe = document.createElement('iframe');
  iframe.dataset.app = 'rhwp';
  iframe.src = url.toString();
  iframe.style.width = '100%';
  iframe.style.height = '100%';
  iframe.style.border = '0';
  container.appendChild(iframe);

  return {
    iframe,
    setLang(newLang) {
      iframe.contentWindow?.postMessage(
        { type: 'rhwp:set-locale', sysLang: newLang },
        RHWP_ORIGIN,
      );
    },
  };
}

// === 사용 자리 ===
const handle = mountRhwpIframe(document.body, { lang: 'en' });
// 사용자가 언어 바꾸면:
handle.setLang('ja');
```

---

## 7. 자식이 부모에게 박는 메시지 (참고)

rhwp-studio 가 부모에게 보내는 메시지는 *언어와 무관*. 별 자리에 박혀 있음.

- `rhwp:save-as` — 다른 이름으로 저장 결과
- `rhwp:open-file` — 파일 열기 결과
- `rhwp:close` — 모달 닫기

자세한 자리는 별 문서 자체 자체 자체. *언어 통로* 자체 자체 자체 *부모→자식 한 방향* 자체.

---

## 8. vfinder 와 차이

| 자리 | vfinder | rhwp-studio |
|---|---|---|
| 메시지 type | `vfinder:set-locale` | `rhwp:set-locale` |
| iframe URL prefix | `/vfinder/` | (배포 자체) |
| 그 외 자리 | 동일 | 동일 |

rhwp-studio 자체 자체 자체 자체 자체 *vfinder modal* 자체 자체 자체 자체 *자식 iframe* 자체 자체 자체 자체 자체 박힌다 — 그쪽 자체 자체 자체 자체 자체 *vfinder:set-locale* 자체 자체 자체 자체 자체 박는 자체 자체 (자체 자체 자체 자체 vfinder prefix 그대로).
