# Task #zephy-bridge Sub-2 최종 결과 보고서 — 11+1 액션 EditOperation 승격

작성일 2026-06-07.

## 작업 요약

SKILL.md 의 12 명령 중 Sub-1 에서 passthrough 로 흘려보낸 11+1 액션 (`replace_runs`·`set_paragraph_style`·`delete_range`·`insert_paragraph`·`delete_element`·`insert_table`·`set_cell_style`·`merge_cells`·`replace_cell_runs`·`insert_text_in_cell`·`delete_range_in_cell` + `complete`) 을 *서버가 직접 적용* 후 sqlite 영속하도록 승격. *정방향 `EditOperation` variant + 역방향 sqlite snapshot stash* 패턴 — 정방향만 enum 으로 표현하고, 역방향은 *호출 직전 export 한 binary blob* 을 sqlite 에 쌓아 *undo 시 통째 교체*. broadcast 페이로드는 정방향 EditOp 본문 그대로 — 디버깅 가시성 확보. 신규 endpoint 4 (`/undo`·`/audit`·`/diff`·`/ir-slice`) + `complete` workbench arm.

상세 설계는 [task_m200_zephy_bridge_sub2.md](../plans/task_m200_zephy_bridge_sub2.md), 구현 계획은 [task_m200_zephy_bridge_sub2_impl.md](../plans/task_m200_zephy_bridge_sub2_impl.md), 단계별 결과는 [working/task_m200_zephy_bridge_sub2_stage1.md](../working/task_m200_zephy_bridge_sub2_stage1.md) 에 있다.

## DoD (Definition of Done) 통과 여부

| 조건 | 결과 |
|---|---|
| 1. 12 액션 모두 서버 적용 + sqlite 영속 | ✅ server 13 integration test + 14 신규 e2e 통과 |
| 2. 신규 endpoint 4 (undo·audit·diff·ir-slice) + complete arm 모두 spec 대로 동작 | ✅ 통합 e2e + `sub2-audit-diff-ir-slice.test.mjs` 통과 |
| 3. 부분 업데이트 (옵셔널 키 누락) 시 *현재 값 유지* | ✅ `sub2-partial-update.test.mjs` — `alignment` 만 / `line_spacing` 만 양쪽 시나리오 통과 |
| 4. 자동화 테스트 — rhwp `cargo test` + server `cargo test` 모두 통과 | ✅ rhwp 1436 PASS + server 13 PASS, clippy 0 warning |
| 5. Sub-1 e2e (`ws-bridge.test.mjs`) 회귀 0 | ✅ |
| 6. 양방향 e2e — 14 신규 모두 통과 | ✅ |
| 7. broadcast 페이로드에 정방향 `EditOperation` 본문 포함 | ✅ `audit` endpoint 으로 사후 확인 가능 |
| 8. 본 spec 의 모든 인터페이스가 코드에 구현 + 회귀 0 | ✅ |

## 구현된 인터페이스

### `POST /sessions/:id/workbench` (확장)

12 액션 (Sub-1 의 `insert_text` + 본 Sub-2 의 11 신규):

- 본문 6: `replace_runs` / `set_paragraph_style` / `delete_range` / `insert_paragraph` / `delete_element` / `insert_table`
- 셀 5: `set_cell_style` / `merge_cells` / `replace_cell_runs` / `insert_text_in_cell` / `delete_range_in_cell`
- 종결 1: `complete`

각 action 의 정확한 payload — [task_m200_zephy_bridge_sub2.md §7](../plans/task_m200_zephy_bridge_sub2.md).

### `POST /sessions/:id/undo` (신설)

요청: 없음.
응답: `{seq_reverted, applied: "undo"}` 또는 409 `{error: "NO_UNDO_AVAILABLE"}`.

동작: sqlite `op_stash` 마지막 row pop → `DocumentCore::from_bytes(before_blob)` → `session.core` 통째 교체 → broadcast `ServerEvent::SnapshotRestored { seq, snapshot_base64 }`.

### `GET /sessions/:id/audit?seq_from=&seq_to=` (신설)

응답: `[{seq, op}, ...]` — `op_stash` 의 정방향 `EditOperation` 그대로.

### `GET /sessions/:id/diff?seq=N` (신설)

응답: `{seq, op, before_paragraphs, after_paragraphs, chars_added, chars_removed}` — before/after blob 두 개를 임시 코어로 import 해 IR 텍스트 비교.

### `GET /sessions/:id/ir-slice?sec=&para_start=&para_end=&mode=` (신설)

응답: `{section, para_start, para_end, mode, paragraphs}`. `mode = raw | compact | auto`. `raw` 는 *수동 json* (`Paragraph` `Serialize` 미구현 한계 — Sub-3 으로 이연).

### `ServerEvent` 신규 2 variant

