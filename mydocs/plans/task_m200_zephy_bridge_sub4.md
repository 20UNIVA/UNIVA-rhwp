# Sub-4 — hwp-doc-patch tool result 에 PatchDiff 노출

## 배경

Sub-2 에서 12 개 EditOperation variants 가 서버 진입점 (`POST /sessions/{id}/workbench`) 으로 통합되었고, 응답은 `{"seq", "applied": "ops", "info": ...}` 였다. *적용 여부 자체* 만 알려 줄 뿐, *어떤 셀/문단이 어떻게 바뀌었는지* 는 응답에서 알 수 없었다.

실제 사고 사례:
```
[exec Bash result]
{"ok": true, "result": {"seq": 18, "applied": "ops", "info": null}}
```
모델은 *applied: "ops"* 만 보고 작업이 성공했다고 판단했지만, 좌표 misalignment 로 실제 변경은 일어나지 않은 상황이 발생했다. *적용 전후 IR 슬라이스 (before/after)* 를 응답에 함께 실어, 모델이 tool result 만 보고도 *실제로 바뀌었는지* / *어떻게 바뀌었는지* 확인할 수 있게 한다.

## 목표

12 개 ops variant (insert_text, delete_text, split_paragraph, merge_paragraph, replace_runs, set_paragraph_style, delete_range, insert_paragraph, delete_element, insert_table, set_cell_style, merge_cells, replace_cell_runs, insert_text_in_cell, delete_range_in_cell) 전부에 대해:

1. *적용 전* 영향 범위 (paragraph 또는 셀) 의 compact IR 슬라이스를 캡처
2. *적용 후* 같은 영향 범위의 compact IR 슬라이스를 캡처 (insert/delete 는 범위가 확장/축소)
3. before/after 와 좌표·요약을 묶어 `WorkbenchResp.diff` 로 응답에 포함
4. complete / passthrough / 알 수 없는 action 에는 diff 가 *없음* (None) 으로 직렬화에서 누락

## 비목표

- 새 endpoint 추가 (기존 `/workbench` 응답에만 끼움)
- 클라이언트(rhwp-studio) UI 변경
- 노트북 `_handle_*` 수정 (응답 body 통째로 sentinel JSON 안에 직렬화되므로 자동 전파)

## 설계

### AffectedRange — 영향 범위 추출

```rust
pub struct AffectedRange {
    pub section: usize,
    pub before: ParaRange,           // [start, end)
    pub after: ParaRange,            // insert 는 end 증가, delete 는 end 축소
    pub cell: Option<CellFocus>,     // 셀 단위 편집이면 채워짐
}
pub struct CellFocus {
    pub table_para: usize,
    pub row: usize, pub col: usize,
    pub cell_idx: Option<usize>,
    pub cell_para: Option<usize>,
}
```

`EditOperation::affected_range()` 메서드가 variant 별로 매핑한다.

| Variant | before | after | cell |
|---|---|---|---|
| InsertText / DeleteText | [para..para+1) | [para..para+1) | — |
| SplitParagraph | [para..para+1) | [para..para+2) | — |
| MergeParagraph | [para-1..para+1) | [para-1..para) | — |
| ReplaceRuns / SetParagraphStyle | [para..para+1) | [para..para+1) | — |
| DeleteRange | [para_start..para_end+1) | [para_start..para_start+1) | — |
| InsertParagraph | [after_para..after_para+1) | [after_para..after_para+1+count) | — |
| DeleteElement::Paragraph | [para..para+1) | [para..para) (empty) | — |
| DeleteElement::Table | [para..para+1) | [para..para+1) | — |
| InsertTable | [after..after+1) | [after..after+2) | — |
| SetCellStyle / MergeCells / ReplaceCellRuns / InsertTextInCell / DeleteRangeInCell | [table_para..table_para+1) | [table_para..table_para+1) | Some(...) |

### PatchDiff — 응답 페이로드

```rust
pub struct PatchDiff {
    pub op: String,                  // "replace_cell_runs" 등 EditOperation 태그
    pub location: PatchLocation,     // section + para 범위 + cell focus
    pub before: CompactIrSlice,
    pub after: CompactIrSlice,
    pub summary: PatchSummary,
}
pub struct PatchSummary {
    pub changed: bool,
    pub before_para_count: usize,
    pub after_para_count: usize,
    pub before_text_len: usize,
    pub after_text_len: usize,
}
```

