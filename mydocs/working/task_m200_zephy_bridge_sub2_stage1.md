# Task #zephy-bridge Sub-2 Stage 1 — 6 Phase 결과

작성일 2026-06-07. 본 보고서는 [task_m200_zephy_bridge_sub2_impl.md](../plans/task_m200_zephy_bridge_sub2_impl.md) 의
6 Phase ~50 task 에 대한 *단계별 통과·실패 기록*.

## Phase 결과 표

| Phase | 범위 | 결과 | 주요 commit |
|---|---|---|---|
| 2a | rhwp 본체 native 2 신설 (`replace_runs_native`·`replace_cell_runs_native`) + 대응 WASM export 2 | ✅ rhwp 1436 lib test PASS, clippy 0 warning | `25ab5351` ~ `15af0db5` |
| 2b | `EditOperation` 본문 7 (ReplaceRuns·SetParagraphStyle·DeleteRange·InsertParagraph·DeleteElement·InsertTable + Partial 타입·RunSpec·ElementType) | ✅ edit_op 누적 unit test 통과 | `a0f48833` ~ `058d6738` |
| 2c | `EditOperation` 셀 5 (SetCellStyle·MergeCells·ReplaceCellRuns·InsertTextInCell·DeleteRangeInCell) + `find_cell_idx` helper | ✅ edit_op 25 unit test PASS (`test_op_apply_equals_direct_native` 포함) | `dbe630b7` ~ `91d75716` |
| 2d | `events.rs` `rename_all` snake_case + `Complete`/`SnapshotRestored` variant + sqlite `op_stash`·`final_snapshots` 테이블 + workbench 11 arms + `apply_op_with_stash` helper | ✅ server 13 integration test PASS, clippy 0 warning | `c013196b` ~ `31bc7560` |
| 2e | `POST /undo` + `GET /audit`·`/diff`·`/ir-slice` + workbench `complete` arm + `ServerEvent::Complete` 발행 | ✅ server 13 test 유지 | `4672d02b` ~ `afdae547` |
| 2f.0 | `findCellIdx` WASM export 보강 — 셀 4 액션의 클라 fallback 제거 | ✅ wasm 빌드 + npm build 통과 | `f3e96478` ~ `b04fc48a` |
| 2f.1-3 | 클라 `wasm-bridge` wrapper 5 (insertParagraph·deleteParagraph·replaceRuns·replaceCellRuns·findCellIdx) + `onServerEvent` ops 11 case + `SnapshotRestored`/`Complete` 핸들러 | ✅ npm build 통과 | `af7447ab` ~ `4ffbb79f` |
| 2f.4 | 노트북 SSR 라우터의 액션 정규화 매핑 점검 + 부분 업데이트 시연 cell 추가 (cell 7) | ✅ (작업 공간 루트는 git 외부) | (untracked) |
| 2f.5 | e2e 공통 helper (`sub2-helpers.mjs`) + 서버 가동 스크립트 (`sub2-server.sh`) | ✅ | `a3ca63f8` |
| 2f.6-16 | e2e 14 작성 — 11 액션 e2e + undo + audit/diff/ir-slice + partial update | ✅ 14 commit + fixup 1 | `b2797299` ~ `7f83c610` + `3891b33e` |
| 2f.17 | 종단 회귀 검증 — 14 신규 e2e 실제 실행 + Sub-1 `ws-bridge.test.mjs` 회귀 0 | ✅ 15/15 PASS + 5 fixup | `4a0d57b5` |
| 2f.18 | stage2 + 최종 보고서 작성 | (본 commit) | |

## 자동 검증 결과

### rhwp 본체 `cargo test --lib`

```
test serializer::cfb_writer::tests::test_serialize_after_edit ... ok
test wasm_api::tests::test_task76_background_image_outside_body_clip ... ok
test serializer::cfb_writer::tests::test_serialize_after_edit_roundtrip ... ok
test wasm_api::tests::test_roundtrip_all_controls ... ok
test document_core::commands::text_editing::tests::test_page_break_with_tight_line_spacing ... ok
test document_core::commands::text_editing::tests::test_page_boundary_with_incremental_spacing_increase ... ok
test serializer::cfb_writer::tests::test_serialize_after_edit_roundtrip ... ok

test result: ok. 1436 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 50.43s
```

### server `cargo test`

