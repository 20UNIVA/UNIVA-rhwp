# Task #m600-29 최종 결과 보고서 — 이중 표 (nested table) cell 편집 op 추가

## 사이클 요약

기존 cell 편집 op 4종 (`ReplaceCellRuns`·`InsertTextInCell`·`DeleteRangeInCell`·`SetCellStyle`) 은 좌표 `(section, table_para, row, col, cell_para)` 로 *단일 계층* 자료만 표현. 표 안에 표가 들어간 *이중 표* 의 안쪽 cell 을 가리킬 방법이 없음. `CellPath` 자료 + `ReplaceCellRunsAtPath` variant 자료 박아 임의 깊이 nested cell 편집 가능하게 박음.

원본 `1. (★사업중 필독) 사업관리 참조표.hwp` 의 3페이지 nested 3x2 표를 fixture 자료로 활용.

## 변경

### `src/document_core/commands/cell_path.rs` (신설)

- `CellPath` + `CellStep` 자료 — `Vec<CellStep>` 으로 nested 깊이 표현.
- `DocumentCore::get_cell_mut_at_path` — path 따라 cell 의 mutable 참조 박음. iterative 로 borrow checker 정합.
- `DocumentCore::replace_cell_runs_at_path_native` — path 의 cell 의 `paragraphs[cell_para]` 의 text·char_count·char_offsets·char_shapes 자료 재구성 + line_segs.clear() + section.raw_stream 무효화 + paginate_if_needed.
- `rebuild_paragraph_text` 헬퍼 — paragraph 자료 새 text 자료로 자료 재구성.
- 단위 테스트 7개 — depth 1 / depth 2 (6 cell 모두) / empty / invalid / replace depth 1·2.

### `src/document_core/commands/edit_op.rs`

- `EditOperation::ReplaceCellRunsAtPath { section, path, cell_para, runs }` variant 추가.
- `affected_range` 분기 — path 첫 step 의 para 자료를 outer paragraph 자료로 박음.
- `apply_op` 분기 — `replace_cell_runs_at_path_native` 호출.
- `apply_inverse_edit_op` 분기 — snapshot stash 자료 (다른 cell variant 와 동일 규약).

### `src/document_core/commands/mod.rs`

`mod commands` → `pub mod commands` 자체 변경 (외부 module 가 `cell_path::CellPath` 자료 접근 가능하게).

### `rhwp-server/src/main.rs`

`workbench` action `"replace_cell_runs_at_path"` 라우트 박음. 페이로드 형식:

```json
{
  "action": "replace_cell_runs_at_path",
  "payload": {
    "section": 0,
    "path": {"steps": [
      {"para": 4, "ctrl_idx": 0, "row": 0, "col": 0},
      {"para": 8, "ctrl_idx": 0, "row": 1, "col": 1}
    ]},
    "cell_para": 0,
    "runs": [{"text": "NESTED_CELL_EDIT_OK"}]
  }
}
```

`apply_op_with_stash` 자료 그대로 사용 — WS broadcast 자료 자동 박힘.

## 검증

### 코드 회귀

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1498 passed / 0 failed (+7 신규 단위 테스트) |
| `cargo build` + `cargo build -p rhwp-server` | warning 0 |

### 자동 e2e — 원본 hwp + path depth 2 편집

```
원본 hwp → sim 세션 → workbench replace_cell_runs_at_path
  (path=[step{para=4,ctrl=0,row=0,col=0}, step{para=8,ctrl=0,row=1,col=1}],
   cell_para=0, runs=[{text:"NESTED_CELL_EDIT_OK"}])
→ export hwp → IR dump
```

결과:
- nested cell (1, 1) 의 paragraphs.len() = 8 (원본 보존)
- paragraphs[0].text = "NESTED_CELL_EDIT_OK" (변경 자료 반영)
- paragraphs[1..7] = 원본 자료 그대로

### WebSocket 전파

`apply_op_with_stash` 자체 자체 자체 `ServerEvent::Ops { seq, ops, origin_client_id }` 자료 broadcast. JSON 직렬화 자체 자체 자체 `ReplaceCellRunsAtPath` variant 자료 그대로 흘러감.

### WASM 적용

rhwp-studio WASM 자체 자체 자체 rhwp 라이브러리 빌드 자체 자체 자체 동일 `Document` IR 자체 자체. nested table 자체 자체 자체 자체 자체 cycle 28 자체 자체 시각 회복 자체 자체. 동일 `EditOperation::ReplaceCellRunsAtPath` 자체 자체 자체 자체 자체 *WASM 자체 자체 자체 자체* 자체 자체 자체 자체 자체 자체.

UI 자체 자체 (nested cell click → path 자료 박는 자체 자체) 자체 자체 follow-up cycle.

## 범위 외 (follow-up)

- `InsertTextInCellAtPath` / `DeleteRangeInCellAtPath` / `SetCellStyleAtPath` variant 자체 자체.
- WASM 자체 자체 nested cell click 자체 자체 자체 path 자료 박는 자체 자체.
- 기존 cell 변종 4종 자체 자체 자체 자체 자체 path variant 자체 자체 마이그레이션 (호환 위해 별도).
