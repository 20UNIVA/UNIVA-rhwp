# Sub-8 — InsertPageBreak EditOperation 광고 (구현 계획서)

수행 계획서: [task_m200_zephy_bridge_sub8.md](task_m200_zephy_bridge_sub8.md). Phase 0 의 *영향 자리 파악* 결과 *코드 변경 자리·줄 번호·정확한 패치 모양* 이 모두 또렷해 4 단계로 묶는다.

## Phase 0 결과 — 자리 매트릭스

| 자리 | 줄 번호 | 모양 (참고: InsertParagraph) |
|---|---|---|
| `EditOperation` enum | [edit_op.rs:276-283](../../src/document_core/commands/edit_op.rs#L276-L283) | `InsertParagraph { section, after_para, count, style }` 옆에 신규 variant 추가 |
| `EditOperation::affected_range` | [edit_op.rs:694-699](../../src/document_core/commands/edit_op.rs#L694-L699) | `InsertParagraph` arm 옆 새 arm |
| `EditOperation::apply_edit_op` | [edit_op.rs:871-880](../../src/document_core/commands/edit_op.rs#L871-L880) | 같은 패턴 — `self.X_native(...)?` |
| `EditOperation::apply_inverse_edit_op` | [edit_op.rs:1010-1012](../../src/document_core/commands/edit_op.rs#L1010-L1012) | `unreachable!("Sub-2 variants use snapshot stash for inverse")` 한 줄 |
| `workbench` handler (server) | [main.rs:670-696](../../server/src/main.rs#L670-L696) `"insert_paragraph"` arm | 새 `"insert_page_break"` arm — payload struct 만들어 `apply_op_with_stash` |
| 단위 테스트 | [edit_op.rs:1465-1599](../../src/document_core/commands/edit_op.rs#L1465-L1599) `affected_range_*` block | `affected_range_insert_page_break_splits_paragraph` 한 자리 |
| e2e (선택) | `server/tests/` 디렉토리 또는 e2e helper | `POST /workbench {action:"insert_page_break"}` 한 시나리오 |

## 정확한 코드 패치

### 패치 1 — `EditOperation` enum 에 variant 추가

[edit_op.rs:351-361](../../src/document_core/commands/edit_op.rs#L351-L361) (`DeleteRangeInCell` 끝) 뒤, `}` 닫기 직전에:

```rust
    /// 강제 쪽 나누기 (Ctrl+Enter 동등). `insert_page_break_native` 위임.
    /// 동작: `(section, para)` 의 `offset` 자리에서 문단 분할 + 새 문단에
    /// `ColumnBreakType::Page` 설정 + 페이지 재배치. 분할 결과 새 문단이
    /// `para+1` 자리에 들어간다.
    InsertPageBreak {
        section: usize,
        para: usize,
        offset: usize,
    },
```

### 패치 2 — `affected_range` arm

[edit_op.rs:694-699](../../src/document_core/commands/edit_op.rs#L694-L699) `InsertParagraph` arm 옆에:

```rust
            EditOperation::InsertPageBreak { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange { start: *para, end: *para + 2 },
                cell: None,
            },
```

근거: `insert_page_break_native` 가 *`para` 분할 → `para+1` 자리에 새 문단 삽입* ([text_editing.rs:1096-1100](../../src/document_core/commands/text_editing.rs#L1096-L1100)). 즉 *before 의 단일 문단 `para` 가 after 에서는 두 문단 `[para, para+1]` 로* 확장. `end` 는 *exclusive* 라 `para+2`.

### 패치 3 — `apply_edit_op` arm

[edit_op.rs:871-880](../../src/document_core/commands/edit_op.rs#L871-L880) `InsertParagraph` arm 옆에:

```rust
            EditOperation::InsertPageBreak { section, para, offset } => {
                self.insert_page_break_native(*section, *para, *offset)?;
            }
```

근거: 같은 패턴. 반환값 (`Result<String, HwpError>` 의 `Ok(String)` JSON) 은 *`apply_edit_op` 가 `Result<(), HwpError>` 만 보장* 하므로 `?` 만으로 충분 — 다른 12 arm 도 같은 모양.

### 패치 4 — `apply_inverse_edit_op` arm

[edit_op.rs:1010-1012](../../src/document_core/commands/edit_op.rs#L1010-L1012) `InsertParagraph` arm 옆에:

```rust
            EditOperation::InsertPageBreak { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
```

근거: Sub-2 의 모든 신규 variant 가 *snapshot stash 로 inverse* 처리 — `apply_op_with_stash` 가 `op_stash` 에 *before_blob* 을 박고, undo 는 그 blob 으로 *통째 복원*. 새 InsertPageBreak 도 같은 자리에 정합.

### 패치 5 — workbench handler arm

[main.rs:670-696](../../server/src/main.rs#L670-L696) `"insert_paragraph"` arm 끝 뒤, `"delete_element"` arm 시작 직전에:

```rust
        "insert_page_break" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para: usize,
                offset: usize,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::InsertPageBreak {
                section: payload.section,
                para: payload.para,
                offset: payload.offset,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
```

### 패치 6 — 단위 테스트 (affected_range)

[edit_op.rs:1582-1598](../../src/document_core/commands/edit_op.rs#L1582-L1598) `affected_range_split_paragraph_grows_after` 옆에:

```rust
    #[test]
    fn affected_range_insert_page_break_grows_after() {
        let op = EditOperation::InsertPageBreak {
            section: 0,
            para: 3,
            offset: 5,
        };
        let r = op.affected_range();
        assert_eq!(r.section, 0);
        assert_eq!(r.before, ParaRange::single(3));
        assert_eq!(r.after, ParaRange { start: 3, end: 5 });
        assert!(r.cell.is_none());
    }
```

근거: `para=3`, `count=1` (page break 는 항상 +1 문단) → after `[3, 5)` 즉 `{3, 4}` 두 문단.

### 패치 7 — apply 단위 테스트

[edit_op.rs:1781-…](../../src/document_core/commands/edit_op.rs#L1781) `apply_set_cell_style_bgcolor_round_trip` 같은 모양 따라 새 자리:

```rust
    #[test]
    fn apply_insert_page_break_splits_and_sets_column_type() {
        use crate::model::paragraph::ColumnBreakType;
        let mut core = DocumentCore::new_empty();
        // 본문에 짧은 문단 한 자리 추가 — split offset 자리 확보용.
        core.apply_edit_op(&EditOperation::InsertText {
            section: 0, para: 0, offset: 0,
            text: "한 줄".to_string(),
        }).unwrap();

        let before_count = core.document().sections[0].paragraphs.len();
        core.apply_edit_op(&EditOperation::InsertPageBreak {
            section: 0, para: 0, offset: 1,
        }).unwrap();

        let secs = &core.document().sections;
        assert_eq!(secs[0].paragraphs.len(), before_count + 1, "한 문단 split");
        assert_eq!(secs[0].paragraphs[1].column_type, ColumnBreakType::Page, "새 문단 page break");
    }
```

`DocumentCore::new_empty()` 가 *기본 1 문단* 으로 시작하는지 확인 — Phase 0 의 *기존 apply test 모양* 본 후 결정 (필요 시 init paragraph 추가).

## 단계 분할 (CLAUDE.md 룰: 최소 3, 최대 6)

| Stage | 자리 | 산출물 |
|---|---|---|
| 1 | 패치 1·2·3·4 (edit_op.rs 네 자리) | enum + affected_range + apply + inverse. `cargo build` 통과 |
| 2 | 패치 6·7 (단위 테스트 두 자리) | `cargo test edit_op` 통과 |
| 3 | 패치 5 (workbench handler arm) + e2e | `cargo test --workspace` 통과. 서버 띄워 `curl POST /workbench` 한 시나리오 검증 |
| 4 | 정합 검증 — `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`, 전체 `cargo test`. 두 자리 커밋 (stage 1+2 한 자리, stage 3 한 자리) | clippy warning 0, 전 test PASS |

각 stage 끝에 `mydocs/working/task_m200_zephy_bridge_sub8_stage{N}.md` 단계별 완료 보고서 + 해당 단계 소스 커밋과 함께 본 브랜치 (`feature/jerry-command-expansion`) 에 커밋.

## 위험·우회

| 위험 | 우회 |
|---|---|
| `affected_range` 의 `after.end` 가 `para+2` 가 *off-by-one* 일 가능성 — `InsertParagraph` 는 `*after_para + 1 + *count` 즉 *exclusive* | 위 단위 테스트 (패치 6) 가 *exclusive 가정으로 작성*. 모양 어긋나면 그 자리에서 갈아끼움 |
| `DocumentCore::new_empty()` 가 빈 paragraphs 로 시작해 `apply InsertPageBreak section=0, para=0` 가 *para out of range* fail | 패치 7 의 사전 `InsertText` 한 줄로 *최소 한 문단* 확보. 또는 `new_empty()` 의 실제 모양 확인 후 조정 |
| `insert_page_break_native` 내부의 `recompose_section + paginate_if_needed + invalidate_page_tree_cache` 가 *큰 문서에서 비용* — server lock hold time | 본 sub 의 비목표. 측정만, *추후 sub 에서 async 측 결정* |
| `op_stash` 에 새 op 종류 저장 — 기존 sqlite migration 영향 | `op_stash` 가 *opaque JSON 그대로 저장* 이라 새 op tag (`"insert_page_break"`) 도 자연 정합. *sqlite schema 변경 없음* |
| 모델·skill 측 호출 자리가 *없는 상태로 서버만 광고* — feature dead until next sub | 본 sub 비목표. 다음 사이클에서 hwp-doc-edit / stylish-doc-edit 가이드 갱신 |

## 승인 요청

위 7 자리 패치 + 4 stage 분할로 진행해도 OK 한지 작업지시자 확인 부탁. 승인 후 *Stage 1 (edit_op.rs 네 자리)* 부터 진입.
