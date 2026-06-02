# Task #999 Stage 2 완료보고서 — EditOperation 양방향 프로토콜 + native 적용기

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 2

## 목표

클라이언트(WASM)에서 일어난 결정적 편집을 서버 `DocumentCore` 에 동일하게 재현하기 위한 **직렬화 가능한 양방향 편집 연산** 프로토콜을 정의하고, native 적용기를 구현한다. inverse 데이터를 함께 담아 서버 측 undo도 가능하게 한다.

## 구현 내용

### Rust — 신규 `src/document_core/commands/edit_op.rs`

- `EditOperation` enum (serde, `#[serde(tag = "op", rename_all = "snake_case")]`):
  - `InsertText { section, para, offset, text }`
  - `DeleteText { section, para, offset, count, deleted_text }` — `deleted_text`가 inverse 데이터
  - `SplitParagraph { section, para, offset }`
  - `MergeParagraph { section, para, prev_len }` — `prev_len`이 inverse(분할 지점) 데이터
- `impl DocumentCore`:
  - `apply_edit_op(&op)` — 정방향. 기존 `insert_text_native`/`delete_text_native`/`split_paragraph_native`/`merge_paragraph_native` 재사용
  - `apply_inverse_edit_op(&op)` — 역방향 (삽입↔삭제, 분할↔병합 대칭)
  - `apply_edit_ops(&[op])` / `apply_edit_ops_json(json)` — 배치 적용

### Rust — 모듈 등록

- `src/document_core/commands/mod.rs`: `pub mod edit_op;`
- `src/document_core/mod.rs`: `pub use commands::edit_op::EditOperation;` (서버/wasm_api에서 사용)

### TS — 신규 `rhwp-studio/src/engine/edit-op.ts`

- `EditOperation` 유니온 타입 — Rust enum과 **동일 JSON 스키마**

### TS — `rhwp-studio/src/engine/command.ts`

- `EditCommand` 인터페이스에 `serialize?(): EditOperation | null` 추가
- 4개 커맨드에 `serialize()` 구현: `InsertTextCommand`, `DeleteTextCommand`, `SplitParagraphCommand`, `MergeParagraphCommand`
  - 셀 내부 편집(`isCell`)은 `null` 반환 → 스냅샷 동기화 폴백

## 핵심 — 양방향 대칭성 (TS undo ↔ Rust inverse 일치)

| 연산 | TS `undo()` | Rust `apply_inverse` |
|------|-------------|----------------------|
| InsertText | `doDeleteText(pos, text.length)` | `delete_text_native(.., text.chars().count())` |
| DeleteText | `doInsertText(pos, deletedText)` | `insert_text_native(.., deleted_text)` |
| SplitParagraph | `mergeParagraph(sec, para+1)` | `merge_paragraph_native(sec, para+1)` |
| MergeParagraph | `splitParagraph(sec, para-1, mergePointOffset)` | `split_paragraph_native(sec, para-1, prev_len)` |

→ 클라이언트 undo 로직과 서버 역적용이 **동일 의미**. 같은 `*_native` 경로를 거치므로 결정성 보장.

## 검증

```
cargo test --lib document_core::commands::edit_op
  test_insert_text_roundtrip ... ok
  test_delete_text_roundtrip ... ok
  test_split_merge_roundtrip ... ok
  test_apply_ops_json ... ok
  test result: ok. 4 passed; 0 failed
```
- 라운드트립: `apply` 후 `apply_inverse` → 원본 텍스트/문단 수 복원 확인
- JSON 배치: `[{"op":"insert_text",...}]` 파싱·적용 검증

```
npx tsc --noEmit
  전체 에러 2개 (모두 @wasm/rhwp.js 모듈 미존재 — WASM pkg/ 빌드 부재로 인한 기존 환경 문제)
  command.ts / edit-op.ts 관련 에러: 0
```

## 스코프 결정·한계

- **WASM `applyOps` 노출 생략**: 1차는 단일 편집자 가정 → 클라이언트는 op를 *생성*만 하고 적용은 자체 편집 경로가 담당. 서버 적용은 native `apply_edit_ops_json` 으로 충분. 멀티 편집자 단계에서 WASM 노출 재검토.
- **char 오프셋**: TS는 UTF-16 `.length`, Rust는 코드포인트 `.chars().count()`. BMP 문자(한글 포함)는 일치하나 서로게이트 페어(이모지)는 불일치 가능 — 한글 문서 대상 1차 스코프상 수용, 후속 정규화 검토.
- **연산형 4종만 직렬화**: 줄바꿈/탭/선택삭제/서식/객체/표 편집은 1차 `null`(스냅샷 폴백). 점진 확대 예정.

## 다음 단계

Stage 3 — Native Rust 서버(axum) + 세션 매니저 + sqlite 영속.