```
test events::tests::server_event_snapshot_restored_serializes_with_snake_case ... ok
test events::tests::server_event_complete_serializes_with_snake_case ... ok
test events::tests::client_message_deserializes_from_json ... ok
test events::tests::server_event_json_has_kind_tag ... ok
test events::tests::publish_delivers_to_subscriber ... ok
test store::tests::test_load_missing ... ok
test store::tests::test_create_and_load ... ok
test store::tests::test_snapshot_supersedes_base ... ok
test store::tests::test_op_stash_append_and_pop ... ok
test store::tests::test_op_stash_list_range ... ok
test store::tests::test_op_stash_100_entry_limit_per_session ... ok
test events::tests::different_file_ids_are_isolated ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

기존 Sub-1 의 7 (events 4 + store 3) 에서 events 2 (Complete·SnapshotRestored serialize) + store 3 (`op_stash` append/pop · list_range · 100 entry 제한) 가 *Sub-2 신설* 분.

### `edit_op` 단위 테스트 (rhwp 본체)

```
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1417 filtered out; finished in 0.03s
```

Sub-1 시점 4 (`InsertText`·`DeleteText`·`SplitParagraph`·`MergeParagraph` + roundtrip) 에서 *21 신설* — variant 별 apply 검증 + `test_op_apply_equals_direct_native` (12 variant 가 native 직접 호출과 *비트 단위 동일* 검증).

### clippy

rhwp 본체:
```
Checking rhwp v0.7.13 (...)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.40s
```

server:
```
Checking rhwp v0.7.13 (...)
Checking rhwp-server v0.1.0 (.../server)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.36s
```

`-D warnings` 로 *경고 0건* — 본 spec 범위의 모든 신규 코드가 lint clean.

### WASM 빌드

```
Finished `release` profile [optimized] target(s) in 0.08s
```

`replaceRuns`·`replaceCellRuns`·`findCellIdx` 3 신규 export 가 `wasm32-unknown-unknown` 타깃으로 빌드 통과.

### npm build (`rhwp-studio`)

```
✓ built in 295ms

PWA v1.3.0
mode      generateSW
precache  53 entries (23539.42 KiB)
files generated
  dist/sw.js
  dist/workbox-dcde9eb3.js
