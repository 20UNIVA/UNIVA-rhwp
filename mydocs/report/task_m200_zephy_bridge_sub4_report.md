# Sub-4 결과 보고서 — hwp-doc-patch tool result 에 PatchDiff 노출

## 요약

hwp-doc-patch 가 `applied: "ops"` 만 돌려주던 응답에, *적용 전후 IR 슬라이스* 와 좌표·요약을 묶은 `PatchDiff` 를 항상 포함하도록 확장. 12 개 EditOperation variant 전부에 적용 — 모델이 tool result 만 보고도 *진짜 바뀌었는지* / *어떻게 바뀌었는지* 즉시 확인 가능.

## 응답 변화

### Before

```json
{"ok": true, "result": {"seq": 18, "applied": "ops", "info": null}}
```

→ *applied: "ops"* 가 *실제 변경 발생* 을 보장하지 않음 — 좌표 misalignment 로 no-op 인 경우 구분 불가.

### After

```json
{
  "ok": true,
  "result": {
    "seq": 1, "applied": "ops",
    "info": {"fileId": "...", "sectionCount": 1, "paragraphCount": 1},
    "diff": {
      "op": "insert_text",
      "location": {
        "section": 0,
        "paraStartBefore": 0, "paraEndBefore": 1,
        "paraStartAfter": 0,  "paraEndAfter": 1
      },
      "before": { "doc_meta": {...}, "paragraphs": [{"para": 0, "text": "", ...}], "defaults": {...} },
      "after":  { "doc_meta": {...}, "paragraphs": [{"para": 0, "text": "Hello", ...}], "defaults": {...} },
      "summary": {
        "changed": true,
        "beforeParaCount": 1, "afterParaCount": 1,
        "beforeTextLen": 0,   "afterTextLen": 5
      }
    }
  }
}
```

→ 모델은 `summary.changed` 하나만 확인하면 *적용 여부* 판단, `before`/`after.paragraphs` 비교로 *어떤 텍스트·스타일이 바뀌었는지* 확인.

## 변경 파일

| 파일 | 변경 | 라인 추가 |
|---|---|---|
| [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs) | AffectedRange/ParaRange/CellFocus + affected_range() + 13 tests | ~340 |
| [src/document_core/mod.rs](../../src/document_core/mod.rs) | 신규 타입 re-export | 1 |
| [server/src/ir_compact.rs](../../server/src/ir_compact.rs) | PatchDiff/PatchLocation/PatchSummary + capture_before/after_slice + build_patch_diff + 5 tests | ~280 |
| [server/src/main.rs](../../server/src/main.rs) | WorkbenchResp.diff, apply_op_with_stash 반환 튜플화, 14 분기 갱신 | ~30 |
| [rhwp-studio/e2e/sub4-patch-diff.test.mjs](../../rhwp-studio/e2e/sub4-patch-diff.test.mjs) | 신규 e2e (7 scenarios) | ~155 |

노트북 (`hwp_sub_agent_simulation_ssr.ipynb`) 은 수정 없음 — `r.json()` 결과 body 가 `format_as_sentinel_json` 안에 통째로 직렬화되므로 새 `diff` 필드가 자동 전파.

## 검증

### 단위 테스트

| 모듈 | 기존 | 신규 | 합계 |
|---|---|---|---|
| `rhwp::document_core::commands::edit_op::tests` | 24 | 13 | 37 |
| `rhwp_server::ir_compact::tests` | 46 | 5 | 51 |
| `rhwp_server` 전체 | 59 | 5 | 64 |

13 affected_range 테스트는 12 ops + SplitParagraph/MergeParagraph 까지 포함, *insert/delete 의 범위 확장·축소*, *cell focus carryover*, *MergeParagraph 의 saturating_sub* 까지 다룬다.

5 PatchDiff 테스트는 *changed 거짓/참 판정*, *location 좌표 보존*, *camelCase 직렬화*, *capture_before_slice 가 build_compact_ir_slice 와 일치* 를 확인.