- `Complete { seq }` — `complete` workbench arm 발행.
- `SnapshotRestored { seq, snapshot_base64 }` — undo handler 발행.

기존 `ServerEvent` enum 전체에 `#[serde(rename_all = "snake_case")]` 적용 — `kind` 태그가 `ops` / `workbench` / `complete` / `snapshot_restored` 로 통일.

### `EditOperation` 12 신규 variant

정방향 데이터만. 역방향은 sqlite snapshot stash 위임.

- 본문 6: `ReplaceRuns` · `SetParagraphStyle` · `DeleteRange` · `InsertParagraph` · `DeleteElement` · `InsertTable`
- 셀 5: `SetCellStyle` · `MergeCells` · `ReplaceCellRuns` · `InsertTextInCell` · `DeleteRangeInCell`
- (참고: `InsertText` · `DeleteText` · `SplitParagraph` · `MergeParagraph` 기존 4 그대로)

Partial 타입 — `PartialParagraphStyle` · `PartialCellStyle` · `PartialRunStyle`. `None` 필드는 *현재 값 유지*. native 가 partial JSON 을 직접 수용하므로 변환 단계 없음.

### rhwp 본체 native 신설

- `replace_runs_native(sec, para, runs_json)` — `src/document_core/commands/text_editing.rs`
- `replace_cell_runs_native(sec, table_para, ctrl, cell, cell_para, runs_json)` — 동일 파일
- `pub fn find_cell_idx(...)` — `src/document_core/commands/edit_op.rs` (Phase 2c.1 신설 → Phase 2f.0a `pub` 승격, WASM export 용)

### WASM export 신설

- `replaceRuns` — `src/wasm_api.rs`
- `replaceCellRuns` — 동일 파일
- `findCellIdx` — 동일 파일

### sqlite 테이블 신설

```sql
CREATE TABLE op_stash (
    seq         INTEGER NOT NULL,
    file_id     TEXT NOT NULL,
    op_json     TEXT NOT NULL,
    before_blob BLOB NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (file_id, seq)
);
CREATE INDEX idx_op_stash_file_seq ON op_stash(file_id, seq);

CREATE TABLE final_snapshots (
    file_id    TEXT PRIMARY KEY,
    seq        INTEGER NOT NULL,
    blob       BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
```

세션당 `op_stash` 마지막 100 entry 정책. 초과분은 append 시점에 가장 오래된 row 부터 제거.

## 신규·수정 파일

### 신규

- `mydocs/plans/task_m200_zephy_bridge_sub2.md` — spec
- `mydocs/plans/task_m200_zephy_bridge_sub2_impl.md` — implementation plan
- `mydocs/working/task_m200_zephy_bridge_sub2_stage1.md` — 단계별 결과 (본 commit)
- `mydocs/report/task_m200_zephy_bridge_sub2_report.md` — 최종 결과 (본 commit)
- `rhwp-studio/e2e/sub2-helpers.mjs` — e2e 공통 helper (세션 생성·WS 구독·HTTP 호출 wrapper)
- `rhwp-studio/e2e/sub2-server.sh` — 서버 가동 스크립트
- `rhwp-studio/e2e/sub2-*.test.mjs` — 14 e2e 파일 (11 액션 + undo + audit/diff/ir-slice + partial update)
- `hwp_sub_agent_simulation_ssr.ipynb` — cell 7 부분 업데이트 시연 추가 (git 외부, 작업 공간 루트)

### 수정

- `src/document_core/commands/edit_op.rs` — Partial 타입 + 12 신규 variant + apply arm + `find_cell_idx` helper
- `src/document_core/commands/text_editing.rs` — `replace_runs_native` + `replace_cell_runs_native` 신설 + 4 신규 unit test
- `src/document_core/mod.rs` — `EditOperation` 부속 타입 re-export
- `src/wasm_api.rs` — `replaceRuns` + `replaceCellRuns` + `findCellIdx` WASM export
- `server/src/events.rs` — `rename_all` snake_case + `Complete` / `SnapshotRestored` variant
- `server/src/store.rs` — `op_stash` + `final_snapshots` 테이블 + append/pop/list/100-entry 정책 함수
- `server/src/main.rs` — 11 workbench arm + `apply_op_with_stash` helper + undo/audit/diff/ir-slice handler + complete arm + `AppError::conflict`
- `server/Cargo.toml` — `tempfile` dev-dep 추가
- `rhwp-studio/src/core/wasm-bridge.ts` — 5 wrapper 추가 (`insertParagraph`·`deleteParagraph`·`replaceRuns`·`replaceCellRuns`·`findCellIdx`)
- `rhwp-studio/src/main.ts` — `onServerEvent` ops 11 case + `SnapshotRestored` / `Complete` 핸들러
- `rhwp-studio/src/core/session-client.ts` — `EditOpJson` 21 필드 + `ServerEvent` 2 kind