```

`main.ts` 의 `onServerEvent` ops 11 case + Complete/SnapshotRestored 핸들러, `wasm-bridge.ts` 의 wrapper 5, `session-client.ts` 의 `EditOpJson` 21 필드·`ServerEvent` 2 kind 추가가 TypeScript 컴파일 통과.

### e2e — 15/15 PASS

신규 14 (`sub2-*.test.mjs`):

| # | 파일 | 검증 시나리오 |
|---|---|---|
| 1 | `sub2-replace-runs.test.mjs` | runs 통째 교체 + char shape 적용 |
| 2 | `sub2-set-paragraph-style.test.mjs` | alignment·line_spacing 변경 |
| 3 | `sub2-delete-range.test.mjs` | 한 문단 내 char 범위 삭제 |
| 4 | `sub2-insert-paragraph.test.mjs` | 신규 문단 1개 삽입 |
| 5 | `sub2-delete-element.test.mjs` | 표 컨트롤 삭제 |
| 6 | `sub2-insert-table.test.mjs` | 2x2 표 삽입 |
| 7 | `sub2-set-cell-style.test.mjs` | 셀 배경색 변경 |
| 8 | `sub2-merge-cells.test.mjs` | 2x2 셀 병합 |
| 9 | `sub2-replace-cell-runs.test.mjs` | 셀 runs 교체 |
| 10 | `sub2-insert-text-in-cell.test.mjs` | 셀 내부 텍스트 삽입 |
| 11 | `sub2-delete-range-in-cell.test.mjs` | 셀 내부 범위 삭제 |
| 12 | `sub2-undo.test.mjs` | undo → SnapshotRestored broadcast → IR 원복 |
| 13 | `sub2-audit-diff-ir-slice.test.mjs` | 3 신규 GET endpoint 응답 검증 |
| 14 | `sub2-partial-update.test.mjs` | 옵셔널 키 누락 시 *현재 값 유지* |

회귀 1 (`ws-bridge.test.mjs`, Sub-1 유산) — 양방향 WS bridge 검증 그대로 통과.

## 수동 검증 (사용자 영역)

자동화 어려운 시나리오는 사용자 시연으로:

1. **LLM 실제 호출 → 브라우저 시각 반영** — 새 노트북 cell 0~7 순차 실행. cell 6 의 LLM 이 SKILL.md 의 12 액션 중 임의를 호출. 브라우저 화면이 액션마다 실시간 반영.
2. **부분 업데이트 시연** — cell 7 에서 `set_paragraph_style {alignment: 'right'}` 만 보내고 다른 서식 (line_spacing 등) 이 *현재 값 유지* 됨을 시각 확인.
3. **Undo 시연** — `POST /sessions/<id>/undo` 호출 (curl 또는 fetch) → `ServerEvent::SnapshotRestored` broadcast → 클라가 `wasm.loadDocument(snapshot_base64)` 로 통째 교체 → 화면 원복.

### 노트북 라우터 분기 — `get-ir-slice`

cell 3 의 `run_bash_command` 가 *모든 액션을 POST /workbench 로 일괄 라우팅* 했으나, 서버는 `get_ir_slice` arm 이 없어 passthrough 만 동작 — *IR 슬라이스 결과를 응답 body 로 반환하지 않음*. 2026-06-07 노트북 cell 3 정정으로 `action == 'get_ir_slice'` 만 `GET /sessions/<id>/ir-slice?<sec/para_start/para_end/mode>` 로 분기. 나머지 11 액션은 기존 POST /workbench 그대로. LLM 이 sentinel JSON 형태로 슬라이스 결과를 수신한다. *노트북 파일은 작업 공간 루트 (git 외부) 에 위치 — 코드 변경 자체는 본 보고서에만 기록.*

## 알려진 한계 (Sub-3 으로 미룸)

| 항목 | 위치 | 처리 |
|---|---|---|
| ~~`EditOperation::InsertParagraph` 의 *doc-comment ↔ 구현 semantic mismatch*~~ → **결정 2026-06-07: 현재 코드 동작 (Enter 와 동일, after_para 위치에 삽입) 이 의도. doc-comment 만 정정 (commit `44ce5187`). Sub-3 추가 작업 없음.** | edit_op.rs:158 docstring | 해결 |
| `insert_text` 가 `op_stash` 에 적재 안 됨 — Sub-1 의 `ws.rs::handle_client_text` 가 `append_op` 만 호출. Sub-2 신규 12 액션만 `apply_op_with_stash` 로 적재. 따라서 *undo 가 insert_text 를 되돌리지 못함*. | `server/src/ws.rs` (Sub-1 유산) | Sub-3 에서 `insert_text` 도 `op_stash` 적재로 통일 또는 undo 정책 별도 |
| `InsertParagraph::style` 의 *부분 적용 위치* — 옵셔널 style 이 신규 문단 *각각*에 동일하게 적용. count > 1 일 때 *모든 신규 문단에 같은 style*. 의도된 동작인지 사용자 확인 권고. | `edit_op.rs::InsertParagraph::apply` | doc 명확화 |
| `delete_table_control_native` 의 `control_idx` — Sub-2 는 *한 paragraph 에 한 table* 가정 (control_idx=0 고정). 한 paragraph 에 *여러 table* 있는 경우 첫 table 만 삭제. | `edit_op.rs::DeleteElement`, `main.ts::case 'delete_element'` | Sub-3 에서 `ElementType` 에 `control_idx` 추가 또는 자동 검색 정교화 |
| `Paragraph` struct 의 `Serialize` 미구현 — `ir-slice` "raw" mode 가 *paragraph 전체 직렬화* 불가. 현재는 *수동 json 으로 핵심 필드 + 컬렉션 길이* 만 노출. | `server/src/main.rs::ir_slice_handler` | Sub-3 에서 `Paragraph` + 부속 타입에 `Serialize` derive 추가 또는 별도 `RawParagraph` view |
| `Mutex::lock().unwrap()` poisoning 위험 — Sub-1 유산. events.rs, main.rs, ws.rs 모두. | 전반 | Sub-3 에서 `parking_lot` 도입 또는 `unwrap_or_else(\|e\| e.into_inner())` |
| sqlite write 실패 시 코어 IR-sqlite 자동 회복 미구현 — workbench arm 이 `apply_edit_op` 성공 후 sqlite write 실패하면 *코어는 변경됨 + sqlite 는 미반영* 의 split-brain. | `server/src/main.rs::apply_op_with_stash` | Sub-3 write-ahead log 또는 transaction wrap |
| 다중 사용자 동시 편집 시 *사용자별 undo stack 분리* — 본 spec 은 *세션당 전역 undo stack* (sqlite op_stash). | sqlite `op_stash` | Sub-3 |
| snapshot binary delta 압축·LRU — 마지막 100 entry 단순 정책. | sqlite `op_stash`, `save_final_snapshot` | Sub-3 |
| `rhwp-studio` 워크벤치 UI 패널 — Sub-2 는 *노트북 호출만*. | (없음) | Sub-3 |
| `complete` 액션의 UI 표시 통합 — 현재 `console.log` 만. | `rhwp-studio/src/main.ts` | Sub-3 |

이 항목들은 [report/task_m200_zephy_bridge_sub2_report.md](../report/task_m200_zephy_bridge_sub2_report.md) 에서 *Sub-3 진입 전 처리 권고* 로 다시 정리.

## 다음 단계

- 사용자 수동 시연 통과 후 Sub-2 종료.
- Sub-3: 위 알려진 한계 항목 처리. InsertParagraph semantic 정정이 *최우선 처리 권고*.
