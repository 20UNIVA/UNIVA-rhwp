# Sub-4 v2 — PatchDiff 응답 크기 압축 (셀 단위 추출)

## 배경

Sub-4 v1 은 셀 한 칸을 편집해도 응답에 *표 전체 IR* 이 before/after 두 번 실려 표 크기에 비례해 응답이 폭증. 사용자 보고: *셀 하나 바꿨는데 6만 자*.

원인: `PatchDiff.before` / `.after` 가 `CompactIrSlice` 였고, 표 paragraph 가 영향 범위면 표 전체 (모든 셀 + 모든 셀 안 paragraphs) 가 두 번 직렬화.

## 해결

`PatchDiff.before` / `.after` 의 타입을 *영향받은 최소 단위* 만 담는 `PatchTarget` 으로 교체.

```rust
#[derive(Serialize)]
#[serde(untagged)]
pub enum PatchTarget {
    Cell { cell: serde_json::Value },              // 셀 편집 — 셀 한 칸만
    Paragraphs { paragraphs: Vec<serde_json::Value> },  // 본문 편집 또는 표 전체 변형
}
```

- 셀 편집 (SetCellStyle / ReplaceCellRuns / InsertTextInCell / DeleteRangeInCell) — 표 paragraph 의 `cells[cell_idx]` *한 칸만* 추출해 `Cell` variant 로
- MergeCells (단일 cell_idx 없음) 와 InsertTable / DeleteElement::Table / 본문 paragraph 편집 — `Paragraphs` variant
- `doc_meta` / `defaults` 제거 — `location` 에 sec/para 좌표가 이미 있음

## 응답 크기 측정

10×10 표 (100셀) 의 셀 한 칸 변경 응답:

| Sub-4 v1 (추정) | Sub-4 v2 (실측) |
|---|---|
| 수만 byte (표 전체 IR × 2 = 100 셀 × 2 직렬화) | **953 byte** |

e2e 검증: `JSON.stringify(body).length < 2000` — 표 크기와 무관하게 1KB 안팎 유지.

## 응답 예시 — 셀 편집

```json
{
  "seq": 2, "applied": "ops",
  "diff": {
    "op": "replace_cell_runs",
    "location": {
      "section": 0,
      "paraStartBefore": 1, "paraEndBefore": 2,
      "paraStartAfter": 1,  "paraEndAfter": 2,
      "cell": {
        "tablePara": 1, "row": 3, "col": 5,
        "cellIdx": 35, "cellPara": 0
      }
    },
    "before": {
      "cell": {
        "row": 3, "col": 5,
        "paragraphs": [{"cell_locator": {...}, "para": -1, "style": {"align": "justify"}, "text": ""}],
        "style": {"border": {...}, "height": 282, "vertical-align": "middle", "width": 4195}
      }
    },
    "after":  {
      "cell": {
        "row": 3, "col": 5,
        "paragraphs": [{"cell_locator": {...}, "para": -1, "runs": [{"text": "변경값"}], ...}],
        "style": {...}
      }
    },
    "summary": {"changed": true, "beforeTextLen": 0, "afterTextLen": 3, "beforeParaCount": 1, "afterParaCount": 1}
  }
}
```

→ 다른 셀 99개의 정보는 응답에 *전혀* 없음. 모델은 변경된 셀의 row/col 과 paragraphs 내용만 본다.

## 응답 예시 — 본문 paragraph 편집

```json
{
  "diff": {
    "op": "insert_text",
    "location": {"section": 0, "paraStartBefore": 0, "paraEndBefore": 1, ...},
    "before": { "paragraphs": [{"para": 0, "text": "", ...}] },
    "after":  { "paragraphs": [{"para": 0, "runs": [{"text": "안녕"}], ...}] },
    "summary": {"changed": true, "beforeTextLen": 0, "afterTextLen": 2}
  }
}
```

`paragraphs` 키로 본문 편집임을 모델이 구분. untagged enum 직렬화로 `cell` vs `paragraphs` 키만으로 두 variant 식별.

## 변경 파일

| 파일 | 변경 |
|---|---|
| [server/src/ir_compact.rs](../../server/src/ir_compact.rs) | PatchTarget enum, extract_compact_cell, slice_to_target, capture_before/after_target, build_patch_diff 시그니처 변경. text_len 계산 PatchTarget 기반 재작성 |
| [server/src/main.rs](../../server/src/main.rs) | apply_op_with_stash 가 PatchTarget 사용 |
| [rhwp-studio/e2e/sub4-patch-diff.test.mjs](../../rhwp-studio/e2e/sub4-patch-diff.test.mjs) | before/after 형식 검증, 큰 표 응답 크기 검증 (8 scenarios) |

`edit_op.rs` / `document_core/mod.rs` 는 변경 없음 (AffectedRange / CellFocus 그대로).

## 검증

### 단위 테스트

| 모듈 | tests |
|---|---|
| `rhwp_server::ir_compact::tests` | 55 (기존 51 → 신규 4 추가, 기존 5 갱신) — 전부 pass |
| `rhwp_server` 전체 | 69 pass |

신규 테스트:
- `patch_diff_cell_target_text_len_counts_cell_paragraphs` — 셀 target text_len 합산 정확성
- `patch_diff_paragraph_target_text_len_supports_compact_text_field` — Sub-3 v2 단일 run 축약 형식 지원
- `extract_compact_cell_returns_cell_by_index` — cell_idx 추출 정확성 + 범위 밖/None 처리
- `slice_to_target_extracts_cell_when_cell_idx_present`, `slice_to_target_falls_back_to_paragraphs_when_no_cell_focus`, `slice_to_target_falls_back_when_cell_idx_none` — 변환 분기 검증

### e2e

`sub4-patch-diff.test.mjs` 8/8 pass:

```
✓ insert_text 응답에 diff 가 채워지고 paragraphs target 사용
✓ replace_runs 응답 diff.op 가 정확히 매핑
✓ insert_paragraph 응답 location.paraEndAfter 가 늘어남
✓ insert_table → replace_cell_runs 가 cell target 으로 압축됨
✓ 큰 표 셀 1개 편집 응답 크기가 표 크기와 무관하게 작음
✓ delete_range 응답 paraEndAfter 가 줄어듦
✓ complete 응답에는 diff 가 없음 (None)
✓ 알 수 없는 action (passthrough) 응답에도 diff 없음
```

회귀: sub2-replace-cell-runs / sub2-canvas-insert-text / sub3-ir-compact 모두 pass.

## 효과

1. *응답 크기 표 크기와 무관*: 셀 1개 변경 → 표 100셀이든 1000셀이든 1KB 미만 (셀 한 칸 + location + summary).
2. *모델 가독성 향상*: 다른 셀 99개 정보 없음 → 모델이 "어디가 어떻게 바뀌었는지" 만 본다. 컨텍스트 부담 최소화.
3. *Sub-4 v1 의 모든 검증 능력 보존*: `summary.changed` / `summary.beforeTextLen` / `location.cell.{row, col, cellIdx}` 그대로 — 모델이 적용 여부 / 위치 / 길이 변화 확인 가능.
