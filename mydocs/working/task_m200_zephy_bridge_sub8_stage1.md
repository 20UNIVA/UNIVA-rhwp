# Task #zephy-bridge Sub-8 Stage 1 — edit_op.rs 광고 + 단위 테스트

본 보고서는 [task_m200_zephy_bridge_sub8_impl.md](../plans/task_m200_zephy_bridge_sub8_impl.md) 의 *Stage 1 (edit_op.rs 네 자리 패치)* + *Stage 2 (단위 테스트 두 자리)* 통합 결과.

## Stage 결과 표

| Stage | 패치 | 결과 |
|---|---|---|
| 1 | 패치 1 — `EditOperation` enum 에 `InsertPageBreak { section, para, offset }` variant 추가 ([edit_op.rs](../../src/document_core/commands/edit_op.rs) ) | `cargo build` PASS (7.55s) |
| 1 | 패치 2 — `affected_range` arm: `before=single(*para), after={start: *para, end: *para+2}` | 위 통합 |
| 1 | 패치 3 — `apply_edit_op` arm: `self.insert_page_break_native(*section, *para, *offset)?` | 위 통합 |
| 1 | 패치 4 — `apply_inverse_edit_op` arm: `unreachable!("Sub-8 variant uses snapshot stash for inverse")` | 위 통합 |
| 2 | 패치 6 — `affected_range_insert_page_break_grows_after` 단위 테스트 | `cargo test insert_page_break` 2/2 PASS |
| 2 | 패치 7 — `apply_insert_page_break_splits_and_sets_column_type` 단위 테스트 | 위 통합 |

## 자동 검증 결과

### `cargo test --lib insert_page_break`

```
running 2 tests
test document_core::commands::edit_op::tests::affected_range_insert_page_break_grows_after ... ok
test document_core::commands::edit_op::tests::apply_insert_page_break_splits_and_sets_column_type ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1491 filtered out; finished in 0.01s
```

### `cargo test --workspace --lib` (전 회귀 검증)

```
test result: ok. 1487 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 52.99s
```

*Stage 1·2 의 패치가 기존 1487 unit test 어느 자리에도 영향 없음 확인*.

## Phase 0 검증 자리

`_impl.md` 의 *위험 자리* 한 줄씩 검증:

| 위험 | 결과 |
|---|---|
| `affected_range.after.end = para + 2` off-by-one | ★ 패치 6 의 `assert_eq!(r.after, ParaRange { start: 3, end: 5 })` 가 *exclusive 가정 정합* 확인 — `SplitParagraph` ([edit_op.rs:1606-1611](../../src/document_core/commands/edit_op.rs#L1606-L1611)) 와 동일 모양 |
| `DocumentCore::new_empty()` 의 빈 paragraphs | ★ 패치 7 에서 `core_with_text("한 줄")` helper 사용 — 기존 `apply_set_paragraph_style_align_via_advertised_key` 와 동일 패턴, 정합 |
| `insert_page_break_native` 반환 JSON 모양 | `apply_edit_op` 의 arm 이 `?` 만으로 `Result<(), HwpError>` 보장 — 반환 String 은 자동 drop, 다른 12 arm 과 동일 |
| `op_stash` 새 op tag 호환 | `op_stash` 가 *opaque JSON 그대로 저장* — sqlite schema 변경 없음. 자동 정합 ✓ |

## 다음 단계

Stage 3 — `main.rs` 의 workbench handler 에 `"insert_page_break"` action arm 추가 + 빌드·테스트 검증. 별도 `_stage2.md` 보고서로 정리.
