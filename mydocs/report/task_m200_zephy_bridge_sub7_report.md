# Sub-7 — Partial*Style ↔ SKILL.md 광고 정합 (속성 silent drop 사고 해소)

## 배경

사용자 사고 — `set-cell-style` 에 `{"bgcolor": "#FFC0CB"}` 보냈고 HTTP 200 받았지만 *PatchDiff 의 `summary.changed: false`* — 실제 셀 색은 안 바뀜.

원인 — `PartialCellStyle` 에 `bgcolor` 필드 *부재*. serde 가 unknown field 를 `#[serde(default)]` 로 *silent drop* (`deny_unknown_fields` 없음) → 빈 JSON 이 native fn 에 전달 → 변경 0 → before==after → `changed: false`.

audit 결과 *시스템 패턴* 확인 — HIGH 7건 + MEDIUM 3건 총 10건 갭. *광고 (SKILL.md) ↔ 입력 schema (Partial*Style) ↔ native fn ↔ IR 응답* 4단 라인업 불일치.

핵심 사실: *native fn 들은 대부분 이미 지원* — 갭은 *Partial*Style 가 키를 노출 안 한 것*. 본 sub 는 *주로 schema 추가* + *alias 정리* + *deny_unknown_fields* 로 해소.

## 변경

### 1. PartialCellStyle 확장 — [edit_op.rs:45-100](../../src/document_core/commands/edit_op.rs#L45)

```rust
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialCellStyle {
    width, height, vertical_align, border_fill_id, is_header, cell_protect,  // 기존
    bgcolor: Option<String>,           // 신규 — "#FFC0CB"
    border: Option<BorderSpec>,        // 신규 — {all/left/right/top/bottom: {color, width, type}}
    padding_left, padding_right, padding_top, padding_bottom,  // 신규
}

#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BorderSpec { all, left, right, top, bottom: Option<BorderLine> }
pub struct BorderLine { color: Option<String>, width: Option<u32>, line_type: Option<u8> }
```

multi-word 필드에는 `alias = "snake_case"` + camelCase 둘 다 받게 — 기존 e2e 가 보내던 `vertical_align`, `padding_left` 등 호환.

`apply_edit_op` 의 SetCellStyle 분기에서 `partial_cell_style_to_native_json` 변환 함수 호출 — `bgcolor` 를 `fillType=solid`+`fillColor` 로 펼침, `border.all` 을 4 방향에 일괄 적용 후 개별 키 override, `vertical_align "middle"` 을 u8 (1) 로 변환.

### 2. PartialRunStyle 확장 + alias — [edit_op.rs:108-150](../../src/document_core/commands/edit_op.rs#L108)

```rust
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialRunStyle {
    bold, italic, underline, strikethrough,    // 기존
    #[serde(alias = "baseSize", alias = "base_size", alias = "fontSize", alias = "font_size")]
    font_size: Option<u16>,                    // rename + 4-way alias
    #[serde(alias = "textColor", alias = "text_color")]
    color: Option<String>,                     // u32 → String rename + alias
    highlight: Option<String>,                 // 신규 — shadeColor 로 변환
    font_name: Option<String>,                 // 신규 — fontId 로 lookup 변환
}
```

`color` 타입을 `u32` → `String` 변경 (IR 응답이 hex string 이라 정합). 기존 호출처 grep 0건 → 안전.

`font_name` 변환은 apply 분기 안에서 `find_or_create_font_id_native` 호출 — 못 찾으면 *7 언어 전체에 신규 등록* 후 fontId 반환 (helpers.rs 영향 0).

ReplaceRuns / ReplaceCellRuns / InsertTextInCell 3 분기 모두 `partial_run_style_to_native_json` 경유.

### 3. PartialParagraphStyle alias — [edit_op.rs:24-58](../../src/document_core/commands/edit_op.rs#L24)

```rust
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialParagraphStyle {
    #[serde(alias = "alignment")]
    align: Option<String>,                                  // rename
    #[serde(alias = "lineSpacing", alias = "line_spacing", alias = "lineHeight")]
    line_height: Option<f64>,                               // rename + alias
    margin_left, margin_right, indent, spacing_before, spacing_after,
}
```

`parse_para_shape_mods` 가 `alignment`/`lineSpacing` 키를 받으므로 변환 함수에서 camelCase → native key (alignment/lineSpacing) 매핑.

### 4. table_ops.rs — BorderFill 트리거 조건 확장 — [table_ops.rs:554-558](../../src/document_core/commands/table_ops.rs#L554)

기존 `has_border = json.contains("\"borderLeft\"")` 만 보던 조건에 `fillType` / `fillColor` 도 트리거에 추가. *bgcolor 단독 변경 시* (4 방향 border 키 없이) BorderFill 이 새로 만들어지지 않아 silent drop 되던 사고를 잡음.

### 5. PatchSummary 가시화 — [server/src/ir_compact.rs](../../server/src/ir_compact.rs)

```rust
pub struct PatchSummary {
    pub changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_change_warning: Option<String>,
    // ...
}
```

`build_patch_diff` 가 changed=false 시 자동으로 *한국어 경고* 채움:
> "before/after 동일 — payload 의 style 키가 schema 와 일치하는지 확인 (오타·미지원 키는 deny_unknown_fields 로 400 반환)."

모델이 응답에서 `changed: false` 만 봐도 *원인 hint* 를 동봉 받음.

### 6. e2e — [sub7-style-round-trip.test.mjs](../../rhwp-studio/e2e/sub7-style-round-trip.test.mjs) (신규)