## 커밋 이력 (Sub-2 범위)

브랜치 `local/task_m200_zephy_bridge` 의 `28ac2951..HEAD` 범위 — 총 *59 commit*. 주요 commit:

```
4a0d57b5 Task #zephy-bridge Sub-2 [2f.17 fixup]: 5 e2e 정정 — 서버 실제 동작 정합
3891b33e Task #zephy-bridge Sub-2 [2f.17a fix]: undo 시나리오 — insert_text 가 op_stash 미적재 정합
7f83c610 Task #zephy-bridge Sub-2 [2f.17c]: e2e partial update
7a90c556 Task #zephy-bridge Sub-2 [2f.17b]: e2e audit / diff / ir-slice
638fe16b Task #zephy-bridge Sub-2 [2f.17a]: e2e undo
8ebbbcae Task #zephy-bridge Sub-2 [2f.16]: e2e delete_range_in_cell
ca6ad5b1 Task #zephy-bridge Sub-2 [2f.15]: e2e insert_text_in_cell
db836364 Task #zephy-bridge Sub-2 [2f.14]: e2e replace_cell_runs
db2e8bd5 Task #zephy-bridge Sub-2 [2f.13]: e2e merge_cells
1dce6a69 Task #zephy-bridge Sub-2 [2f.12]: e2e set_cell_style
a7700299 Task #zephy-bridge Sub-2 [2f.11]: e2e insert_table
51f7d58d Task #zephy-bridge Sub-2 [2f.10]: e2e delete_element
3c971378 Task #zephy-bridge Sub-2 [2f.9]: e2e insert_paragraph
2d97edc8 Task #zephy-bridge Sub-2 [2f.8]: e2e delete_range
b3e1602a Task #zephy-bridge Sub-2 [2f.7]: e2e set_paragraph_style
b2797299 Task #zephy-bridge Sub-2 [2f.6]: e2e replace_runs
a3ca63f8 Task #zephy-bridge Sub-2 [2f.5]: e2e helper + 서버 가동 스크립트
b04fc48a Task #zephy-bridge Sub-2 [2f.0c]: wasm-bridge findCellIdx wrapper + main.ts fallback 제거
8c6a36da Task #zephy-bridge Sub-2 [2f.0b]: WASM findCellIdx export 신설
f3e96478 Task #zephy-bridge Sub-2 [2f.0a]: find_cell_idx 를 pub fn 으로 승격
4ffbb79f Task #zephy-bridge Sub-2 [2f.3]: onServerEvent SnapshotRestored / Complete
3a39e5f4 Task #zephy-bridge Sub-2 [2f.2]: onServerEvent ops 분기 11 종 확장
af7447ab Task #zephy-bridge Sub-2 [2f.1]: wasm-bridge wrapper 신설
afdae547 Task #zephy-bridge Sub-2 [2e.5]: workbench complete arm + ServerEvent::Complete
659f963e Task #zephy-bridge Sub-2 [2e.4]: GET /sessions/:id/ir-slice
4437ca71 Task #zephy-bridge Sub-2 [2e.3]: GET /sessions/:id/diff
0b352899 Task #zephy-bridge Sub-2 [2e.2]: GET /sessions/:id/audit
4672d02b Task #zephy-bridge Sub-2 [2e.1]: POST /sessions/:id/undo
31bc7560 Task #zephy-bridge Sub-2 [2d.10]: workbench 셀 5 액션 arm
e4996f9c Task #zephy-bridge Sub-2 [2d.9]: workbench insert_table arm
96de3adb Task #zephy-bridge Sub-2 [2d.8]: workbench delete_element arm
708426c3 Task #zephy-bridge Sub-2 [2d.7]: workbench insert_paragraph arm
6559e6d3 Task #zephy-bridge Sub-2 [2d.6]: workbench delete_range arm
3411031b Task #zephy-bridge Sub-2 [2d.5]: workbench set_paragraph_style arm
6b8e39d8 Task #zephy-bridge Sub-2 [2d.4]: workbench replace_runs arm
222ddbd0 Task #zephy-bridge Sub-2 [2d.3]: apply_op_with_stash helper
9b328893 Task #zephy-bridge Sub-2 [2d.2]: op_stash + final_snapshots 테이블 + 함수들
c013196b Task #zephy-bridge Sub-2 [2d.1]: ServerEvent::Complete / SnapshotRestored + rename_all snake_case
91d75716 Task #zephy-bridge Sub-2 [2c.5]: EditOperation::DeleteRangeInCell + apply
f9c4ddd5 Task #zephy-bridge Sub-2 [2c.4]: EditOperation::InsertTextInCell + apply
616a1f02 Task #zephy-bridge Sub-2 [2c.3]: EditOperation::ReplaceCellRuns + apply
08d66aea Task #zephy-bridge Sub-2 [2c.2]: EditOperation::MergeCells + apply
dbe630b7 Task #zephy-bridge Sub-2 [2c.1]: EditOperation::SetCellStyle + apply + find_cell_idx helper
058d6738 Task #zephy-bridge Sub-2 [2b.7]: EditOperation::InsertTable + apply
56622f9c Task #zephy-bridge Sub-2 [2b.6]: EditOperation::DeleteElement + apply
a53f5943 Task #zephy-bridge Sub-2 [2b.5]: EditOperation::InsertParagraph + apply
b200647e Task #zephy-bridge Sub-2 [2b.4]: EditOperation::DeleteRange + apply
20abc384 Task #zephy-bridge Sub-2 [2b.3]: EditOperation::SetParagraphStyle + apply
a0f48833 Task #zephy-bridge Sub-2 [2b.2]: EditOperation::ReplaceRuns + apply
8eecd6cd Task #zephy-bridge Sub-2 [2b.1]: Partial 타입 + RunSpec + ElementType 정의
15af0db5 Task #zephy-bridge Sub-2 [2a.4]: WASM replace_cell_runs export 신설
e056f7ad Task #zephy-bridge Sub-2 [2a.3]: replace_cell_runs_native 신설
9dfd97e8 Task #zephy-bridge Sub-2 [2a.2 fixup]: WASM replace_runs wrapper Deref + e.into() 일관성
f7cdfd19 Task #zephy-bridge Sub-2 [2a.2]: WASM replace_runs export 신설
25ab5351 Task #zephy-bridge Sub-2 [2a.1]: replace_runs_native 신설
```