- `changed`: paragraphs JSON 직렬 비교. false 면 *no-op* — 좌표/payload 가 실제 데이터를 못 바꾼 경우.
- `before_text_len` / `after_text_len`: 표 셀 내부까지 재귀적으로 글자 수 합산.
- camelCase 직렬화 — 모델이 응답 JSON 에서 키를 일관되게 읽음.

### 캡처 흐름 (apply_op_with_stash)

```
1. range = op.affected_range()
2. (export_hwpx_native + capture_before_slice) — 같은 lock 안에서
3. apply_edit_op(&op)
4. capture_after_slice
5. append_op_stash (snapshot 영속)
6. broadcast ServerEvent::Ops
7. build_patch_diff → Option<PatchDiff>
8. return (seq, Some(diff))
```

### Response 확장

```rust
struct WorkbenchResp {
    seq: i64,
    applied: String,
    info: Option<SessionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<PatchDiff>,
}
```

complete / passthrough / 알 수 없는 action 분기는 `diff: None` — `skip_serializing_if` 로 키 자체가 출력에서 빠진다.

## 변경 파일

- [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs) — AffectedRange / ParaRange / CellFocus 타입 + `affected_range()` impl + 13 unit tests
- [src/document_core/mod.rs](../../src/document_core/mod.rs) — re-export
- [server/src/ir_compact.rs](../../server/src/ir_compact.rs) — PatchDiff / PatchLocation / PatchSummary + `capture_before_slice` / `capture_after_slice` / `build_patch_diff` + 5 unit tests
- [server/src/main.rs](../../server/src/main.rs) — `WorkbenchResp.diff` 필드, `apply_op_with_stash` 반환 튜플화, 12 ops 분기 + complete + passthrough 응답 빌더 갱신
- [rhwp-studio/e2e/sub4-patch-diff.test.mjs](../../rhwp-studio/e2e/sub4-patch-diff.test.mjs) — e2e (7 scenarios)

## 검증

### 단위 테스트

- `cargo test --lib document_core::commands::edit_op` — 37 tests (기존 24 + 신규 13) pass
- `cd server && cargo test ir_compact::` — 51 tests (기존 46 + 신규 5) pass
- `cd server && cargo test` — 64 tests pass

### e2e 테스트

`sub4-patch-diff.test.mjs` 7/7 pass:

1. insert_text → diff.summary.changed=true, afterTextLen > beforeTextLen
2. replace_runs → diff.op="replace_runs", changed=true
3. insert_paragraph (count=2) → location.paraEndAfter=3
4. insert_table → replace_cell_runs (cell focus, cellIdx 서버 자동 변환 검증)
5. delete_range → location.paraEndAfter < paraEndBefore, afterParaCount ≤ beforeParaCount
6. complete → diff 없음
7. passthrough → diff 없음

### 회귀

- sub2-replace-runs / sub2-canvas-insert-text / sub2-replace-cell-runs / sub3-ir-compact / ws-bridge / sub2-audit-diff-ir-slice / sub2-partial-update — 전부 pass
- 실 응답 sample (insert_text "Hello"): diff.op="insert_text", location.section=0, before.paragraphs[0].text="", summary 채워짐.

## 트레이드오프

*응답 크기*: 표 셀 편집 시 표 전체 IR (compact 형식) 이 before/after 두 번 실린다. 큰 표(예: 100 셀) 는 응답이 수 KB 까지 늘 수 있음. *그러나* Sub-3 v2 의 cell flat entry 제거 + 셀 paragraphs 압축 덕에 대부분 1-5KB 범위에 머문다. 옵트인 환경변수로 끄는 옵션은 *불필요* — 모델이 검증할 수 없으면 의미가 없는 응답이라는 판단.

*no-op 감지*: paragraphs JSON 직렬 비교 — 같은 runs 로 replace_runs 호출 시 changed=false 로 정확히 잡힌다. defaults 박스는 비교 대상에서 빠진다 (문서 전체 mode 기반이라 셀 한 칸 변경으로 안 바뀜).
