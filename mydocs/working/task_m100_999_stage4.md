# Task #999 Stage 4 완료보고서 — rhwp-studio 클라이언트 미러링 + iframe fileId 연결

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 4

## 목표

iframe(rhwp-studio)에서 일어난 편집을 SSR 서버 세션에 **디바운스 배치**로 미러링하고, iframe 진입 시 `fileId`로 서버 세션을 생성·연결한다.

## 구현 내용

### 신규 `rhwp-studio/src/core/session-client.ts`

- `MirrorSink` 인터페이스: `queueOp(op)` / `requestSnapshot()`
- `SessionClient` (implements MirrorSink):
  - `createSession(bytes)` → `POST /sessions` {fileId, format, fileBase64}
  - `queueOp(op)` → 디바운스(기본 600ms) 후 `POST /sessions/{id}/ops` 배치 전송
  - `requestSnapshot()` → 대기 op 폐기 후 `PUT /sessions/{id}/snapshot` (전체 export)
  - 실패 시 큐 복원 + 재시도, `beforeunload` 시 `sendBeacon`으로 잔여 flush
  - `bytesToBase64` 청크 처리(대용량 안전)

### `rhwp-studio/src/engine/input-handler.ts`

- `mirrorSink: MirrorSink | null` 필드 추가
- `executeOperation` 말미에 `this.mirror(desc)` 호출
- `mirror(desc)`: 연산형(`serialize()` 성공)은 `queueOp`, 그 외(스냅샷형/직렬화 불가/셀 내부)는 `requestSnapshot` 폴백

### `rhwp-studio/src/main.ts`

- URL query `?fileId=&ssrBase=` 파싱 (`SSR_URL_FILE_ID`, `SSR_BASE_URL`)
- `connectSsrSession(bytes, fileId)`: 세션 생성 + `inputHandler.mirrorSink` 연결. 실패해도 로컬 편집 지속(graceful degrade)
- `loadBytes(..., fileId)`: 로드 후 fileId 있으면 세션 연결
- postMessage `loadFile`/`hwpctl-load` 핸들러에서 `fileId` 전달

### `npm/editor/index.js`, `index.d.ts`

- `loadFile(data, fileName, { fileId })` — 부모 페이지가 fileId를 전달하면 postMessage params에 포함

## 통신 경로 (확정 설계 반영)

- iframe이 서버에 **직접 HTTP** (`ssrBase` 미지정 시 same-origin 상대경로)
- 편집 → **디바운스 배치** op 전송, 스냅샷형은 전체 export PUT
- 단일 편집자 1차 (세션 lock은 서버 측)

## 검증

```
npx tsc --noEmit
  전체 에러 2개 (모두 @wasm/rhwp.js — WASM pkg/ 빌드 부재, 기존 환경 문제)
  session-client / input-handler / main.ts / command.ts / edit-op 관련 에러: 0
```

**프로토콜 일치 확인**: SessionClient가 보내는 요청 형식이 Stage 3 실서버 스모크에서 성공한 형식과 동일.

| 클라이언트 | 서버 DTO (Stage 3) |
|-----------|-------------------|
| `POST /sessions` {fileId, format, fileBase64} | `CreateReq` ✓ |
| `POST /ops` [{op,section,para,...}] | `Json<Vec<Value>>` ✓ |
| `PUT /snapshot` {fileBase64} | `SnapshotReq` ✓ |
| op 직렬화 `{op:'insert_text',...}` | `EditOperation` serde ✓ |

## 한계 / 후속

- **브라우저 E2E 미수행**: WASM `pkg/` 산출물이 없어(Docker WASM 빌드 필요) vite 빌드/puppeteer E2E를 본 단계에서 실행하지 못함 → Stage 6에서 WASM 빌드 가능 시 수행. 현재는 타입 검증 + 프로토콜 일치로 1차 검증.
- 디바운스 중 op flush와 snapshot PUT의 엄밀한 순서 보장은 단일 편집자 가정하 best-effort. 멀티 편집자 단계에서 seq 기반 정합 재설계.

## 다음 단계

Stage 5 — export API(`GET /sessions/{id}/export`) + 외부(minio) 모듈 인터페이스 경계 정의.
