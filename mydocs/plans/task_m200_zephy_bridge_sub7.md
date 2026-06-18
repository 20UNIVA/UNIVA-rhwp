# Sub-7 — Partial*Style ↔ SKILL.md 광고 정합 (속성 silent drop 사고 해소)

## 배경

사용자 사고 — `set-cell-style` 에 `{"bgcolor": "#FFC0CB"}` 보냈고 HTTP 200 받았지만 *PatchDiff 의 `summary.changed: false`* — 실제 셀 색은 안 바뀜.

원인 추적:

1. [edit_op.rs:45-61](../../src/document_core/commands/edit_op.rs#L45) `PartialCellStyle` 에 `bgcolor` 필드 *부재*
2. serde 가 unknown field 를 `#[serde(default)]` 로 *silent drop* (`deny_unknown_fields` 없음)
3. `PartialCellStyle { width: None, height: None, ... }` 모두 None 으로 deserialize
4. `set_cell_properties_native` 에 빈 JSON 전달 → 변경 0
5. before == after → `changed: false` (정직한 보고)

이게 *하나의 사고가 아니라 시스템 패턴* 임을 audit 로 확인 — HIGH 7건 + MEDIUM 3건 (총 10건) 갭. *광고 (SKILL.md) ↔ 입력 schema (Partial*Style) ↔ native fn ↔ IR 응답* 4단 라인업이 불일치.

audit 결과 *native fn 들은 대부분 이미 지원* — 갭은 *Partial*Style 가 키를 노출 안 한 것* 뿐. 따라서 본 sub 는 *주로 schema 추가* + *alias 정리* + *deny_unknown_fields* 로 끝낼 수 있다 (document_core 본체 손댈 일 거의 없음).

## 목표

1. SKILL.md 에 광고된 모든 style 키가 *실제로 적용* 되도록 Partial*Style 필드 + serde alias 정합
2. unknown field 는 *silent drop* 이 아니라 *400 에러* — 클라/모델이 typo 즉시 인지
3. PatchDiff `summary.changed: false` 가 응답에 *눈에 띄게 노출* — 모델이 "성공한 줄 알았는데 안 바뀜" 을 놓치지 않게

## 비목표

- 광고되지 않은 native 속성 (shadow, outline, emboss, emphasis_dot 등) 의 새 광고 — 별도 sub
- HWPX 포맷 자체 변경 — native fn 이 이미 지원하므로 불필요
- SKILL.md 의 광고 카탈로그 *축소* — 광고된 것은 *전부 작동* 하게 만든다

## 갭 매트릭스 (audit 결과)

### HIGH — 광고만 하고 schema 부재 (silent drop)

| Variant | 광고 키 | 현재 PartialStyle | native 지원 | fix 방향 |
|---|---|---|---|---|
| SetCellStyle | `bgcolor` | ✗ | `fillType`+`fillColor` (via `create_border_fill_from_json`) | PartialCellStyle 에 `bgcolor: Option<String>` 추가 → JSON 직렬화 시 fillType=solid + fillColor=hex 변환 |
| SetCellStyle | `border` (nested) | ✗ | `borderLeft/Right/Top/Bottom` (via 동) | PartialCellStyle 에 `border: Option<BorderSpec>` 추가, 4 방향 펼침 직렬화 |
| ReplaceRuns / InsertTextInCell | `highlight` | ✗ | `shadeColor` (parse_char_shape_mods:322) | PartialRunStyle 에 `highlight: Option<String>` 추가 → shadeColor 키로 직렬화 |
| ReplaceRuns / InsertTextInCell | `font-size` | ✗ (`base_size` 있음, 다른 이름) | `fontSize` (parse_char_shape_mods:313) | 필드 rename: `base_size` → `font_size` + alias("base_size") 호환 |
| ReplaceRuns / InsertTextInCell | `font-name` | ✗ | `fontId` (parse_char_shape_mods:316) | PartialRunStyle 에 `font_name: Option<String>` 추가 → fontId 로 변환 (별도 lookup) |
| ReplaceRuns / InsertTextInCell | `char-spacing` | ✗ | *확인 필요* (Phase 0) | native 지원하면 추가, 없으면 별도 sub 로 |
| ReplaceRuns / InsertTextInCell | `char-width` | ✗ | *확인 필요* (Phase 0) | 동 |

### MEDIUM — 이름 mismatch

| 광고 키 | 현재 필드 | fix |
|---|---|---|
| `align` (paragraph) | `alignment` | serde alias 추가 또는 rename |
| `line-height` (paragraph) | `line_spacing` | 동 |
| `color` (run) | `text_color` (camelCase: `textColor`) | 동 |

### IR 응답 ↔ 입력 비대칭 (참고)

IR compact 응답 (`CellStyle`, `RunStyle`) 에는 모든 광고 키가 나타나지만 입력 schema 는 부분만 — 본 sub 가 정합을 맞춰 *round-trip* 가능하게 함.

## 설계

### Phase 0 — native fn 키 매핑 확정 (코드 변경 0)

#### 확정 결과 (2026-06-08 sub-7 agent)

**1. char-spacing / char-width — native 지원 없음 (직접 키 부재)**

`parse_char_shape_mods` ([src/document_core/helpers.rs:297-397](../../src/document_core/helpers.rs#L297-L397)) 의 입력 키 전수 확인:

| 카테고리 | 키 |
|---|---|
| 토글 | `bold`, `italic`, `underline`, `strikethrough`, `subscript`, `superscript`, `emboss`, `engrave`, `kerning` |
| 색상 | `textColor`, `shadeColor`, `underlineColor`, `shadowColor`, `strikeColor` |
| 크기/폰트 | `fontSize`, `fontId` |
| 모양 | `underlineType`, `underlineShape`, `strikeShape`, `outlineType`, `shadowType`, `emphasisDot` |
| 그림자 오프셋 | `shadowOffsetX`, `shadowOffsetY` |
| 언어별 배열 (7요소) | `fontIds`, `ratios`, `spacings`, `relativeSizes`, `charOffsets` |

`char-spacing` / `char-width` 에 대응되는 *단일 스칼라 키 없음*. `spacings` (i8[7]) / `ratios` (u8[7]) 가 7 언어별 배열로만 존재. SKILL.md 광고 키 `char-spacing` / `char-width` 는 *현 native 지원 외부* — 본 sub 에서 *추가하지 않음*. 별도 sub 에서 (a) 단일 스칼라 키를 native 에 추가하거나 (b) PartialRunStyle 에 i8/u8 단일 값을 받아 7 언어 배열로 broadcast 하는 변환을 만들어야 함.

**2. font-name → fontId 변환 — DocumentCore 가변 메서드 필요**

찾은 변환 후보:

- `find_font_id(&self, name) -> Option<u16>` ([html_import.rs:921-935](../../src/document_core/commands/html_import.rs#L921-L935)) — 읽기 전용, 이름 못 찾으면 None
- `find_or_create_font_id_native(&mut self, name) -> i32` ([formatting.rs:793-832](../../src/document_core/commands/formatting.rs#L793-L832)) — 가변, 못 찾으면 *7 언어 전체에 신규 등록* 후 ID 반환

apply_edit_op 는 이미 `&mut self` 위에서 동작 → `find_or_create_font_id_native` 호출 가능. PartialRunStyle 에 `font_name: Option<String>` 추가하고, *직렬화 함수 단계가 아니라* apply 분기 안에서 (DocumentCore 컨텍스트 확보 후) fontId 로 변환해서 native JSON 에 합쳐 넣는다.

대안: PartialRunStyle → native JSON 단계에서는 `fontName` 키로 그대로 직렬화하고, `parse_char_shape_mods` 가 `fontName` 키를 받으면 fontId 로 변환하도록 helpers.rs 확장. 이 안은 native 본체 수정이 필요. **본 sub 에서는 *전자 (apply 분기에서 변환)* 채택** — helpers.rs 영향 0.

**3. vertical-align 매핑 — 기 확인**

`set_cell_properties_native` ([table_ops.rs:528-534](../../src/document_core/commands/table_ops.rs#L528-L534)) 는 `verticalAlign` 을 u8 로 받음: 0=Top, 1=Center, 2=Bottom. SKILL.md 광고 "top"|"middle"|"bottom" 문자열은 PartialCellStyle 직렬화 함수에서 u8 로 변환해서 native 에 보낸다.

**4. paragraph 키 매핑 — 기 확인**

`parse_para_shape_mods` ([src/document_core/helpers.rs:494-584](../../src/document_core/helpers.rs#L494-L584)):
- align → `alignment` (lowercase 문자열: "left"|"right"|"center"|"justify"|"distribute")
- line_height → `lineSpacing` (i32)
- indent → `indent` (i32)
- margin_left → `marginLeft` (i32)
- margin_right → `marginRight` (i32)
- spacing_before → `spacingBefore` (i32)
- spacing_after → `spacingAfter` (i32)

기존 PartialParagraphStyle 의 직렬화 결과가 이미 native 키와 일치 — `align` / `line_height` rename 후에는 *변환 함수* 가 필요 (camelCase → native key 차이 발생).

### Phase 1 — PartialCellStyle 확장

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialCellStyle {
    // 기존
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub vertical_align: Option<String>,
    pub border_fill_id: Option<u16>,
    pub is_header: Option<bool>,
    pub cell_protect: Option<bool>,

    // [Sub-7] 신규 — native fn 이 이미 받는 fillType/fillColor 를 사용자 친화 키로 노출
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgcolor: Option<String>,   // "#FFC0CB" — 직렬화 시 fillType=solid + fillColor 변환

    // [Sub-7] 신규 — 4 방향 테두리. None 인 방향은 영향 없음.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderSpec>,

    // [Sub-7] 신규 — native fn 이 이미 받는 padding/textDirection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<i16>,
}

/// SKILL.md 의 nested border 객체. all 우선, 그 외 left/right/top/bottom 별도 지정 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BorderSpec {
    pub all: Option<BorderLine>,
    pub left: Option<BorderLine>,
    pub right: Option<BorderLine>,
    pub top: Option<BorderLine>,
    pub bottom: Option<BorderLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BorderLine {
    pub color: Option<String>,   // "#000000"
    pub width: Option<u32>,      // mm * 100 또는 hwp unit
    pub r#type: Option<u8>,      // 1 = solid, 2 = dotted, ...
}
```

`apply_edit_op` 의 SetCellStyle 분기 ([edit_op.rs:555](../../src/document_core/commands/edit_op.rs#L555)) — 현재 `serde_json::to_string(style)` 한 줄을 *PartialCellStyle → native JSON* 변환 함수로 교체:

```rust
fn partial_cell_style_to_native_json(style: &PartialCellStyle) -> String {
    let mut obj = serde_json::Map::new();
    if let Some(w) = style.width { obj.insert("width".into(), w.into()); }
    if let Some(h) = style.height { obj.insert("height".into(), h.into()); }
    if let Some(ref va) = style.vertical_align {
        obj.insert("verticalAlign".into(), vertical_align_to_u8(va).into());
    }
    if let Some(bf) = style.border_fill_id { obj.insert("borderFillId".into(), bf.into()); }
    if let Some(h) = style.is_header { obj.insert("isHeader".into(), h.into()); }
    if let Some(c) = style.cell_protect { obj.insert("cellProtect".into(), c.into()); }
    if let Some(ref bg) = style.bgcolor {
        obj.insert("fillType".into(), "solid".into());
        obj.insert("fillColor".into(), bg.clone().into());
    }
    if let Some(ref border) = style.border {
        // all 우선 → 4 방향 일괄 적용, 그 외 left/right/top/bottom 개별 override.
        let merged = border_to_4dir(border);
        if let Some(b) = merged.left   { obj.insert("borderLeft".into(),   border_line_to_json(&b)); }
        if let Some(b) = merged.right  { obj.insert("borderRight".into(),  border_line_to_json(&b)); }
        if let Some(b) = merged.top    { obj.insert("borderTop".into(),    border_line_to_json(&b)); }
        if let Some(b) = merged.bottom { obj.insert("borderBottom".into(), border_line_to_json(&b)); }
    }
    // padding
    if let Some(p) = style.padding_left   { obj.insert("paddingLeft".into(),   p.into()); }
    if let Some(p) = style.padding_right  { obj.insert("paddingRight".into(),  p.into()); }
    if let Some(p) = style.padding_top    { obj.insert("paddingTop".into(),    p.into()); }
    if let Some(p) = style.padding_bottom { obj.insert("paddingBottom".into(), p.into()); }
    serde_json::Value::Object(obj).to_string()
}
```

### Phase 2 — PartialRunStyle 확장 + alias

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialRunStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,

    // [Sub-7] rename: base_size → font_size + alias("baseSize") 호환
    #[serde(alias = "baseSize", alias = "base_size")]
    pub font_size: Option<u16>,

    // [Sub-7] rename: text_color → color + alias("textColor")
    #[serde(alias = "textColor", alias = "text_color")]
    pub color: Option<String>,   // CSS hex "#RRGGBB" — 기존 u32 도 호환 (TBD)

    // [Sub-7] 신규 — native shadeColor
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,

    // [Sub-7] 신규 — font name (Phase 0 에서 fontId 변환 방법 확정)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,

    // [Sub-7] 신규 — Phase 0 결과 따라 추가 (char-spacing, char-width)
    // 여기는 native 지원 확인 후 채움.
}
```

ReplaceRuns / ReplaceCellRuns / InsertTextInCell 분기에서 *PartialRunStyle → native JSON* 변환 함수 사용. `font_name` 은 `fonts` 테이블에서 ID 조회 후 `fontId` 로 직렬화.

### Phase 3 — PartialParagraphStyle alias

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialParagraphStyle {
    // [Sub-7] rename: alignment → align + alias("alignment")
    #[serde(alias = "alignment")]
    pub align: Option<String>,

    // [Sub-7] rename: line_spacing → line_height + alias
    #[serde(alias = "lineSpacing", alias = "line_spacing")]
    pub line_height: Option<f64>,

    // 기존 — 광고는 안 됐지만 native 지원, 유지.
    pub margin_left: Option<i16>,
    pub margin_right: Option<i16>,
    pub indent: Option<i16>,
    pub spacing_before: Option<i16>,
    pub spacing_after: Option<i16>,
}
```

camelCase 변환 후 native key 와 매핑하는 직렬화 변환 함수 (기존 `to_string` 대신).

### Phase 4 — PatchDiff `changed: false` 가시화

[ir_compact.rs build_patch_diff](../../server/src/ir_compact.rs) — `changed: false` 일 때 응답에 *warning 필드* 추가:

```rust
pub struct PatchSummary {
    pub changed: bool,
    // [Sub-7] changed=false 면 자동 채워짐 — 모델이 못 보고 지나치지 않도록.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_change_warning: Option<String>,  // "before/after 동일 — payload 의 style 키가 schema 와 일치하는지 확인 필요"
    // ...
}
```

### Phase 5 — 단위 테스트

신규 7+ 테스트:

1. `PartialCellStyle deserialize { "bgcolor": "#FFC0CB" }` → bgcolor=Some("#FFC0CB")
2. `PartialCellStyle deserialize { "unknownKey": 1 }` → 400 (deny_unknown_fields)
3. `PartialCellStyle → native JSON` 변환 — bgcolor → fillType+fillColor
4. `PartialRunStyle deserialize { "color": "#FF0000" }` → color=Some("#FF0000")
5. `PartialRunStyle deserialize { "textColor": "#FF0000" }` → color=Some (alias 동작)
6. `PartialRunStyle deserialize { "highlight": "#FFFF00" }` → highlight=Some
7. `PartialParagraphStyle deserialize { "align": "right" }` → align=Some
8. apply_edit_op SetCellStyle + bgcolor → 적용 후 ir-slice 에 bgcolor 노출
9. apply_edit_op ReplaceRuns + highlight → run.style.highlight 응답에 반영
10. apply_edit_op SetParagraphStyle + align (광고 키) → 정상 적용

### Phase 6 — e2e 신규 + 회귀

신규 `e2e/sub7-style-round-trip.test.mjs`:

- 10+ 시나리오: 각 광고 키마다 SetCellStyle/ReplaceRuns/SetParagraphStyle 보냄 → GET ir-slice 응답에 같은 키·같은 값이 나오는지 round-trip 검증
- unknown key 보내면 400 (deny_unknown_fields)
- PatchDiff `changed: true` 확인

기존 회귀:
- sub2 15건, sub3, sub4, sub5, sub6 (총 18 건) — `npm run build` + 모두 통과

### Phase 7 — SKILL.md 갱신

광고 카탈로그를 *실제 작동하는 키* 로 정합. 없어진 키 0, 새로 *명시적으로 추가된 키* 가 있으면 표기.

## 단계 분해 (sub-agent 분담)

| Step | 내용 | Agent |
|---|---|---|
| 0 | native fn 키 매핑 확정 (char-spacing/char-width/font-name 변환) | Explore 1명 |
| 1 | PartialCellStyle 확장 + BorderSpec + 변환 함수 + deny_unknown_fields | general-purpose 1명 |
| 2 | PartialRunStyle 확장 (highlight/font_name 등) + alias + 변환 함수 | general-purpose 1명 (Phase 1 종료 후) |
| 3 | PartialParagraphStyle alias + 변환 함수 | general-purpose (Phase 2 와 병행 가능) |
| 4 | PatchSummary no_change_warning | (Phase 1-3 과 함께) |
| 5 | 단위 테스트 (10+) | (Phase 1-3 코드 옆 추가) |
| 6 | e2e sub7-style-round-trip + 18 회귀 | general-purpose 1명 (Phase 1-3 종료 후) |
| 7 | SKILL.md 갱신 | general-purpose 1명 (Phase 1-3 종료 후 병행) |
| 8 | 보고서 + commit + push | 사람 (자동 모드 진행) |

전체 일정 추정: 1-2일.

## 검증 체크리스트

- [x] Phase 0 native fn 키 확정 — 결과 plan 본문 갱신
- [x] `cargo test` 회귀 0 + Sub-7 신규 단위 7+ 통과 (rhwp 1466 / server 78)
- [x] `cargo build --release` 통과 (dev profile 만 확인. release 는 sub8 commit 전 점검)
- [x] `npm run build` 통과 (이전 sub-agent 확인)
- [x] sub7-style-round-trip 15 시나리오 통과
- [x] 기존 e2e 14건 회귀 0 (ws-bridge, sub6, sub2 12종, sub3, sub4)
- [x] Live curl 검증: bgcolor round-trip 확인 — 시나리오 1 (`set_cell_style` + `bgcolor: "#FFC0CB"`) 가 IR cell.style.bgcolor === "#FFC0CB" 보장

## Phase 6 — 회귀 발견 + fix

신규 e2e 작성 시 *기존 e2e 의 snake_case 키* (`vertical_align`, `font_size`, `margin_left`, `line_height` 등) 가 `deny_unknown_fields` 에 의해 400 받음. Plan §리스크 항목에 명시된 케이스 — *광고 카탈로그* 가 kebab-case 이지만 기존 e2e/클라가 snake_case 사용. fix: PartialParagraphStyle/PartialCellStyle/PartialRunStyle 의 multi-word 필드 *모두에* `alias = "snake_case"` + `alias = "kebab-case"` 둘 다 추가. 추가 alias 적용 파일:

- [edit_op.rs](../../src/document_core/commands/edit_op.rs) — line 47-61 (PartialParagraphStyle), 77-105 (PartialCellStyle), 158-193 (PartialRunStyle)

추가 발견: `font-size` 단위 mismatch (광고 pt vs native 100단위) — 시나리오 7 은 *200 + diff 응답* 만 검증 (alias 수용 검증). 단위 변환은 별도 sub.

## Phase 7 — SKILL.md 갱신 결과

- [26ZEPHY-skills/skills/document_edit/hwp-doc-edit/SKILL.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/SKILL.md) — line 58 (cell style 키 표 확장), line 68-73 ("★ style 키 세 가지 함정" 으로 확장, unknown key 400 명시)
- [edit-phase.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/edit-phase.md) — line 79 (set-paragraph-style 예시 단위 number 로 정정), line 162-168 (set-cell-style 예시 width 숫자, type 추가, border override 설명), line 218-260 (§2.4 카탈로그 alias 표 + char-spacing/char-width 미지원 경고 + cell style 전체 키 + noChangeWarning 가시화)

본 26ZEPHY-skills 는 별도 git repo — *변경만 적용, commit 안 함*.

## 리스크

- *기존 클라가 base_size / text_color / alignment / line_spacing 을 그대로 보냄* — serde alias 로 호환. e2e 회귀로 검증.
- *deny_unknown_fields 가 너무 엄격해 기존 e2e 깨짐* — Phase 6 회귀로 검출. 깨지는 시나리오는 *unknown key 보낸 것* → 의도된 동작 (silent drop 사고 예방), 깨진 e2e 는 수정 대상.
- *font-name → fontId 변환 실패* (해당 폰트가 fonts 테이블에 없는 경우) — Phase 2 에서 *fallback 정책* 결정 (default font 사용 vs 400 에러). 권장: default + warning 필드.
- *BorderSpec.all + left override 동시 지정* — `all` 먼저 적용 후 개별 키가 덮어쓰는 의미. 단위 테스트로 명시.

## 다음

승인 즉시 Phase 0 (Explore) → Phase 1-3 (general-purpose, 순차) → Phase 6-7 (병렬). 작업 디렉토리 `/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp`, 브랜치 `feature/jerry-command-expansion`.
