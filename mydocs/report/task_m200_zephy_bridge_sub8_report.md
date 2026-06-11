# Task #zephy-bridge Sub-8 — InsertPageBreak EditOperation 광고 (최종 결과 보고서)

수행 계획서: [task_m200_zephy_bridge_sub8.md](../plans/task_m200_zephy_bridge_sub8.md)
구현 계획서: [task_m200_zephy_bridge_sub8_impl.md](../plans/task_m200_zephy_bridge_sub8_impl.md)
단계 보고서: [_stage1.md](../working/task_m200_zephy_bridge_sub8_stage1.md), [_stage2.md](../working/task_m200_zephy_bridge_sub8_stage2.md)

## 배경

stylish-doc-edit (26ZEPHY-skills) 의 *blank_build 흐름 — 빈 새 문서 N 페이지 만들기* 를 rhwp 12 명령 어휘로 갈아끼우는 작업 중, *page break 동작이 외부 REST 에 광고되지 않은 자리* 가 발견됨. `insert_page_break_native` 함수는 [text_editing.rs:1071-1131](../../src/document_core/commands/text_editing.rs#L1071-L1131) 에 이미 구현 완비 — *enum 광고 + apply wire + workbench arm* 만 연결.

## 결과

| 자리 | 변경 |
|---|---|
| [edit_op.rs](../../src/document_core/commands/edit_op.rs) | `EditOperation::InsertPageBreak { section, para, offset }` variant 추가, affected_range / apply / inverse 3 arm wire |
| [main.rs](../../server/src/main.rs) | workbench handler 에 `"insert_page_break"` action arm — payload deserialize → `apply_op_with_stash` |
| 단위 테스트 | `affected_range_insert_page_break_grows_after` + `apply_insert_page_break_splits_and_sets_column_type` 2 자리 PASS |
| 자동 정합 | `POST /sessions/:id/ops` · `ws.rs` 의 `ClientMessage::Ops` · `op_stash` sqlite · `ServerEvent::Ops` broadcast — `serde(tag="op")` 정합으로 *추가 코드 0* |

## 검증

| 항목 | 결과 |
|---|---|
| `cargo build` | PASS (lib 7.55s, server 11.56s) |
| `cargo test --lib insert_page_break` | 2/2 PASS |
| `cargo test --workspace --lib` (회귀) | 1487 PASS, 0 FAIL, 6 ignored |
| `cargo clippy --workspace --lib -- -D warnings` | warning 0, error 0 |
| `cargo fmt --check` (내가 만진 자리만, CLAUDE.md 룰 정합) | 새 코드는 기존 패턴 그대로 따라 시각 정합. 기존 자리 fmt diff 는 본 sub 범위 밖 |

## 커밋

| 커밋 | 자리 | 메시지 |
|---|---|---|
| `9414a74c` | edit_op.rs + 계획서 두 자리 + _stage1.md | Sub-8: InsertPageBreak EditOperation 광고 |
| `2e492236` | server/src/main.rs + _stage2.md | Sub-8: workbench REST 의 insert_page_break action arm |
| (본 커밋) | _report.md + orders 갱신 | Sub-8: 최종 결과 보고서 |

## 사용 예시

```bash
# Ctrl+Enter 동등 — section 0, para 3 의 offset 5 자리에서 페이지 나눔
curl -X POST http://127.0.0.1:7710/hwp/sessions/<file_id>/workbench \
  -H 'Content-Type: application/json' \
  -d '{"action":"insert_page_break","payload":{"section":0,"para":3,"offset":5}}'

# 응답: {"seq":N, "applied":"ops", "info":null, "diff":{...}}
# 동작: para 3 분할 → 새 문단이 para 4 자리에 들어가고 column_type=Page 설정
#       → recompose_section + paginate_if_needed 자동
```

## 영향·이어지는 작업

| 자리 | 효과 |
|---|---|
| stylish-doc-edit (26ZEPHY-skills, `feature/jerry-rhwp`) | *blank_build 흐름 정합* — N 페이지 빈 문서 생성 시 `insert_page_break` action 사용 가능. 단계 3 (patch-phase.md 재작성) 재개 가능 |
| hwp-doc-edit (26ZEPHY-skills, `feature/jerry-rhwp`) | 명령 카탈로그 12 → 13 (insert_page_break 추가) 으로 확장 가능 — *별도 작업 사이클* |
| rhwp-studio (브라우저 측 WASM API) | 새 `wasm.insertPageBreak()` export 노출 — 사용자 직접 입력 단의 ws 진입 자리. *별개 사이클* |

## 비목표 확인

| 비목표 | 처리 |
|---|---|
| `insert_column_break_native` 동시 광고 | 본 sub 범위 외 — Sub-9 검토 자리 |
| 모델 가이드 (SKILL.md / references) 갱신 | 본 sub 광고 안정 다음 별개 사이클 |
| rhwp-studio WASM API 노출 | 별개 사이클 |
| Partial*Style 의 새 키 추가 | 본 sub 는 EditOperation 한 자리만 늘림 — 정합 |

## 위험 자리 검증 (Phase 0 의 가정 vs 실제)

| 위험 | 가정 | 실제 |
|---|---|---|
| `affected_range.after.end = para + 2` off-by-one | exclusive 가정 | ✓ 패치 6 단위 테스트 PASS 로 확인 |
| `DocumentCore::new_empty()` 빈 paragraphs | 직접 호출 필요 | `core_with_text` helper 우회 — apply test PASS |
| `insert_page_break_native` 반환 JSON 모양 | `?` 만으로 충분 | ✓ `apply_edit_op` arm 다른 12 자리와 동일 |
| `op_stash` 새 op 호환 | opaque JSON 정합 | ✓ sqlite schema 변경 없음 — workspace 1487 tests 회귀 0 |
| `recompose_section + paginate_if_needed` 비용 | 측정만 | 본 sub 단위 테스트 0.01s — 작은 문서에선 무시 가능. 큰 문서 측정은 별개 |

## 마무리

Sub-7 v3 의 *Partial*Style ↔ SKILL.md 광고 정합* 자리에 이어 *EditOperation enum 자체에도 광고 갭이 존재* 함을 본 sub 가 확인·해소. 함수가 구현되어 있어도 *외부 REST 진입이 없으면 dead* 인 자리가 패턴화될 가능성 — 이후 sub 에서 *EditOperation enum vs native fn 카탈로그 비교 audit* 한 자리 진행 검토 (별도 작업).

본 sub 의 광고가 *stylish-doc-edit 의 단계 3 (patch-phase.md 재작성)* 의 page_break 자리에 정합. 다음 작업은 *stylish-doc-edit 의 작업 재개* — `feature/jerry-rhwp` 브랜치에서 진행.
