# Sub-5 — `/hwp` prefix 일괄 적용

## 배경

지금까지 server / rhwp-studio 모두 라우팅 루트 (`/`) 를 점유. 같은 호스트에서 다른 서비스와 공존하기 어렵고, 리버스 프록시 설정도 path 구분 없이 *서브도메인 분리* 가 강제됨. 단일 도메인의 `/hwp` 아래로 모든 경로 (정적 자산·API·헬스체크·WebSocket) 를 nest 해 prefix 한 줄로 격리.

## 변경

### 1. server — [server/src/main.rs:1254-1305](../../server/src/main.rs#L1254)

`fn router(state)` 끝부분을 두 단계로 재구성.

```rust
// 1) 기존 14 route + 정적 자산 fallback 을 내부 app 으로 묶기.
let mut app = Router::new().route(...).layer(CorsLayer::permissive()).with_state(state);
if let Some((dir, idx)) = index_path.clone() {
    app = app.fallback_service(
        ServeDir::new(dir)
            .append_index_html_on_directories(true)
            .fallback(ServeFile::new(idx)),  // SPA deep-link: ?fileId=... 새로고침
    );
}

// 2) Router::new().nest("/hwp", app) + trailing-slash 사각지대 보정.
let mut root = Router::new().nest("/hwp", app);
if let Some((_, idx)) = index_path {
    root = root.route_service("/hwp/", ServeFile::new(idx));
}
root
```

- *SPA deep-link*: `ServeDir.fallback(ServeFile::new(index))` — `/hwp/?fileId=X` 새로고침 시 정적 파일에 없는 경로라도 index.html 로 폴백 → 클라이언트 라우팅이 처리.
- *trailing-slash*: axum 0.7 `nest("/hwp")` 는 `/hwp`(exact) 와 `/hwp/{*rest}`(≥1세그) 만 매칭. 정확히 `/hwp/` 가 빠지므로 `route_service("/hwp/", ServeFile)` 로 명시 보정.

### 2. vite — [rhwp-studio/vite.config.ts](../../rhwp-studio/vite.config.ts)

- 최상위에 `base: '/hwp/'` 추가 — 모든 자산 URL (`<script src>`, `<link href>`, 동적 `import()`, `import.meta.env.BASE_URL`) 이 `/hwp/...` 로 생성.
- PWA `manifest.start_url` / `scope` 를 `/rhwp/` → `/hwp/` 로 통일.

### 3. 프론트 — [rhwp-studio/src/main.ts:67](../../rhwp-studio/src/main.ts#L67)

```ts
const SSR_BASE_URL =
  SSR_PARAMS.get('ssrBase') ?? import.meta.env.BASE_URL.replace(/\/$/, '');
```

`ssrBase` 명시 시 그 값 우선, 없으면 `import.meta.env.BASE_URL` (`/hwp/`) 의 trailing slash 만 제거. fetch 도, [session-client.ts:106-107](../../rhwp-studio/src/core/session-client.ts#L106) 의 `baseUrlWs` 도 prefix 자동 포함 (session-client 코드 변경 *없음*).

### 4. e2e — 5개 파일

- [sub2-helpers.mjs](../../rhwp-studio/e2e/sub2-helpers.mjs) — `BASE='http://127.0.0.1:7710/hwp'`, `WS_BASE='ws://127.0.0.1:7710/hwp'`
- [sub2-server.sh](../../rhwp-studio/e2e/sub2-server.sh) — 헬스체크 `/hwp/health`
- [sub2-canvas-helpers.mjs](../../rhwp-studio/e2e/sub2-canvas-helpers.mjs) — `STUDIO_BASE` 에 `/hwp` 포함
- [ws-bridge.test.mjs](../../rhwp-studio/e2e/ws-bridge.test.mjs) — `SERVER`/`WS_BASE` 에 `/hwp`
- [sub3-ir-compact.test.mjs](../../rhwp-studio/e2e/sub3-ir-compact.test.mjs) — `BASE_URL` fallback 에 `/hwp`

### 5. notebook — `hwp_sub_agent_simulation_ssr.ipynb` cell 1

`SSR_BASE = 'http://127.0.0.1:7710'` → `'http://127.0.0.1:7710/hwp'`. 모든 `_handle_*` 함수가 이 BASE 변수를 통해 호출하므로 한 줄 변경으로 전 액션 prefix 일괄 반영.

### 6. deploy — [deploy/README.md](../../deploy/README.md), [deploy/run.sh](../../deploy/run.sh)

- 모든 URL 예시 (iframe, 헬스체크, API 호출) 에 `/hwp` 적용.
- **§7 신규** — Nginx Proxy Manager GUI 절차 + `nginx.conf` 예시. WS 업그레이드 헤더, `proxy_pass` 가 path 재작성 *없이* 통째로 전달하는 점 명시.
- run.sh 의 echo 메시지에 prefix 안내 한 줄 추가.

## 검증

### 단위 / 빌드

- `cargo test` — 70 tests pass (회귀 없음).
- `vite build` — 산출물에 `/hwp/assets/...`, `/hwp/favicon.ico`, `/hwp/manifest.webmanifest` 정상 반영.

### Live smoke test (curl)

| 경로 | 결과 | 의미 |
|---|---|---|
| `GET /hwp/health` | `ok` | nest 안의 API 도달 |
| `GET /hwp/` | `<!DOCTYPE html>` | trailing-slash 사각지대 보정 동작 |
| `GET /hwp` | 200 | nest exact 매칭 (index.html fallback) |
| `GET /` | **404** | prefix 외부 차단 |
| `GET /health` | **404** | prefix 외부 차단 |
| `GET /hwp/assets/` | 200 | 정적 자산 서빙 |
| `POST /hwp/sessions` | 200 | 세션 생성 API |
| `POST /sessions` (prefix 없이) | **404** | 외부 차단 |
| `GET /hwp/sessions/{id}/ir` | 200 | IR 조회 |

### e2e 회귀

9 시나리오 전수 통과:

- sub4-patch-diff (9/9) — PatchDiff 응답 셀 압축 검증
- ws-bridge — 양방향 WS 통신
- sub3-ir-compact — Compact IR slice
- sub2-replace-cell-runs / sub2-canvas-insert-text — Canvas 시각 회귀
- sub2-audit-diff-ir-slice / sub2-partial-update / sub2-replace-runs
- sub2-insert-text-in-cell / sub2-undo

## 효과

1. *단일 prefix 격리* — 같은 도메인에서 다른 서비스와 path 로 공존 가능 (`/hwp/...` vs `/other/...`).
2. *리버스 프록시 설정 단순화* — nginx 가 `location /hwp/ { proxy_pass http://127.0.0.1:7710; }` 한 블록으로 path 재작성 없이 통째 전달.
3. *prefix 누락 시 즉시 404* — 모델/MCP 가 실수로 root 호출하면 *조용히 fallback 가 아닌* 즉시 실패하므로 디버깅 명확.

## 트레이드오프

- 기존 자동화·외부 호출자가 root 경로를 가정하면 모두 prefix 추가 필요. 이번 변경에서 e2e + notebook + deploy 가이드 일괄 갱신 — 다른 사용처는 없음 (확인 완료).
- `ssrBase` 가 cross-origin 으로 명시되는 경우 *부모 측이 prefix 포함* 시켜야 함. deploy README iframe 예시에 명시.