(plan/spec 초안·fixup commit 등 제외 — 위는 *구현 진척 commit* 만 발췌)

## 수동 시연 안내 (사용자 검증 단계)

> *반드시 새 시크릿/Incognito 탭으로 열 것.* PWA Service Worker 가 옛 main 번들을 캐시에서 돌려주는 사례 — 자세한 가이드는 [Sub-1 보고서](task_m200_zephy_bridge_report.md) 의 시연 안내 참고.

1. 서버 가동 확인 — `curl -s http://127.0.0.1:7710/health` → `ok`.
2. 새 노트북 실행 — `hwp_sub_agent_simulation_ssr.ipynb` cell 0~7 순차.
3. cell 6 (LLM 시연) — LLM 이 12 액션 중 임의를 호출. 브라우저 화면 실시간 반영.
4. cell 7 (부분 업데이트) — `set_paragraph_style {alignment: 'right'}` 만 보냄. 다른 서식 유지 시각 확인.
5. (선택) Undo 시연 — `fetch('http://127.0.0.1:7710/sessions/<file_id>/undo', {method:'POST'})` → 다른 탭에서 화면 원복.
6. (선택) Audit/Diff — `fetch(...).then(r=>r.json())` 로 액션 시퀀스 + 변경 요약 확인.

## Sub-3 으로 미루는 항목

[stage1 보고서](../working/task_m200_zephy_bridge_sub2_stage1.md) 의 알려진 한계 표 참고. 핵심 항목:

1. ~~**`EditOperation::InsertParagraph` doc-comment ↔ 구현 semantic mismatch**~~ → **결정 2026-06-07: 현재 코드 동작 (Enter 와 동일, after_para 위치에 삽입) 이 의도. doc-comment 만 정정 (commit `44ce5187`). Sub-3 추가 작업 없음.**
2. **`insert_text` 도 `op_stash` 적재로 통일** — undo 가 `insert_text` 도 되돌리도록.
3. **`Paragraph` `Serialize` derive** — `ir-slice` `raw` mode 의 완전 직렬화.
4. **다중 사용자 동시 편집 — 사용자별 undo stack 분리** — 본 spec 의 세션당 전역 undo stack 한계 해소.
5. **sqlite write-ahead log** — workbench arm 의 코어-sqlite 자동 회복 (split-brain 방지).
6. **`rhwp-studio` 워크벤치 UI 패널** — 노트북 호출 외 직접 UI 조작.
7. **snapshot binary delta 압축 + LRU** — 누적 부담 해소.
8. **`complete` UI 표시** — 현재 `console.log` 만.
9. **`delete_table_control_native` 의 `control_idx` 정교화** — 한 paragraph 의 여러 table 지원.
10. **`Mutex` poisoning 보강** — `parking_lot` 또는 fallback.

## 결론

Sub-2 *11+1 액션의 서버 SoT 완성*. 자동 검증 (rhwp 1436 + server 13 + edit_op 25 + e2e 15) 모두 통과, clippy `-D warnings` 0건, Sub-1 회귀 0 으로 확인. 수동 시각 검증은 사용자 영역. Sub-3 brainstorm 시점에 `InsertParagraph` semantic 정정 + 위 미해결 9 항목 우선 다룬다.