15 시나리오:

| # | 시나리오 | 검증 |
|---|---|---|
| 1 | set_cell_style + bgcolor | IR cell.style.bgcolor 일치 |
| 2 | border.all 단독 | 4 방향 같은 색 |
| 3 | border.all + left override | 좌측만 다른 색, 나머지 base |
| 4 | unknown cell key | HTTP 400 |
| 5 | replace_runs + color (광고) | IR run.style.color 일치 |
| 6 | replace_runs + textColor alias | 동일 동작 |
| 7 | font_size 3 alias (snake/camel/baseSize) | 모두 수용 |
| 8 | replace_runs + highlight | IR run.style.highlight 일치 |
| 9 | replace_runs + font_name "함초롬바탕" | fontId 변환 후 IR 노출 |
| 10 | unknown run key | HTTP 400 |
| 11 | set_paragraph_style + align (광고) | IR paragraph.style.align 일치 |
| 12 | alignment alias | 동일 동작 |
| 13 | line_height snake | IR 반영 |
| 14 | unknown paragraph key | HTTP 400 |
| 15 | no-op (이미 align right 인 문단에 다시 right) | changed:false + no_change_warning 채움 |

### 7. SKILL.md 갱신 — [SKILL.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/SKILL.md), [edit-phase.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/edit-phase.md)

- `...` 와일드카드 풀어쓰기 — run/paragraph/cell 모든 키 명시
- alias 정보 표기 (color = textColor / text_color, font-size = font_size / fontSize / baseSize / base_size, align = alignment, line-height = line_height / lineHeight / lineSpacing 등)
- *unknown key 는 400* (`deny_unknown_fields`) 명시
- ⚠ char-spacing / char-width — 현 native 단일 키 미지원, 별도 sub 예정 (보내면 400)
- PartialCellStyle 신규 키 (bgcolor, border, padding-*, border-fill-id, is-header, cell-protect) 추가 광고
- `body.diff.summary.no_change_warning` 가시화 안내

## 검증

### 단위

- `cargo test --lib` rhwp: **1466 pass / 0 fail / 6 ignored** (Sub-7 신규 17건 포함)
- `cargo test` rhwp-server: **78 pass / 0 fail** (Sub-7 신규 4건 포함)

### 빌드

- `cargo build --release` (rhwp-server): 20.47s ok
- `npm run build` (rhwp-studio): ok

### e2e 회귀 — 15 시나리오 (신규 + 기존)

| 시나리오 | 결과 |
|---|---|
| ws-bridge | PASS |
| sub6-ws-echo-skip | PASS |
| sub2-set-paragraph-style | PASS |
| sub2-replace-runs | PASS |
| sub2-set-cell-style | PASS |
| sub2-merge-cells | PASS |
| sub2-replace-cell-runs | PASS |
| sub2-insert-text-in-cell | PASS |
| sub2-undo | PASS |
| sub2-audit-diff-ir-slice | PASS |
| sub2-partial-update | PASS |
| sub2-canvas-insert-text | PASS |
| sub3-ir-compact | PASS |
| sub4-patch-diff | PASS |
| **sub7-style-round-trip** | **PASS (15 시나리오)** |

## 효과

1. *광고 키가 실제로 작동* — bgcolor / border / highlight / font-name / font-size / color / align / line-height 모두 광고된 이름 그대로 적용.
2. *silent drop 사고 종결* — unknown key 보내면 400 응답 + serde 에러 메시지에 잘못된 키 노출. 모델/클라가 typo 즉시 인지.
3. *no-op 가시화* — changed:false 일 때 응답에 한국어 경고 문구 자동 동봉. 모델이 "성공한 줄 알고 지나치는" 사고 예방.
4. *기존 호출자 호환* — snake_case / camelCase / kebab-case alias 다중 등록으로 기존 e2e 14건 회귀 0.

## 트레이드오프 / 후속

- **char-spacing / char-width** — native `parse_char_shape_mods` 가 7 언어별 배열 (`spacings[7]`, `ratios[7]`) 만 받음. 단일 스칼라 키 부재. 별도 sub 에서 (a) native 에 charSpacing/charWidth 단일 키 추가하거나 (b) PartialRunStyle 의 단일 값을 7 언어 배열로 broadcast 하는 변환 필요. 현 sub 에서는 *광고만 유지하고 보내면 400*.
- **bgcolor 단독 변경 시 기존 border 사라짐** — `create_border_fill_from_json` 이 borderLeft 등 키 없으면 기본값 (line_type=None, width=0) 으로 BorderFill 생성 → 셀의 기존 border 가 보존되지 않음. native 자체 한계. 별도 후속에서 셀의 기존 BorderFill 을 base 로 가져와 누락 키만 채우도록 native 보완 필요.
- **font_name 자동 등록 부수효과** — `find_or_create_font_id_native` 가 새 폰트 등록 시 *문서 defaults.run.font-name 도 갱신*. 의도된 동작이지만 사용자 인지 필요.
- **font-size 단위 mismatch** — 광고는 pt 단위지만 native 는 100 unit (8 pt = 800). 본 sub 에서는 변환 없이 그대로 전달 — 별도 sub 에서 단위 변환 추가 필요.

## 다음

`feature/jerry-command-expansion` 에 push → VM 재배포 → 사용자가 `set-cell-style {"bgcolor":"#FFC0CB"}` 보내서 *실제로 셀이 분홍색으로 변하는지* 확인.
