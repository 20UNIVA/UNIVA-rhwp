# Sub-4 v3 — 빈 paragraph 의 char_shape 가 응답에 드러나도록

## 배경

사용자가 사례 보고: 빨간색 char_shape (id=36) 가 묶인 *빈 셀* 에 `replace_cell_runs` 로 텍스트 "10" 을 넣었더니 결과가 빨간색. before 응답에는 "검은색" 으로 보였는데 after 가 갑자기 빨간색이라 *원인 진단 불가*.

진단:
- audit endpoint 조회 — 모델이 보낸 payload `runs[0].style: {}` (빈 객체). color 명시 *없음*.
- ir-slice raw 조회 — 셀 paragraph 의 `char_shapes: [{"char_shape_id": 36, ...}]`. 셀에 이미 빨간색 char_shape 가 묶여 있었음.
- `replace_cell_runs_native` 가 PartialRunStyle 의 None 필드를 *셀 기존 char_shape 상속* 으로 처리 (의도된 native 동작) — 빨간색 char_shape 36 이 새 run 에 그대로 적용.

근본 원인: `ir_compact::collect_runs` 가 *빈 paragraph (len=0)* 일 때 placeholder run 의 style 을 `RunStyle::default()` 로 무조건 박음. *paragraph 의 첫 char_shape* 를 가져오지 않아 셀에 묶인 색이 응답에서 사라짐. compact 직렬화는 default style 을 omit 하므로 모델은 "이 셀이 빨간색" 임을 알 길이 없었음.

## 수정

`collect_runs(text, len=0, style_at)` 의 placeholder run style 을 `style_at(0)` 호출 결과로 교체.

```rust
// Before
if len == 0 {
    return vec![IrRun { ..., style: RunStyle::default() }];
}

// After
if len == 0 {
    return vec![IrRun { ..., style: style_at(0) }];
}
```

`build_text_paragraph` / `build_cell_paragraph` 두 호출자가 만드는 `style_at` 람다는 *paragraph 의 `char_shape_id_at(offset)`* 으로 char_shape 를 가져와 `char_shape_to_run_style` 로 변환. `offset=0` 도 정상 동작 — 빈 paragraph 라도 *글자가 들어갈 자리의 기본 char_shape* 가 응답에 노출.

## 변경 파일

- [server/src/ir_compact.rs](../../server/src/ir_compact.rs) — `collect_runs` 빈 paragraph 분기 수정, 신규 테스트 `collect_runs_empty_paragraph_takes_style_from_callback`
- [rhwp-studio/e2e/sub4-patch-diff.test.mjs](../../rhwp-studio/e2e/sub4-patch-diff.test.mjs) — 빈 셀의 before.cell.paragraphs[0].runs 존재 검증

## 검증

### 단위 테스트

- `ir_compact` 56 tests pass (55 → 신규 1 추가, 기존 1 호환)
- server 전체 70 pass

### 라이브 응답 비교

같은 흐름 재현 (insert_table → replace_cell_runs → delete_range_in_cell → 빈 상태에서 replace_cell_runs):

**이전 응답** — `before.cell.paragraphs[0]` 에 `runs` 키 *없음*. 모델은 셀 char_shape 를 알 수 없음.

**현재 응답**:
```json
"before": {
  "cell": {
    "col": 0,
    "paragraphs": [{
      "cell_locator": {...},
      "para": -1,
      "runs": [{"style": {"highlight": "#FFFFFF"}, "text": ""}],
      "style": {"align": "justify"}
    }],
    ...
  }
}
```

→ `runs[0].style` 이 응답에 *항상 노출*. 셀에 묶인 char_shape (이번엔 highlight) 가 모델에게 보임. 실제 빨간색 char_shape_id=36 의 경우 `color: "#FF0000"`, `char-spacing: -5`, `char-width: 90`, `highlight: "#FFFFFF"` 가 모두 before 에 노출되어 모델이 *"이 셀에 글자를 넣으면 빨간색이 된다"* 를 미리 알 수 있음.

### e2e

`sub4-patch-diff.test.mjs` 9/9 pass (Sub-4 v3 가드 1건 추가).

회귀: sub2-replace-cell-runs / sub2-canvas-insert-text / sub3-ir-compact 통과.

## 효과

1. *셀 char_shape 가 응답에 노출* — 빈 셀이라도 *어떤 스타일로 글자가 들어갈지* 미리 보임. "before 검은색 → after 빨간색" 같은 *유령 변화* 사라짐.
2. *insert/delete 텍스트의 진짜 변화* 식별 가능 — placeholder run 의 style 이 살아 있으면 모델이 *진짜 색 변경* 과 *셀 char_shape 상속* 을 구분 가능.
3. *데이터 누락 없음* — 빈 paragraph 의 길이 0 invariant 그대로 (`length: 0, text: ""`), style 정보만 풍부해짐.

## 트레이드오프

응답 크기 — 빈 paragraph 라도 default 와 다른 style 키가 1-3개 추가될 수 있음. 셀당 수십 byte 수준이라 영향 미미.
