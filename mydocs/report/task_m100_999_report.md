# Task #999 최종 결과보고서 — SSR 배포 전환: fileId 세션 기반 서버사이드 문서 영속 + 모델 조회 API

- 이슈: #999 (잠정 — gh 인증 복구 후 정식 채번)
- 브랜치: `feature/ssr` (포크 `20UNIVA/UNIVA-rhwp`)
- 마일스톤: M100 (v1.0.0)
- 계획서: [수행](../plans/task_m100_999.md) · [구현](../plans/task_m100_999_impl.md)

## 1. 목표와 결과

기존 rhwp는 순수 프론트엔드 SPA(브라우저 WASM)로, 탭/iframe을 닫으면 문서 상태가 휘발했다. 이를 **서버사이드 문서 상태 영속** 구조로 전환했다.

| 요구 | 결과 |
|------|------|
| iframe으로 에디터를 불러와 편집 | studio가 `?fileId=&ssrBase=` 또는 postMessage `loadFile({fileId})` 로 세션 연결 |
| 닫아도 서버단에 파일·patch 유지 | sqlite 영속 + 복원. DELETE/재시작 후에도 편집 유지 실증 |
| 모델이 현재 상태 조회 | `GET /sessions/{id}/ir` → Document IR JSON (신규 직렬화) |
| fileId(=minio) 단위 세션 | fileId 키 세션 매니저 |
| 현재 상태를 rhwp 객체로 받는 API | IR JSON + `GET /export`(hwp/hwpx) |

## 2. 확정 설계 (작업지시자)

1. **Native Rust 서버**(axum) 신규 — `Document` IR의 single source of truth
2. **하이브리드 동기화** — 단순 편집은 `EditOperation` 양방향 patch, 복잡 연산은 전체 스냅샷
3. **영속** — 작업 중 VM 로컬(sqlite), 확정 시 export → 외부 minio 모듈
4. **모델 조회** — Document IR JSON
5. **통신** — iframe 직접 HTTP / **디바운스 배치** / **sqlite** / **단일 편집자(1차)**
6. **minio 제외** — 서버는 fileId + 파일 바이트만 input

## 3. 단계별 산출물

| Stage | 산출물 | 검증 |
|-------|--------|------|
| 1 | `src/model/ir_view.rs` — IR→JSON 뷰(`to_ir_json`) | 단위 2 |
| 2 | `src/document_core/commands/edit_op.rs` — `EditOperation` 양방향 + native 적용기 / TS `edit-op.ts`·`command.ts serialize()` | 라운드트립 4 + 결정성 1 + tsc 클린 |
| 3 | `server/` crate — axum 세션 서버 + sqlite 영속 + 복원 | store 3 + 스모크 + 재시작 복원 |
| 4 | `rhwp-studio/src/core/session-client.ts` + InputHandler 미러링 + main.ts fileId 연결 + npm/editor `loadFile({fileId})` | tsc 클린 + 프로토콜 일치 |
| 5 | `GET /export` + 외부 모듈 계약 문서 | export 라운드트립(hwp/hwpx) |
| 6 | 결정성 회귀 테스트 + 통합 시나리오 | 전체 lib 1412 + 통합 PASS |

## 4. 아키텍처 (최종)

```
[클라이언트 iframe: rhwp 에디터(WASM)]
   │  편집 → SessionClient
   │   ├─ 연산형: 디바운스 배치 → POST /sessions/{id}/ops
   │   └─ 스냅샷형: 전체 export → PUT /sessions/{id}/snapshot
   ▼ (직접 HTTP)
[VM: rhwp-server (axum)]
   세션 매니저 fileId → DocumentCore (native rhwp, WASM과 동일 document_core 경로)
   sqlite: sessions(base) + ops(연산로그) + snapshots
   복원: 최근 snapshot(없으면 base) + 이후 ops 재적용
   ├─ [모델]  GET /sessions/{id}/ir       → Document IR JSON
   └─ [외부]  GET /sessions/{id}/export   → hwp/hwpx → minio 업로드(외부)
```

## 5. "patch만 남기기"의 구현 (작업지시자 질의 응답)

undo가 작동한다는 것은 각 편집이 이미 역연산 정보를 보유한다는 뜻이다. 이를 직렬화 가능한 양방향 `EditOperation`(forward + inverse 데이터)으로 승격했다:
- `InsertText`↔삭제, `DeleteText`(deleted_text)↔삽입, `SplitParagraph`↔`MergeParagraph`(prev_len)
- 클라이언트 `EditCommand.serialize()` → 서버 native `apply_edit_ops_json` (같은 `*_native` 경로 → 결정성 보장, `test_op_apply_equals_direct_native`로 고정)
- 연산으로 표현 불가한 `SnapshotCommand`(붙여넣기/객체/표)는 전체 스냅샷 폴백

## 6. 검증 종합

- `cargo test --lib`: **1412 passed, 0 failed** (신규 7 포함, 기존 무손상)
- `server`: store 3 passed, clippy error 0
- 실서버 스모크: 세션 생성/op/ir/재시작 복원/export 라운드트립 PASS
- **통합 시나리오**: 편집 → DELETE(연결 끊김) → 복원 조회 → export **PASS**

## 7. 한계 / 후속 과제

- **브라우저 E2E 미수행**: Docker daemon 미실행으로 WASM `pkg/` 빌드 불가. studio는 타입+프로토콜+결정성으로 대체 검증. WASM 빌드 환경에서 후속 E2E 권장.
- 연산형 직렬화는 4종(텍스트/문단)만. 줄바꿈/탭/서식/표/객체/셀편집은 스냅샷 폴백 — 점진 확대.
- char 오프셋 BMP 한정(서로게이트 페어 미정규화).
- 단일 편집자 1차. 멀티 편집자(CRDT/OT), 세션 TTL/GC는 후속.
- gh 인증 만료로 이슈 정식 채번 보류(잠정 #999).

## 8. 변경 파일

```
신규: src/model/ir_view.rs
      src/document_core/commands/edit_op.rs
      server/{Cargo.toml,.gitignore,src/main.rs,src/store.rs}
      rhwp-studio/src/core/session-client.ts
      rhwp-studio/src/engine/edit-op.ts
      mydocs/tech/ssr_server_external_module_contract.md
수정: src/model/mod.rs, src/document_core/mod.rs, src/document_core/commands/mod.rs
      rhwp-studio/src/engine/command.ts, input-handler.ts, main.ts
      npm/editor/index.js, index.d.ts
```