### e2e 테스트

`node e2e/sub4-patch-diff.test.mjs` 7/7 pass:

```
  ✓ insert_text 응답에 diff 가 채워지고 changed=true
  ✓ replace_runs 응답 diff.op 가 정확히 매핑
  ✓ insert_paragraph 응답 location.paraEndAfter 가 늘어남
  ✓ insert_table → replace_cell_runs 로 cell focus 동작
  ✓ delete_range 응답 paraEndAfter 가 줄어듦
  ✓ complete 응답에는 diff 가 없음 (None)
  ✓ 알 수 없는 action (passthrough) 응답에도 diff 없음
```

### 회귀

- `sub2-replace-runs` / `sub2-canvas-insert-text` / `sub2-replace-cell-runs` — pass
- `sub3-ir-compact` — pass (sub-3 v2 의 compact 직렬화 영향 없음)
- `ws-bridge` / `sub2-audit-diff-ir-slice` / `sub2-partial-update` — pass

실 호출 응답 검증 (curl):

```
POST /sessions/sub4-smoke/workbench {"action":"insert_text", payload:{section:0, para:0, offset:0, text:"Hello"}}
→ {"seq":1, "applied":"ops", "diff":{"op":"insert_text", "summary":{"changed":true, ...}}}
```

응답 구조가 e2e 와 동일하게 채워짐을 확인.

## 효과

1. *no-op 감지*: 같은 runs 로 replace_runs 호출 시 `summary.changed: false` — 좌표가 의도와 달랐는지 즉시 파악.
2. *적용 위치 검증*: cell variants 응답에 `location.cell.{tablePara, row, col, cellIdx}` 가 채워져 *어느 셀에 들어갔는지* 명확. `cellIdx` 는 서버 사전 변환 결과를 그대로 노출.
3. *길이 변화 추적*: `summary.{beforeTextLen, afterTextLen, beforeParaCount, afterParaCount}` — insert/delete 가 의도된 만큼 진행됐는지 한 숫자로 확인.
4. *모든 액션 일관 처리*: ops 분기 14 곳 (insert_text, replace_runs, set_paragraph_style, delete_range, insert_paragraph, delete_element, insert_table, set_cell_style, merge_cells, replace_cell_runs, insert_text_in_cell, delete_range_in_cell, complete, passthrough) — ops 인 12 개는 항상 diff 채움, complete/passthrough 는 `skip_serializing_if` 로 키 자체 누락.

## 트레이드오프

*응답 크기*: 셀 편집 시 표 전체 compact IR 이 두 번 (before+after) 실림. Sub-3 v2 의 압축 (cell flat entry 제거 + paragraphs 압축 + structural key omit) 덕에 단일 표 1-5KB 수준. 모델 컨텍스트 부담은 일반 응답 ~수백 byte 보다 10-50× 증가하지만, *검증 가능성* 확보를 위한 의도된 비용.

*defaults 박스 비교 제외*: 셀 한 칸 변경은 문서 전체 mode 를 안 바꿔서 defaults 가 동일. `changed` 판정은 `paragraphs` JSON 비교만 — 정확도 100%.

## 후속 작업 후보

- *cell 단위 diff 만 추출*: 표 변경 시 표 전체 IR 대신 `location.cell` 좌표가 가리키는 셀 한 칸만 잘라 보내면 응답이 더 작아짐 (현재 1-5KB → 수백 byte).
- *PatchDiff 환경변수 옵트아웃*: 자동화 시나리오에서 응답 크기가 부담이면 `RHWP_PATCH_DIFF=0` 으로 끄는 옵션.
- *audit endpoint 연동*: 과거 ops 도 diff 형태로 재생 가능하게 — sqlite op_stash 의 before_blob 을 활용한 *PatchDiff 재구성* endpoint.
