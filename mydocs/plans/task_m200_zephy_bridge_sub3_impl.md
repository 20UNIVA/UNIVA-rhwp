# Sub-3 zephy-bridge — IR Compact 응답 서버 포팅 구현 계획서

> **에이전트 작업자용:** 본 plan 은 `superpowers:subagent-driven-development` (권장) 또는 `superpowers:executing-plans` 로 task 단위 실행. 단계는 `- [ ]` 체크박스로 추적.

**목표:** 옛 rhwp 원본의 `ir-builder.ts` 알고리즘을 서버 측 Rust 모듈 (`server/src/ir_compact.rs`) 로 옮겨, `get-ir-slice` 가 init.md 가이드의 평탄 형식 (type/runs/cell_locator/defaults) 으로 응답하도록 한다.

**구조:** 신규 1 파일 + 기존 endpoint 분기 교체 + 노트북 라우터 fix. 데이터 접근은 wasm round-trip 없이 `DocumentCore` 내부 struct 를 직접 읽음. rhwp 본체 무변경.

**기술 스택:** Rust · serde · axum · tokio · Python 노트북 · Node WS e2e.

**참고 spec:** [task_m200_zephy_bridge_sub3.md](task_m200_zephy_bridge_sub3.md) — 본 plan 의 *값 변환 공식·DoD·검증 기준*.

---

## 파일 구조

| 파일 | 신규/수정 | 역할 |
|---|---|---|
| `server/src/ir_compact.rs` | **신규** (~500 lines) | IR 타입 + 변환 + 압축 + unit test 모두 |
| `server/src/main.rs` | 수정 (`mod ir_compact;` + `ir_slice_handler`) | endpoint 진입점 — ir_compact 모듈에 위임 |
| `hwp_sub_agent_simulation_ssr.ipynb` cell 3 | 수정 | `compact: true/false` payload 키를 `mode` query 로 변환 |
| `rhwp-studio/e2e/sub3-ir-compact.test.mjs` | **신규** (~150 lines) | compact 응답 형식 + 표·셀 + defaults 검증 |

## 작업 디렉토리·브랜치

- 작업 디렉토리: `/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp`
- 브랜치: `local/task_m200_zephy_bridge` (Sub-1·2 와 같은 브랜치 — Sub-3 도 *zephy_bridge milestone* 의 후속이라 단일 통합)
- 서버 가동 스크립트: `rhwp-studio/e2e/sub2-server.sh start|stop|restart`

---

## Phase 1 — 모듈 scaffolding + IR 타입

### Task 1.1 — ir_compact 모듈 신설

**Files:**
- Create: `server/src/ir_compact.rs`
- Modify: `server/src/main.rs` (mod 등록 한 줄)

- [ ] **Step 1: 빈 모듈 파일 생성**

`server/src/ir_compact.rs`:
```rust
//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

#![allow(dead_code)]  // 구현 진행 중 일시 허용. Phase 5 종료 시 제거.

use serde::Serialize;

// 이하 Phase 1.2 ~ 5 에서 채워짐.
```

- [ ] **Step 2: main.rs 에 mod 등록**

`server/src/main.rs` 상단 (다른 `mod` 선언 옆, 약 [server/src/main.rs:20](server/src/main.rs#L20)):
```rust
mod ir_compact;
```

- [ ] **Step 3: 빌드 확인**

Run: `cd server && cargo build`
Expected: 컴파일 성공, warning 0 (dead_code 는 allow 로 무시됨).

- [ ] **Step 4: commit**

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
git add server/src/ir_compact.rs server/src/main.rs
git commit -m "Task #zephy-bridge Sub-3: ir_compact 모듈 scaffolding"
```

### Task 1.2 — 글자·문단·셀 서식 타입 정의

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: RunStyle / ParagraphStyle / CellStyle / CellBorderSpec 정의**

`server/src/ir_compact.rs` 에 추가 (옛 `types.ts:10-47` 의 Rust 대응):
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct RunStyle {
    #[serde(skip_serializing_if = "Option::is_none")] pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub highlight: Option<String>,
    #[serde(rename = "font-size", skip_serializing_if = "Option::is_none")] pub font_size: Option<f64>,
    #[serde(rename = "font-name", skip_serializing_if = "Option::is_none")] pub font_name: Option<String>,
    #[serde(rename = "char-spacing", skip_serializing_if = "Option::is_none")] pub char_spacing: Option<i32>,
    #[serde(rename = "char-width", skip_serializing_if = "Option::is_none")] pub char_width: Option<i32>,
    #[serde(rename = "vertical-align", skip_serializing_if = "Option::is_none")] pub vertical_align: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ParagraphStyle {
    #[serde(skip_serializing_if = "Option::is_none")] pub align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub indent: Option<i32>,
    #[serde(rename = "line-height", skip_serializing_if = "Option::is_none")] pub line_height: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CellBorderSpec {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")] pub border_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CellBorder {
    #[serde(skip_serializing_if = "Option::is_none")] pub left: Option<CellBorderSpec>,
    #[serde(skip_serializing_if = "Option::is_none")] pub right: Option<CellBorderSpec>,
    #[serde(skip_serializing_if = "Option::is_none")] pub top: Option<CellBorderSpec>,
    #[serde(skip_serializing_if = "Option::is_none")] pub bottom: Option<CellBorderSpec>,
    #[serde(skip_serializing_if = "Option::is_none")] pub all: Option<CellBorderSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CellStyle {
    #[serde(skip_serializing_if = "Option::is_none")] pub bgcolor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub border: Option<CellBorder>,
    #[serde(rename = "vertical-align", skip_serializing_if = "Option::is_none")] pub vertical_align: Option<String>,
}
```

`#[serde(rename_all = ...)]` 대신 *키별 명시* — 일부 키만 하이픈 (`font-size`) 이고 나머지는 그대로라 정확히 init.md spec 과 일치.

- [ ] **Step 2: 컴파일 확인**

Run: `cd server && cargo build`
Expected: 성공.

- [ ] **Step 3: unit test — Serialize JSON 키 확인**

`server/src/ir_compact.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn run_style_serializes_with_hyphens() {
        let s = RunStyle {
            bold: Some(true),
            font_size: Some(11.0),
            font_name: Some("맑은 고딕".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v, json!({"bold": true, "font-size": 11.0, "font-name": "맑은 고딕"}));
    }

    #[test]
    fn paragraph_style_serializes_with_hyphens() {
        let s = ParagraphStyle {
            align: Some("center".into()),
            line_height: Some(160),
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v, json!({"align": "center", "line-height": 160}));
    }

    #[test]
    fn cell_style_with_border_all_only() {
        let s = CellStyle {
            bgcolor: Some("#FFC107".into()),
            border: Some(CellBorder {
                all: Some(CellBorderSpec { width: Some(100), color: Some("#000000".into()), ..Default::default() }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert!(v["border"]["all"]["width"] == 100);
        assert!(v["border"]["left"].is_null());
    }
}
```

Run: `cd server && cargo test ir_compact::tests::`
Expected: 3 test 모두 pass.

- [ ] **Step 4: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: 글자·문단·셀 서식 타입 + Serialize 키 검증"
```

### Task 1.3 — IR 컨테이너 타입 (IrRun · IrParagraph · IrSlice · DocDefaults)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: IrRun · CellLocator · IrTextParagraph · IrTableCell · IrTableParagraph · IrParagraph enum 정의**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct IrRun {
    pub char_offset: usize,
    pub length: usize,
    pub text: String,
    pub style: RunStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellLocator {
    pub table_para: usize,
    pub row: u16,
    pub col: u16,
    pub cell_para: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTextParagraph {
    pub id: String,
    pub sec: usize,
    pub para: i64,  // 셀 내부 문단이면 -1
    #[serde(rename = "type")] pub kind: &'static str,  // "text"
    pub style: ParagraphStyle,
    pub runs: Vec<IrRun>,
    #[serde(skip_serializing_if = "Option::is_none")] pub cell_locator: Option<CellLocator>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTableCell {
    pub row: u16,
    pub col: u16,
    #[serde(skip_serializing_if = "Option::is_none")] pub row_span: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")] pub col_span: Option<u16>,
    pub style: CellStyle,
    pub paragraphs: Vec<IrParagraph>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTableParagraph {
    pub id: String,
    pub sec: usize,
    pub para: usize,
    #[serde(rename = "type")] pub kind: &'static str,  // "table"
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<IrTableCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum IrParagraph {
    Text(IrTextParagraph),
    Table(IrTableParagraph),
}
```

- [ ] **Step 2: IrAnchor · IrDocMeta · IrSlice · DocDefaults 정의**

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IrAnchor {
    pub sec: usize,
    pub para_start: usize,
    pub para_end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrDocMeta {
    pub edit_session_id: String,
    pub page: u32,
    pub total_pages: u32,
    pub anchor: IrAnchor,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrSlice {
    pub doc_meta: IrDocMeta,
    pub paragraphs: Vec<IrParagraph>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocDefaults {
    pub run: RunStyle,
    pub paragraph: ParagraphStyle,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompactIrSlice {
    pub doc_meta: IrDocMeta,
    pub paragraphs: Vec<serde_json::Value>,  // 압축 후엔 dynamic — 단일 run 은 runs 생략 등
    pub defaults: DocDefaults,
}
```

- [ ] **Step 3: 컴파일 + 빈 IrSlice JSON 직렬화 unit test**

```rust
#[test]
fn empty_ir_slice_serializes() {
    let slice = IrSlice {
        doc_meta: IrDocMeta {
            edit_session_id: "sim-1".into(),
            page: 1,
            total_pages: 1,
            anchor: IrAnchor { sec: 0, para_start: 0, para_end: 0 },
        },
        paragraphs: vec![],
    };
    let v = serde_json::to_value(&slice).unwrap();
    assert_eq!(v["doc_meta"]["page"], 1);
    assert_eq!(v["doc_meta"]["anchor"]["para_start"], 0);
    assert!(v["paragraphs"].as_array().unwrap().is_empty());
}
```

Run: `cd server && cargo test ir_compact::tests::empty_ir_slice_serializes`
Expected: pass.

- [ ] **Step 4: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: IrRun·IrParagraph·IrSlice·DocDefaults 타입 정의"
```

---

## Phase 2 — 값 변환 5 종

### Task 2.1 — 보조 helper (color · alignment · vertical-align · primary font)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: color_ref_to_css 복제**

rhwp 본체 [src/document_core/helpers.rs:839](src/document_core/helpers.rs#L839) 의 `pub(crate)` 함수와 동일. server 가 다른 crate 라 직접 호출 불가 — 동일 로직 복제:

```rust
use rhwp::model::ColorRef;

fn color_ref_to_css(color: ColorRef) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}
```

(실제 `ColorRef` 의 정확한 필드 — `r`/`g`/`b` 또는 `.0`/`.1`/`.2` — 는 빌드 시 확인. spec §5.5 의 *rhwp 의 helpers 와 동일 결과* 가 invariant.)

- [ ] **Step 2: alignment_to_str + vertical_align_to_str**

```rust
use rhwp::model::style::Alignment;

fn alignment_to_str(a: Alignment) -> &'static str {
    match a {
        Alignment::Justify => "justify",
        Alignment::Left => "left",
        Alignment::Right => "right",
        Alignment::Center => "center",
        Alignment::Distribute => "distribute",
        Alignment::Split => "split",
    }
}

fn vertical_align_to_str(subscript: bool, superscript: bool) -> &'static str {
    if subscript { "sub" }
    else if superscript { "super" }
    else { "baseline" }
}

fn cell_vertical_align_to_str(va: u8) -> &'static str {
    match va {
        0 => "top",
        1 => "middle",
        2 => "bottom",
        _ => "top",
    }
}
```

- [ ] **Step 3: unit test — 변환 결과 정확**

```rust
#[test]
fn helper_alignment_to_str() {
    assert_eq!(alignment_to_str(Alignment::Center), "center");
    assert_eq!(alignment_to_str(Alignment::Justify), "justify");
}

#[test]
fn helper_vertical_align() {
    assert_eq!(vertical_align_to_str(true, false), "sub");
    assert_eq!(vertical_align_to_str(false, true), "super");
    assert_eq!(vertical_align_to_str(false, false), "baseline");
}

#[test]
fn helper_color_ref() {
    let red = ColorRef { r: 0xFF, g: 0x00, b: 0x00 };
    assert_eq!(color_ref_to_css(red), "#FF0000");
}
```

Run: `cd server && cargo test ir_compact::tests::helper_`
Expected: 3 test pass.

- [ ] **Step 4: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: 보조 helper — color/alignment/vertical-align 변환"
```

### Task 2.2 — char_shape_to_run_style

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

옛 `style-map.ts::charPropsToRunStyle` 의 Rust 대응. ParagraphView 가 *resolved CharStyle* (style_resolver 의 결과) 와 *raw CharShape* (doc_info.char_shapes) 둘을 모두 받아야 함 — base_size (HWPUNIT) 는 *raw 측* 에 있음.

```rust
use rhwp::model::style::{CharShape, UnderlineType};
use rhwp::renderer::style_resolver::{primary_font_name, ResolvedCharStyle};  // 정확 path 는 빌드 시 확인

fn char_shape_to_run_style(
    cs: &ResolvedCharStyle,
    raw_cs: &CharShape,
    lang_idx: usize,
) -> RunStyle {
    let font_family_raw = cs.font_family_for_lang(lang_idx);
    let font_family = primary_font_name(font_family_raw).to_string();
    RunStyle {
        bold: Some(cs.bold),
        italic: Some(cs.italic),
        underline: Some(!matches!(cs.underline, UnderlineType::None)),
        strikethrough: Some(cs.strike_out),
        color: Some(color_ref_to_css(cs.text_color)),
        highlight: cs.shade_color.map(color_ref_to_css),
        font_size: Some((raw_cs.base_size as f64) / 100.0),
        font_name: Some(font_family),
        char_spacing: Some(cs.spacings.first().copied().unwrap_or(0)),
        char_width: Some(cs.ratios.first().copied().unwrap_or(100)),
        vertical_align: Some(vertical_align_to_str(cs.subscript, cs.superscript).to_string()),
    }
}
```

*ResolvedCharStyle* 의 정확한 type 이름 / 필드 — `cs.bold`·`cs.italic`·`cs.underline`·`cs.strike_out`·`cs.text_color`·`cs.shade_color`·`cs.spacings`·`cs.ratios`·`cs.subscript`·`cs.superscript`·`cs.font_family_for_lang(idx)` — 는 rhwp 본체의 *style_resolver.rs* 또는 *get_char_properties_at_native* ([formatting.rs:149-220](src/document_core/commands/formatting.rs#L149-L220)) 의 참조 위치 확인 후 정확히 매핑.

대안: *`get_char_properties_at_native`* 의 String JSON 을 받아 *serde_json::Value* 로 parse — 정확하지만 round-trip 비용. **구현 시 우선 *struct 직접 접근*, 실패 (private 필드 등) 시 native JSON path 로 fallback.**

- [ ] **Step 2: unit test — bold + size 변환**

```rust
#[test]
fn run_style_from_char_shape_bold_size() {
    use rhwp::document_core::DocumentCore;
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    // 첫 문단에 텍스트 + bold 적용 (workbench 와 동등한 native 호출)
    core.insert_text_native(0, 0, 0, "A").unwrap();
    core.set_char_shape_native(0, 0, 0, 1, r#"{"bold": true}"#).unwrap();

    // resolved + raw 둘 다 받아 run_style 변환
    let para = &core.document.sections[0].paragraphs[0];
    let id = para.char_shape_id_at(0).unwrap();
    let resolved = core.styles.char_styles.get(id as usize).expect("resolved");
    let raw = core.document.doc_info.char_shapes.get(id as usize).expect("raw");
    let run_style = char_shape_to_run_style(resolved, raw, 0);

    assert_eq!(run_style.bold, Some(true));
    assert!(run_style.font_size.unwrap() > 0.0);  // base_size 가 들어옴
    assert!(run_style.color.is_some());
}
```

Run: `cd server && cargo test ir_compact::tests::run_style_from_char_shape_bold_size`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: char_shape_to_run_style + 변환 검증"
```

### Task 2.3 — para_shape_to_para_style

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
use rhwp::model::style::{ParaShape, LineSpacingType};

fn para_shape_to_para_style(ps: &ParaShape) -> ParagraphStyle {
    ParagraphStyle {
        align: Some(alignment_to_str(ps.alignment).to_string()),
        indent: Some(ps.indent),
        line_height: if matches!(ps.line_spacing_type, LineSpacingType::Percent) {
            Some(ps.line_spacing)
        } else {
            None
        },
    }
}
```

- [ ] **Step 2: unit test**

```rust
#[test]
fn para_style_align_center_percent_160() {
    let ps = ParaShape {
        alignment: Alignment::Center,
        indent: 0,
        line_spacing: 160,
        line_spacing_type: LineSpacingType::Percent,
        ..Default::default()
    };
    let s = para_shape_to_para_style(&ps);
    assert_eq!(s.align.as_deref(), Some("center"));
    assert_eq!(s.line_height, Some(160));
}

#[test]
fn para_style_line_height_omitted_when_not_percent() {
    let ps = ParaShape {
        line_spacing: 1000,
        line_spacing_type: LineSpacingType::Fixed,
        ..Default::default()
    };
    let s = para_shape_to_para_style(&ps);
    assert_eq!(s.line_height, None);
}
```

Run: `cd server && cargo test ir_compact::tests::para_style_`
Expected: 2 pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: para_shape_to_para_style + percent-only line-height"
```

### Task 2.4 — cell_to_cell_style

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
use rhwp::model::table::Cell;
use rhwp::model::table::CellBorder as RawCellBorder;  // 정확 path 는 model/table.rs 확인

fn cell_border_spec(b: Option<&RawCellBorder>) -> Option<CellBorderSpec> {
    b.map(|raw| CellBorderSpec {
        border_type: Some(raw.border_type),
        width: Some(raw.width),
        color: Some(color_ref_to_css(raw.color)),
    })
}

fn cell_to_cell_style(cell: &Cell) -> CellStyle {
    let border = CellBorder {
        left:   cell_border_spec(cell.border_left.as_ref()),
        right:  cell_border_spec(cell.border_right.as_ref()),
        top:    cell_border_spec(cell.border_top.as_ref()),
        bottom: cell_border_spec(cell.border_bottom.as_ref()),
        all:    None,  // compact 단계에서 all 로 축약. raw 단계에서는 4면 분리.
    };
    let has_border = border.left.is_some() || border.right.is_some() || border.top.is_some() || border.bottom.is_some();
    CellStyle {
        bgcolor: cell.fill_color.map(color_ref_to_css),
        width: Some(cell.width),
        height: Some(cell.height),
        border: if has_border { Some(border) } else { None },
        vertical_align: Some(cell_vertical_align_to_str(cell.vertical_align).to_string()),
    }
}
```

`Cell` struct 의 실제 필드 이름 — `fill_color`/`width`/`height`/`border_left`·`right`·`top`·`bottom`/`vertical_align` — 은 [src/model/table.rs:85](src/model/table.rs#L85) 에서 확인.

- [ ] **Step 2: unit test**

```rust
#[test]
fn cell_style_with_bgcolor() {
    let mut cell = Cell::default();
    cell.fill_color = Some(ColorRef { r: 0xFF, g: 0xC1, b: 0x07 });
    cell.width = 1000;
    cell.height = 500;
    let s = cell_to_cell_style(&cell);
    assert_eq!(s.bgcolor.as_deref(), Some("#FFC107"));
    assert_eq!(s.width, Some(1000));
    assert!(s.border.is_none());  // border 미설정 시 None
}
```

Run: `cd server && cargo test ir_compact::tests::cell_style_`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: cell_to_cell_style + bgcolor/border 변환"
```

---

## Phase 3 — build_ir_slice 텍스트 paragraph + collect_runs

### Task 3.1 — collect_runs (인접 동일 스타일 run 병합)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성** (옛 `ir-builder.ts::collectRuns` Rust 대응)

```rust
fn collect_runs<F>(text: &str, len: usize, mut style_at: F) -> Vec<IrRun>
where
    F: FnMut(usize) -> RunStyle,
{
    if len == 0 {
        return vec![IrRun { char_offset: 0, length: 0, text: String::new(), style: RunStyle::default() }];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut runs = Vec::new();
    let mut run_start = 0usize;
    let mut current: Option<RunStyle> = None;
    for offset in 0..len {
        let style = style_at(offset);
        match &current {
            None => current = Some(style),
            Some(cur) if *cur != style => {
                let text_slice: String = chars[run_start..offset].iter().collect();
                runs.push(IrRun {
                    char_offset: run_start,
                    length: offset - run_start,
                    text: text_slice,
                    style: cur.clone(),
                });
                run_start = offset;
                current = Some(style);
            }
            _ => {}
        }
    }
    if let Some(cur) = current {
        let text_slice: String = chars[run_start..len].iter().collect();
        runs.push(IrRun {
            char_offset: run_start,
            length: len - run_start,
            text: text_slice,
            style: cur,
        });
    }
    runs
}
```

- [ ] **Step 2: unit test**

```rust
#[test]
fn collect_runs_single_style() {
    let s = RunStyle::default();
    let runs = collect_runs("ABC", 3, |_| s.clone());
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "ABC");
    assert_eq!(runs[0].length, 3);
}

#[test]
fn collect_runs_two_styles() {
    let bold = RunStyle { bold: Some(true), ..Default::default() };
    let plain = RunStyle::default();
    let runs = collect_runs("ABCDE", 5, |off| if off < 2 { bold.clone() } else { plain.clone() });
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "AB");
    assert_eq!(runs[0].style.bold, Some(true));
    assert_eq!(runs[1].text, "CDE");
    assert_eq!(runs[1].style.bold, None);
}

#[test]
fn collect_runs_empty_paragraph() {
    let runs = collect_runs("", 0, |_| RunStyle::default());
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].length, 0);
}
```

Run: `cd server && cargo test ir_compact::tests::collect_runs_`
Expected: 3 pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: collect_runs — 인접 동일 스타일 run 병합"
```

### Task 3.2 — build_text_paragraph (전체 문단 IR 빌드)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
fn build_text_paragraph(core: &rhwp::document_core::DocumentCore, sec: usize, para: usize) -> IrTextParagraph {
    let section = &core.document.sections[sec];
    let p = &section.paragraphs[para];
    let len = p.text.chars().count();

    let para_style = core.styles.para_styles
        .get(p.para_shape_id as usize)
        .map(para_shape_to_para_style)
        .unwrap_or_default();

    let runs = collect_runs(&p.text, len, |off| {
        let id = p.char_shape_id_at(off).unwrap_or(0) as usize;
        let resolved = core.styles.char_styles.get(id);
        let raw = core.document.doc_info.char_shapes.get(id);
        match (resolved, raw) {
            (Some(rs), Some(rw)) => {
                let lang_idx = p.text.chars().nth(off)
                    .map(rhwp::renderer::style_resolver::detect_lang_category)
                    .unwrap_or(0);
                char_shape_to_run_style(rs, rw, lang_idx)
            }
            _ => RunStyle::default(),
        }
    });

    IrTextParagraph {
        id: format!("p_{}_{}", sec, para),
        sec,
        para: para as i64,
        kind: "text",
        style: para_style,
        runs,
        cell_locator: None,
    }
}
```

- [ ] **Step 2: unit test — blank hwpx + insert_text "ABC" + bold 첫 글자**

```rust
#[test]
fn build_text_paragraph_two_runs() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_text_native(0, 0, 0, "ABC").unwrap();
    core.set_char_shape_native(0, 0, 0, 1, r#"{"bold": true}"#).unwrap();
    let para = build_text_paragraph(&core, 0, 0);
    assert_eq!(para.kind, "text");
    assert_eq!(para.id, "p_0_0");
    assert_eq!(para.runs.len(), 2);
    assert_eq!(para.runs[0].text, "A");
    assert_eq!(para.runs[0].style.bold, Some(true));
    assert_eq!(para.runs[1].text, "BC");
    assert!(para.runs[1].style.bold == None || para.runs[1].style.bold == Some(false));
}
```

Run: `cd server && cargo test ir_compact::tests::build_text_paragraph_two_runs`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: build_text_paragraph + run 분할 검증"
```

### Task 3.3 — build_ir_slice 진입점 (text only)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: BuildOptions + 진입 함수**

```rust
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub sec: usize,
    pub para_start: usize,
    pub para_end: Option<usize>,
    pub edit_session_id: Option<String>,
}

/// IR slice 빌드. 본 단계 (3.3) 에서는 *텍스트 문단만* 처리.
/// 표 처리는 Phase 4 에서 build_paragraph 분기로 합쳐짐.
pub fn build_ir_slice(core: &rhwp::document_core::DocumentCore, opts: &BuildOptions) -> IrSlice {
    let sec = opts.sec;
    let section = &core.document.sections[sec];
    let total = section.paragraphs.len();
    let start = opts.para_start.min(total);
    let end = opts.para_end.unwrap_or(total).min(total);

    let mut paragraphs = Vec::new();
    for p in start..end {
        paragraphs.push(IrParagraph::Text(build_text_paragraph(core, sec, p)));
    }

    IrSlice {
        doc_meta: IrDocMeta {
            edit_session_id: opts.edit_session_id.clone().unwrap_or_else(|| format!("ed_{}", chrono::Utc::now().timestamp_millis())),
            page: 1,
            total_pages: 1,
            anchor: IrAnchor { sec, para_start: start, para_end: end },
        },
        paragraphs,
    }
}
```

`chrono` 미사용 시 — `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` 로 대체. server 의 Cargo.toml 확인.

- [ ] **Step 2: unit test**

```rust
#[test]
fn build_ir_slice_blank_doc_one_paragraph() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    let slice = build_ir_slice(&core, &BuildOptions {
        sec: 0, para_start: 0, para_end: None, edit_session_id: Some("test".into()),
    });
    assert_eq!(slice.doc_meta.anchor.sec, 0);
    assert!(slice.paragraphs.len() >= 1);
    assert_eq!(slice.doc_meta.edit_session_id, "test");
}
```

Run: `cd server && cargo test ir_compact::tests::build_ir_slice_blank_doc_one_paragraph`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: build_ir_slice 진입점 + 텍스트 path 동작 검증"
```

---

## Phase 4 — 표·셀 처리

### Task 4.1 — build_cell_paragraph (셀 안 문단)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
fn build_cell_paragraph(
    core: &rhwp::document_core::DocumentCore,
    sec: usize,
    parent_para: usize,
    control_idx: usize,
    cell_idx: usize,
    cell_para: usize,
    cell_row: u16,
    cell_col: u16,
) -> IrTextParagraph {
    use rhwp::renderer::style_resolver::detect_lang_category;

    let cell_para_ref = core.get_cell_paragraph_ref(sec, parent_para, control_idx, cell_idx, cell_para);
    let (text, para_shape_id) = cell_para_ref
        .map(|p| (p.text.clone(), p.para_shape_id))
        .unwrap_or_default();
    let len = text.chars().count();

    let para_style = core.styles.para_styles
        .get(para_shape_id as usize)
        .map(para_shape_to_para_style)
        .unwrap_or_default();

    let runs = collect_runs(&text, len, |off| {
        if let Some(p) = cell_para_ref {
            let id = p.char_shape_id_at(off).unwrap_or(0) as usize;
            let resolved = core.styles.char_styles.get(id);
            let raw = core.document.doc_info.char_shapes.get(id);
            if let (Some(rs), Some(rw)) = (resolved, raw) {
                let lang = text.chars().nth(off).map(detect_lang_category).unwrap_or(0);
                return char_shape_to_run_style(rs, rw, lang);
            }
        }
        RunStyle::default()
    });

    IrTextParagraph {
        id: format!("p_{}_{}_c{}_{}_{}", sec, parent_para, control_idx, cell_idx, cell_para),
        sec,
        para: -1,
        kind: "text",
        style: para_style,
        runs,
        cell_locator: Some(CellLocator {
            table_para: parent_para,
            row: cell_row,
            col: cell_col,
            cell_para,
        }),
    }
}
```

`core.get_cell_paragraph_ref` 가 `pub` 인지 확인 — 아니라면 `pub(crate)` 일 수 있음. server 에서 접근 가능하도록 *DocumentCore native 메서드들이 pub* 이라는 spec §2.2 정합을 빌드 시 검증. 안 되면 `core.document.sections[sec].paragraphs[parent_para].controls[control_idx]` 의 `Control::Table(t).cells[cell_idx].paragraphs[cell_para]` 로 직접 접근.

- [ ] **Step 2: unit test — insert_table + replace_cell_runs**

```rust
#[test]
fn build_cell_paragraph_with_text() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_table_native(0, 0, 2, 2).unwrap();  // 2x2 표
    let table_para = 0;  // insert_table 가 어디에 표를 박는지 확인 필요 — 빈 문서 + insert_table 이면 보통 같은 문단
    core.replace_cell_runs_native(0, table_para, 0, 0, 0, r#"[{"text": "HELLO"}]"#).unwrap();
    let cell = build_cell_paragraph(&core, 0, table_para, 0, 0, 0, 0, 0);
    assert_eq!(cell.para, -1);
    assert_eq!(cell.cell_locator.as_ref().unwrap().table_para, table_para);
    assert_eq!(cell.cell_locator.as_ref().unwrap().row, 0);
    assert_eq!(cell.runs.iter().map(|r| r.text.as_str()).collect::<String>(), "HELLO");
}
```

Run: `cd server && cargo test ir_compact::tests::build_cell_paragraph_with_text`
Expected: pass. 실패 시 — Cell 구조 직접 접근 fallback 사용.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: build_cell_paragraph + cell_locator 채움"
```

### Task 4.2 — build_table_paragraph + try_build_cell

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
fn try_build_cell(
    core: &rhwp::document_core::DocumentCore,
    sec: usize,
    parent_para: usize,
    control_idx: usize,
    cell_idx: usize,
) -> Option<IrTableCell> {
    // get_cell_info_native 의 결과 — JSON {row, col, rowSpan, colSpan}
    let info_json = core.get_cell_info_native(sec, parent_para, control_idx, cell_idx).ok()?;
    let info: serde_json::Value = serde_json::from_str(&info_json).ok()?;
    let row = info["row"].as_u64()? as u16;
    let col = info["col"].as_u64()? as u16;
    let row_span = info["rowSpan"].as_u64().unwrap_or(1) as u16;
    let col_span = info["colSpan"].as_u64().unwrap_or(1) as u16;

    // get_cell_properties_native — JSON {fillColor, width, height, borderLeft, ...}
    let props_json = core.get_cell_properties_native(sec, parent_para, control_idx, cell_idx).ok()?;
    let style = cell_style_from_json(&props_json);

    let cpc = core.get_cell_paragraph_count_native(sec, parent_para, control_idx, cell_idx).unwrap_or(0);
    let mut paragraphs = Vec::with_capacity(cpc);
    for cp in 0..cpc {
        paragraphs.push(IrParagraph::Text(build_cell_paragraph(core, sec, parent_para, control_idx, cell_idx, cp, row, col)));
    }

    Some(IrTableCell {
        row, col,
        row_span: if row_span > 1 { Some(row_span) } else { None },
        col_span: if col_span > 1 { Some(col_span) } else { None },
        style,
        paragraphs,
    })
}

/// get_cell_properties_native 의 JSON 응답 → CellStyle 변환.
/// (cell_to_cell_style 와 동등하지만 입력이 JSON 인 path — native 메서드 통합 일관성)
fn cell_style_from_json(json: &str) -> CellStyle {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::Value::Null);
    // fillColor·width·height·borderLeft/Right/Top/Bottom·verticalAlign 추출.
    // 자세한 키는 get_cell_properties_native 출력 형식 (formatting.rs / table_ops.rs:441 부근) 확인.
    let mut style = CellStyle::default();
    if let Some(s) = v.get("fillColor").and_then(|x| x.as_str()) { style.bgcolor = Some(s.to_string()); }
    if let Some(n) = v.get("width").and_then(|x| x.as_i64()) { style.width = Some(n as i32); }
    if let Some(n) = v.get("height").and_then(|x| x.as_i64()) { style.height = Some(n as i32); }
    if let Some(n) = v.get("verticalAlign").and_then(|x| x.as_u64()) {
        style.vertical_align = Some(cell_vertical_align_to_str(n as u8).to_string());
    }
    // border 4면 — JSON 형식 확인 후 매핑
    style
}

fn build_table_paragraph(
    core: &rhwp::document_core::DocumentCore,
    sec: usize,
    para: usize,
    control_idx: usize,
) -> Option<IrTableParagraph> {
    let dims_json = core.get_table_dimensions_native(sec, para, control_idx).ok()?;
    let dims: serde_json::Value = serde_json::from_str(&dims_json).ok()?;
    let rows = dims["rowCount"].as_u64()? as u16;
    let cols = dims["colCount"].as_u64()? as u16;
    let cell_count = dims["cellCount"].as_u64()? as usize;

    let mut cells = Vec::with_capacity(cell_count);
    for cell_idx in 0..cell_count {
        if let Some(c) = try_build_cell(core, sec, para, control_idx, cell_idx) {
            cells.push(c);
        }
    }

    Some(IrTableParagraph {
        id: format!("p_{}_{}", sec, para),
        sec, para,
        kind: "table",
        rows, cols,
        cells,
    })
}
```

`get_cell_properties_native` 응답 JSON 의 정확한 키 (fillColor / width / height / borderLeft …) 는 [src/document_core/commands/table_ops.rs:441](src/document_core/commands/table_ops.rs#L441) 의 본문에서 확인. border 4면 매핑은 그 형식 따라 작성.

- [ ] **Step 2: unit test — 2×2 표 빌드**

```rust
#[test]
fn build_table_paragraph_2x2() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_table_native(0, 0, 2, 2).unwrap();
    let table = build_table_paragraph(&core, 0, 0, 0).expect("table built");
    assert_eq!(table.kind, "table");
    assert_eq!(table.rows, 2);
    assert_eq!(table.cols, 2);
    assert_eq!(table.cells.len(), 4);
}
```

Run: `cd server && cargo test ir_compact::tests::build_table_paragraph_2x2`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: build_table_paragraph + try_build_cell + 2x2 표 검증"
```

### Task 4.3 — build_paragraph 분기 (control 검사) + cell 평탄 entry 추가

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: build_paragraph 분기 함수 + build_ir_slice 갱신**

`init.md §2` 의 예제는 *table 문단* 과 *cell 안 문단* 을 *동시에 paragraphs[] 안에 평탄* 으로 둠 (table 본체는 cells nested 도 가짐). 둘 다 채움.

```rust
fn build_paragraph(core: &rhwp::document_core::DocumentCore, sec: usize, para: usize) -> Vec<IrParagraph> {
    let p = &core.document.sections[sec].paragraphs[para];
    // 표 control 검색
    let mut out = Vec::new();
    for (ci, ctrl) in p.controls.iter().enumerate() {
        if matches!(ctrl, rhwp::model::control::Control::Table(_)) {
            if let Some(table) = build_table_paragraph(core, sec, para, ci) {
                // table 본체 + 모든 셀 paragraph 의 평탄 entry
                let cell_paras = flatten_cell_paragraphs(&table, sec);
                out.push(IrParagraph::Table(table));
                out.extend(cell_paras);
                return out;  // 한 문단에 표 하나가 표준 — 본 작업 범위 안에서 첫 표만
            }
        }
    }
    out.push(IrParagraph::Text(build_text_paragraph(core, sec, para)));
    out
}

fn flatten_cell_paragraphs(table: &IrTableParagraph, sec: usize) -> Vec<IrParagraph> {
    let mut out = Vec::new();
    for cell in &table.cells {
        for cp in &cell.paragraphs {
            if let IrParagraph::Text(t) = cp {
                out.push(IrParagraph::Text(t.clone()));
            }
            // 셀 안에 표가 또 있는 경우는 본 작업 범위 외 — 추후.
        }
    }
    out
}
```

`pub fn build_ir_slice` 의 paragraph 누적을 `extend(build_paragraph(core, sec, p))` 로 변경:

```rust
for p in start..end {
    paragraphs.extend(build_paragraph(core, sec, p));
}
```

- [ ] **Step 2: unit test — 텍스트 + 표 혼합**

```rust
#[test]
fn build_ir_slice_text_and_table() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_text_native(0, 0, 0, "TITLE").unwrap();
    // 표를 새 문단으로 삽입
    let after_para = 0;
    core.insert_paragraph_native(0, after_para, 1, false, "{}").unwrap();
    core.insert_table_native(0, 1, 2, 2).unwrap();

    let slice = build_ir_slice(&core, &BuildOptions {
        sec: 0, para_start: 0, para_end: None, edit_session_id: None,
    });
    let kinds: Vec<&str> = slice.paragraphs.iter().map(|p| match p {
        IrParagraph::Text(t) => t.kind,
        IrParagraph::Table(t) => t.kind,
    }).collect();
    assert!(kinds.contains(&"text"));
    assert!(kinds.contains(&"table"));
}
```

Run: `cd server && cargo test ir_compact::tests::build_ir_slice_text_and_table`
Expected: pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: build_paragraph 분기 + 셀 평탄 entry"
```

---

## Phase 5 — compact 압축

### Task 5.1 — compute_doc_defaults + mode()

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: mode() 함수 + compute_doc_defaults**

```rust
fn mode<T: Clone + serde::Serialize>(arr: &[T]) -> Option<T> {
    if arr.is_empty() { return None; }
    use std::collections::HashMap;
    let mut counts: HashMap<String, (T, usize)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for v in arr {
        let k = serde_json::to_string(v).ok()?;
        let entry = counts.entry(k.clone()).or_insert_with(|| { order.push(k.clone()); (v.clone(), 0) });
        entry.1 += 1;
    }
    // 동률 시 *먼저 등장한 값* 우선 (ts 원본 동작)
    order.iter()
        .map(|k| counts.get(k).unwrap())
        .max_by_key(|(_, c)| *c)
        .map(|(v, _)| v.clone())
}

fn compute_doc_defaults(ir: &IrSlice) -> DocDefaults {
    let mut sizes: Vec<f64> = Vec::new();
    let mut fonts: Vec<String> = Vec::new();
    fn visit(p: &IrParagraph, sizes: &mut Vec<f64>, fonts: &mut Vec<String>) {
        match p {
            IrParagraph::Text(t) => for r in &t.runs {
                if let Some(s) = r.style.font_size { sizes.push(s); }
                if let Some(f) = &r.style.font_name { fonts.push(f.clone()); }
            },
            IrParagraph::Table(tt) => for cell in &tt.cells {
                for inner in &cell.paragraphs { visit(inner, sizes, fonts); }
            },
        }
    }
    for p in &ir.paragraphs { visit(p, &mut sizes, &mut fonts); }
    DocDefaults {
        run: RunStyle {
            bold: Some(false), italic: Some(false), underline: Some(false), strikethrough: Some(false),
            color: Some("#000000".into()), highlight: None,
            char_spacing: Some(0), char_width: Some(100),
            vertical_align: Some("baseline".into()),
            font_size: Some(mode(&sizes).unwrap_or(10.0)),
            font_name: Some(mode(&fonts).unwrap_or_else(|| "맑은 고딕".into())),
        },
        paragraph: ParagraphStyle {
            align: Some("left".into()),
            indent: Some(0),
            line_height: Some(160),
        },
    }
}
```

*max_by_key* 는 *마지막* 최빈값을 반환할 수 있음 — ts 원본 동작 (*먼저 등장한 값*) 을 위해 동률 시 `order` 인덱스 작은 쪽 우선이 되도록 정렬:

```rust
order.iter().enumerate()
    .map(|(idx, k)| (idx, counts.get(k).unwrap()))
    .max_by(|(ia, a), (ib, b)| a.1.cmp(&b.1).then(ib.cmp(ia)))  // count desc, order asc
    .map(|(_, (v, _))| v.clone())
```

- [ ] **Step 2: unit test**

```rust
#[test]
fn mode_returns_most_frequent() {
    assert_eq!(mode(&vec![1.0, 2.0, 2.0, 3.0]), Some(2.0));
    assert_eq!(mode::<f64>(&vec![]), None);
}

#[test]
fn mode_ties_keep_first() {
    assert_eq!(mode(&vec!["a".to_string(), "b".to_string(), "a".to_string(), "b".to_string()]), Some("a".to_string()));
}

#[test]
fn compute_doc_defaults_from_blank_doc() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    let slice = build_ir_slice(&core, &BuildOptions { sec: 0, para_start: 0, para_end: None, edit_session_id: None });
    let d = compute_doc_defaults(&slice);
    assert_eq!(d.run.bold, Some(false));
    assert_eq!(d.run.color.as_deref(), Some("#000000"));
    assert!(d.run.font_size.unwrap() > 0.0);
    assert_eq!(d.paragraph.align.as_deref(), Some("left"));
}
```

Run: `cd server && cargo test ir_compact::tests::mode_ ir_compact::tests::compute_doc_defaults_`
Expected: 3 pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: compute_doc_defaults + mode() tie-break"
```

### Task 5.2 — omit_defaults

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
/// `style` 의 키 중 *defaults 와 같은 값* 은 제외한 `serde_json::Value` 반환.
/// 결과가 빈 Object 면 None.
fn omit_run_style_defaults(style: &RunStyle, defaults: &RunStyle) -> Option<serde_json::Value> {
    let s_json = serde_json::to_value(style).ok()?;
    let d_json = serde_json::to_value(defaults).ok()?;
    let mut out = serde_json::Map::new();
    if let (serde_json::Value::Object(s_obj), serde_json::Value::Object(d_obj)) = (s_json, d_json) {
        for (k, v) in s_obj {
            if d_obj.get(&k) == Some(&v) { continue; }
            out.insert(k, v);
        }
    }
    if out.is_empty() { None } else { Some(serde_json::Value::Object(out)) }
}

fn omit_para_style_defaults(style: &ParagraphStyle, defaults: &ParagraphStyle) -> Option<serde_json::Value> {
    let s_json = serde_json::to_value(style).ok()?;
    let d_json = serde_json::to_value(defaults).ok()?;
    let mut out = serde_json::Map::new();
    if let (serde_json::Value::Object(s_obj), serde_json::Value::Object(d_obj)) = (s_json, d_json) {
        for (k, v) in s_obj {
            if d_obj.get(&k) == Some(&v) { continue; }
            out.insert(k, v);
        }
    }
    if out.is_empty() { None } else { Some(serde_json::Value::Object(out)) }
}
```

- [ ] **Step 2: unit test**

```rust
#[test]
fn omit_defaults_drops_matching_keys() {
    let style = RunStyle {
        bold: Some(false),       // default 와 같음 → 제외
        font_size: Some(22.0),   // default 다름 → 유지
        ..Default::default()
    };
    let defaults = RunStyle {
        bold: Some(false),
        font_size: Some(11.0),
        ..Default::default()
    };
    let out = omit_run_style_defaults(&style, &defaults).unwrap();
    assert!(out.get("bold").is_none());
    assert_eq!(out["font-size"], 22.0);
}

#[test]
fn omit_defaults_all_same_returns_none() {
    let s = RunStyle { bold: Some(false), ..Default::default() };
    let d = RunStyle { bold: Some(false), ..Default::default() };
    assert!(omit_run_style_defaults(&s, &d).is_none());
}
```

Run: `cd server && cargo test ir_compact::tests::omit_defaults_`
Expected: 2 pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: omit_run_style_defaults + omit_para_style_defaults"
```

### Task 5.3 — compact_run + compact_text (단일 run text 직속)

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: 함수 작성**

```rust
fn compact_run(run: &IrRun, defaults: &DocDefaults) -> serde_json::Value {
    let style = omit_run_style_defaults(&run.style, &defaults.run);
    let mut out = serde_json::Map::new();
    out.insert("char_offset".into(), serde_json::json!(run.char_offset));
    out.insert("text".into(), serde_json::json!(run.text));
    if let Some(s) = style {
        out.insert("style".into(), s);
    }
    serde_json::Value::Object(out)
}

fn compact_text(p: &IrTextParagraph, defaults: &DocDefaults) -> serde_json::Value {
    let runs: Vec<serde_json::Value> = p.runs.iter().map(|r| compact_run(r, defaults)).collect();
    let para_style = omit_para_style_defaults(&p.style, &defaults.paragraph);

    let mut out = serde_json::Map::new();
    out.insert("id".into(), serde_json::json!(p.id));
    out.insert("sec".into(), serde_json::json!(p.sec));
    out.insert("para".into(), serde_json::json!(p.para));
    out.insert("type".into(), serde_json::json!("text"));
    if let Some(cl) = &p.cell_locator {
        out.insert("cell_locator".into(), serde_json::to_value(cl).unwrap());
    }
    if let Some(s) = para_style {
        out.insert("style".into(), s);
    }
    // 단일 run + 스타일 없음 → text 직속
    if runs.len() == 1 && runs[0].get("style").is_none() {
        out.insert("text".into(), runs[0]["text"].clone());
    } else {
        out.insert("runs".into(), serde_json::Value::Array(runs));
    }
    serde_json::Value::Object(out)
}
```

- [ ] **Step 2: unit test**

```rust
#[test]
fn compact_text_single_run_text_inline() {
    let t = IrTextParagraph {
        id: "p_0_0".into(),
        sec: 0, para: 0, kind: "text",
        style: ParagraphStyle::default(),
        runs: vec![IrRun { char_offset: 0, length: 3, text: "ABC".into(), style: RunStyle::default() }],
        cell_locator: None,
    };
    let defaults = DocDefaults {
        run: RunStyle::default(),
        paragraph: ParagraphStyle::default(),
    };
    let v = compact_text(&t, &defaults);
    assert!(v.get("runs").is_none(), "runs 가 생략되어야 함");
    assert_eq!(v["text"], "ABC");
}

#[test]
fn compact_text_styled_run_keeps_runs() {
    let t = IrTextParagraph {
        id: "p_0_0".into(),
        sec: 0, para: 0, kind: "text",
        style: ParagraphStyle::default(),
        runs: vec![IrRun {
            char_offset: 0, length: 3, text: "ABC".into(),
            style: RunStyle { bold: Some(true), ..Default::default() },
        }],
        cell_locator: None,
    };
    let defaults = DocDefaults {
        run: RunStyle { bold: Some(false), ..Default::default() },
        paragraph: ParagraphStyle::default(),
    };
    let v = compact_text(&t, &defaults);
    assert!(v.get("text").is_none());
    let runs = v["runs"].as_array().unwrap();
    assert_eq!(runs[0]["style"]["bold"], true);
}
```

Run: `cd server && cargo test ir_compact::tests::compact_text_`
Expected: 2 pass.

- [ ] **Step 3: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: compact_run + compact_text — 단일 run text 직속"
```

### Task 5.4 — compact_border + cell + table + compact_ir_slice 진입

**Files:**
- Modify: `server/src/ir_compact.rs`

- [ ] **Step 1: compact_border + compact_cell + compact_table**

```rust
fn compact_border(border: &CellBorder) -> Option<CellBorder> {
    let sides = [&border.left, &border.right, &border.top, &border.bottom];
    let first = sides[0].clone();
    let all_same = sides.iter().all(|s| **s == first) && first.is_some();
    if all_same {
        return Some(CellBorder {
            left: None, right: None, top: None, bottom: None,
            all: first,
        });
    }
    let mut out = CellBorder::default();
    out.left = border.left.clone();
    out.right = border.right.clone();
    out.top = border.top.clone();
    out.bottom = border.bottom.clone();
    if out.left.is_some() || out.right.is_some() || out.top.is_some() || out.bottom.is_some() {
        Some(out)
    } else {
        None
    }
}

fn compact_cell(cell: &IrTableCell, defaults: &DocDefaults) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    out.insert("row".into(), serde_json::json!(cell.row));
    out.insert("col".into(), serde_json::json!(cell.col));
    if let Some(rs) = cell.row_span { out.insert("row_span".into(), serde_json::json!(rs)); }
    if let Some(cs) = cell.col_span { out.insert("col_span".into(), serde_json::json!(cs)); }
    // cell style 압축
    let mut style = cell.style.clone();
    if let Some(b) = &cell.style.border {
        style.border = compact_border(b);
    }
    let style_v = serde_json::to_value(&style).ok();
    if let Some(s) = style_v {
        if s.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            out.insert("style".into(), s);
        }
    }
    let paras: Vec<serde_json::Value> = cell.paragraphs.iter().map(|p| match p {
        IrParagraph::Text(t) => compact_text(t, defaults),
        IrParagraph::Table(tt) => compact_table(tt, defaults),
    }).collect();
    out.insert("paragraphs".into(), serde_json::Value::Array(paras));
    serde_json::Value::Object(out)
}

fn compact_table(p: &IrTableParagraph, defaults: &DocDefaults) -> serde_json::Value {
    serde_json::json!({
        "id": p.id,
        "sec": p.sec,
        "para": p.para,
        "type": "table",
        "rows": p.rows,
        "cols": p.cols,
        "cells": p.cells.iter().map(|c| compact_cell(c, defaults)).collect::<Vec<_>>(),
    })
}
```

- [ ] **Step 2: compact_ir_slice 진입 함수**

```rust
pub fn compact_ir_slice(ir: IrSlice) -> CompactIrSlice {
    let defaults = compute_doc_defaults(&ir);
    let paragraphs: Vec<serde_json::Value> = ir.paragraphs.iter().map(|p| match p {
        IrParagraph::Text(t) => compact_text(t, &defaults),
        IrParagraph::Table(tt) => compact_table(tt, &defaults),
    }).collect();
    CompactIrSlice {
        doc_meta: ir.doc_meta,
        paragraphs,
        defaults,
    }
}

pub fn build_compact_ir_slice(
    core: &rhwp::document_core::DocumentCore,
    opts: &BuildOptions,
) -> CompactIrSlice {
    compact_ir_slice(build_ir_slice(core, opts))
}
```

- [ ] **Step 3: unit test — 통합**

```rust
#[test]
fn compact_ir_slice_text_only_blank() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_text_native(0, 0, 0, "Hello").unwrap();
    let slice = build_compact_ir_slice(&core, &BuildOptions {
        sec: 0, para_start: 0, para_end: None, edit_session_id: None,
    });
    let v = serde_json::to_value(&slice).unwrap();
    assert!(v["defaults"]["run"]["bold"] == false);
    assert_eq!(v["doc_meta"]["anchor"]["sec"], 0);
    // 단일 run "Hello" → text 직속 (defaults 와 모두 같으면)
    let para0 = &v["paragraphs"][0];
    assert_eq!(para0["type"], "text");
    // defaults 와 모두 같다면 text 직속, 아니면 runs 형태 — 어느 쪽이든 spec 합치
    assert!(para0.get("text").is_some() || para0.get("runs").is_some());
}

#[test]
fn compact_ir_slice_with_table() {
    let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
    let mut core = DocumentCore::from_bytes(bytes, "hwpx").unwrap();
    core.insert_table_native(0, 0, 2, 2).unwrap();
    let slice = build_compact_ir_slice(&core, &BuildOptions {
        sec: 0, para_start: 0, para_end: None, edit_session_id: None,
    });
    let v = serde_json::to_value(&slice).unwrap();
    let types: Vec<String> = v["paragraphs"].as_array().unwrap().iter()
        .map(|p| p["type"].as_str().unwrap_or("").to_string()).collect();
    assert!(types.contains(&"table".to_string()));
}
```

Run: `cd server && cargo test ir_compact::tests::compact_ir_slice_`
Expected: 2 pass.

- [ ] **Step 4: commit**

```bash
git add server/src/ir_compact.rs
git commit -m "Task #zephy-bridge Sub-3: compact_ir_slice 진입 + 압축 4 규칙 통합"
```

---

## Phase 6 — endpoint 분기 교체

### Task 6.1 — ir_slice_handler 의 compact 분기 교체

**Files:**
- Modify: `server/src/main.rs` ([server/src/main.rs:983-1057](server/src/main.rs#L983-L1057))

- [ ] **Step 1: ir_slice_handler 본문 갱신**

`mode` 정책 변경 (spec §7):
```rust
let resolved_mode = match q.mode.as_str() {
    "raw" => "raw",
    _ => "compact",  // "compact" / "auto" / default — 모두 compact
};
```

compact 분기를 `ir_compact::build_compact_ir_slice` 호출로 교체:
```rust
if resolved_mode == "compact" {
    let opts = ir_compact::BuildOptions {
        sec,
        para_start,
        para_end: Some(para_end),
        edit_session_id: Some(format!("cli_{}", file_id)),
    };
    let slice = ir_compact::build_compact_ir_slice(&s.core, &opts);
    let mut v = serde_json::to_value(&slice).unwrap_or(serde_json::Value::Null);
    // spec §3.3 의 호환 — top-level section/para_start/para_end/mode 도 유지 (옛 client)
    if let serde_json::Value::Object(ref mut m) = v {
        m.insert("section".into(), serde_json::json!(sec));
        m.insert("para_start".into(), serde_json::json!(para_start));
        m.insert("para_end".into(), serde_json::json!(para_end));
        m.insert("mode".into(), serde_json::json!("compact"));
    }
    return Ok(Json(v));
}
```

raw 분기는 *현재 코드 그대로 유지*. `5000 자 미만 raw 폴백 제거* — `match q.mode.as_str()` 의 `_ => "compact"` 가 그 자리.

- [ ] **Step 2: 빌드 + 기존 raw 회귀 검증**

```bash
cd server && cargo build
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
./rhwp-studio/e2e/sub2-server.sh restart
node rhwp-studio/e2e/sub2-ir-slice.test.mjs   # 또는 기존 ir-slice e2e
```

Expected: 빌드 성공, raw 모드 e2e 통과.

- [ ] **Step 3: commit**

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-3: ir_slice_handler compact 분기를 ir_compact 호출로 교체"
```

### Task 6.2 — e2e sub3-ir-compact 신규

**Files:**
- Create: `rhwp-studio/e2e/sub3-ir-compact.test.mjs`

- [ ] **Step 1: e2e 스크립트 작성**

`rhwp-studio/e2e/sub3-ir-compact.test.mjs`:
```javascript
// Sub-3 IR compact 응답 검증.
// 텍스트 + bold + 표 + 셀 텍스트가 모두 spec §3.1 형식으로 나오는지.

import { newFileId, createSession, postWorkbench, getIrSlice } from './sub2-helpers.mjs';
import assert from 'node:assert/strict';

const BASE = 'http://127.0.0.1:7710';

async function main() {
  const fid = newFileId();
  await createSession(BASE, fid);

  await postWorkbench(BASE, fid, 'insert_text',  { section: 0, para: 0, offset: 0, text: 'A' });
  await postWorkbench(BASE, fid, 'set_char_shape', { section: 0, para: 0, char_start: 0, char_end: 1, style: { bold: true } });
  await postWorkbench(BASE, fid, 'insert_paragraph', { section: 0, after_para: 0, count: 1 });
  await postWorkbench(BASE, fid, 'insert_table', { section: 0, insert_after_para: 1, rows: 2, cols: 2 });
  await postWorkbench(BASE, fid, 'replace_cell_runs', {
    section: 0, table_para: 2, row: 0, col: 0, cell_para: 0,
    runs: [{ text: 'CELL_TEXT' }],
  });

  // compact 모드 호출
  const compact = await getIrSlice(BASE, fid, { mode: 'compact' });
  console.log('compact:', JSON.stringify(compact, null, 2).slice(0, 800));

  // 검증 1 — defaults 박스 존재
  assert.ok(compact.defaults, 'defaults 박스 없음');
  assert.equal(compact.defaults.run.bold, false);
  assert.equal(compact.defaults.run.color, '#000000');
  assert.equal(compact.defaults.paragraph.align, 'left');

  // 검증 2 — 첫 문단 'A' 가 bold (single run with style)
  const para0 = compact.paragraphs.find(p => p.para === 0);
  assert.ok(para0, 'para 0 없음');
  assert.equal(para0.type, 'text');
  assert.ok(para0.runs || para0.text, 'runs 도 text 도 없음');
  if (para0.runs) {
    assert.equal(para0.runs[0].style?.bold, true, 'bold 누락');
  }

  // 검증 3 — 표 문단 (rows/cols)
  const table = compact.paragraphs.find(p => p.type === 'table');
  assert.ok(table, '표 문단 없음');
  assert.equal(table.rows, 2);
  assert.equal(table.cols, 2);
  assert.equal(table.cells.length, 4);

  // 검증 4 — 셀 (0,0) 안에 'CELL_TEXT'
  const cell00 = table.cells.find(c => c.row === 0 && c.col === 0);
  assert.ok(cell00, '셀 (0,0) 없음');
  const cellPara = cell00.paragraphs[0];
  const cellText = cellPara.text ?? cellPara.runs?.map(r => r.text).join('');
  assert.equal(cellText, 'CELL_TEXT');

  // 검증 5 — 셀 평탄 entry — paragraphs[] 안에 cell_locator 가 있는 entry 존재
  const cellEntry = compact.paragraphs.find(p => p.cell_locator);
  assert.ok(cellEntry, 'cell_locator 평탄 entry 없음');
  assert.equal(cellEntry.cell_locator.table_para, 2);
  assert.equal(cellEntry.cell_locator.row, 0);
  assert.equal(cellEntry.cell_locator.col, 0);

  // 검증 6 — raw 모드 호환 (기존 e2e 회귀 0)
  const raw = await getIrSlice(BASE, fid, { mode: 'raw' });
  assert.equal(raw.mode, 'raw');
  assert.ok(Array.isArray(raw.paragraphs));

  console.log('✓ sub3-ir-compact PASS');
}

main().catch(e => { console.error(e); process.exit(1); });
```

`sub2-helpers.mjs` 에 `getIrSlice` 가 없으면 추가:
```javascript
// sub2-helpers.mjs 끝부분
export async function getIrSlice(base, fileId, opts = {}) {
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(opts)) qs.set(k, String(v));
  const r = await fetch(`${base}/sessions/${fileId}/ir-slice?${qs}`);
  return r.json();
}
```

- [ ] **Step 2: 실행**

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
./rhwp-studio/e2e/sub2-server.sh restart
node rhwp-studio/e2e/sub3-ir-compact.test.mjs
```

Expected: `✓ sub3-ir-compact PASS`.

- [ ] **Step 3: commit**

```bash
git add rhwp-studio/e2e/sub3-ir-compact.test.mjs rhwp-studio/e2e/sub2-helpers.mjs
git commit -m "Task #zephy-bridge Sub-3: e2e sub3-ir-compact — compact 응답 6 검증"
```

---

## Phase 7 — 노트북 라우터 + 시연

### Task 7.1 — cell 3 `_handle_get_ir_slice` 의 compact 키 변환

**Files:**
- Modify: `hwp_sub_agent_simulation_ssr.ipynb` cell 3

- [ ] **Step 1: `_handle_get_ir_slice` 함수의 query 변환 부분 수정**

cell 3 안 `_handle_get_ir_slice` 의 `if 'mode' in payload:` 뒤에 추가:
```python
if 'mode' in payload:
    query['mode'] = str(payload['mode'])
elif 'compact' in payload:
    # init.md §1: 디버깅용 raw 가 정말 필요하면 {"compact": false} 명시.
    # compact: false → mode=raw, 그 외 (true 또는 키 없음) → default(compact).
    query['mode'] = 'raw' if payload['compact'] is False else 'compact'
```

`'compact' in payload` 가 false 이고 mode 도 없으면 server 가 default(compact). spec §7 정합.

- [ ] **Step 2: cell 4 self-test 재실행**

Jupyter 또는 nbexecute 로 cell 1~4 실행:
- cell 1: 세션 생성
- cell 3: 라우터 재정의
- cell 4: `hwp-doc-patch insert_text` self-test
Expected: "self-test OK" + 브라우저 화면 갱신.

- [ ] **Step 3: cell 4 다음에 ir-slice 검증 한 줄 추가** (선택)

```python
# 임시 검증 — compact 응답에 defaults 박스 + paragraphs[] 가 있는지.
_ir = run_bash_command(f'hwp-doc-patch get-ir-slice --file-id {SESSION_FILE_ID} --payload \'{{}}\' ')
print(_ir['stdout'][:500])
assert '"defaults"' in _ir['stdout'], 'compact 응답 defaults 누락'
print('compact IR self-test OK')
```

- [ ] **Step 4: commit**

```bash
# 노트북은 작업 공간 루트의 git 외부 — 변경은 보고서에만 기록
# 그러나 e2e 호환 정보 (helpers 변경 등) 가 같이 들어가면 같은 commit
git add UNIVA-rhwp/mydocs/working/task_m200_zephy_bridge_sub3_stage7.md  # stage 보고서 작성 시
git commit -m "Task #zephy-bridge Sub-3: 노트북 라우터 compact 키 변환 + self-test"
```

### Task 7.2 — 사용자 수동 시연 안내

**Files:** 없음 (안내만)

- [ ] **Step 1: 서버 재가동**

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
./rhwp-studio/e2e/sub2-server.sh restart
```

- [ ] **Step 2: 표가 있는 hwp 파일 준비**

`samples/` 또는 `pdf/` 에 표 포함 sample 1개 (예: `samples/hwpx/sample_with_table.hwpx`) 확인. 없으면 직접 생성.

- [ ] **Step 3: 시연 시나리오**

1. 브라우저 (시크릿 탭) — `http://127.0.0.1:7710/?fileId=sim-XXXX`
2. 파일 열기 → 표가 있는 hwp 로드
3. 노트북 cell 1 의 fileId 출력 확인 (또는 새 cell 로 기존 fileId 조회)
4. cell 6 의 sub_agent_run 호출 — 예시 prompt:
   ```python
   await sub_agent_run(
       "표의 (0,0) 셀에 '제목' 텍스트를 굵게 입력해줘.",
       file_id=SESSION_FILE_ID,
   )
   ```
5. 모델 출력 확인:
   - `get-ir-slice` 호출 → 응답에 `type:"table"` + `rows`/`cols` + `cell_locator` 보임
   - 모델이 *정확한 cell 좌표* 로 `replace-cell-runs` 호출
6. 브라우저 화면에 *셀 텍스트* 즉시 등장 확인

- [ ] **Step 4: 결과 보고서 작성**

`UNIVA-rhwp/mydocs/working/task_m200_zephy_bridge_sub3_stage7.md` 에 시연 결과 + 모델 출력 발췌 + 브라우저 스크린샷 경로 기록.

```bash
git add UNIVA-rhwp/mydocs/working/task_m200_zephy_bridge_sub3_stage7.md
git commit -m "Task #zephy-bridge Sub-3: 수동 시연 결과 — 표 셀 IR 받아 편집 통과"
```

---

## 검증 종합 (DoD 7 조건)

각 phase 끝에서 DoD 1 조건씩 누적 충족. 마지막 점검:

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
cd server && cargo test ir_compact::tests        # unit 모두 통과
cd ..
./rhwp-studio/e2e/sub2-server.sh restart
node rhwp-studio/e2e/sub2-ir-slice.test.mjs       # raw 회귀 0
node rhwp-studio/e2e/sub3-ir-compact.test.mjs     # compact 검증
node rhwp-studio/e2e/ws-bridge.test.mjs           # Sub-1 broadcast 회귀
node rhwp-studio/e2e/sub2-canvas-insert-text.test.mjs  # Sub-2 시각 회귀
```

- [ ] 모든 test 통과
- [ ] 사용자 수동 시연 통과 보고 받음
- [ ] 최종 보고서 작성: `mydocs/report/task_m200_zephy_bridge_sub3_report.md`

```bash
git add UNIVA-rhwp/mydocs/report/task_m200_zephy_bridge_sub3_report.md
git commit -m "Task #zephy-bridge Sub-3: 최종 결과 보고서"
```

---

## 회귀·호환성 점검 (구현 도중 발생 가능 이슈)

| 사항 | 대응 |
|---|---|
| `DocumentCore::get_cell_paragraph_ref` 가 `pub(crate)` 라 server 에서 호출 안 되면 | *Cell 구조 직접 접근* fallback — `paragraph.controls[ci]` 에서 `Control::Table(t)` 매치 후 `t.cells[cell_idx].paragraphs[cell_para]` |
| `color_ref_to_css` 도 `pub(crate)` — server 가 사용 불가 | server 측에 복제 (8줄 format!). spec §5.5 |
| `ResolvedCharStyle` 의 정확한 type 이름·필드 path 가 spec 과 다를 수 있음 | 빌드 에러 따라 정정. *값 변환 결과* 가 ts 원본과 일치해야 invariant |
| `chrono` 미사용 → `edit_session_id` timestamp 생성 어려움 | `std::time::SystemTime + UNIX_EPOCH` 또는 `format!("ed_{}", fileId)` 로 대체 |
| `insert_table_native` / `insert_paragraph_native` 의 정확한 argument 시그니처 | Sub-2 의 task 매핑표 ([task_m200_zephy_bridge_sub2_impl.md](task_m200_zephy_bridge_sub2_impl.md) Phase 2c) 참조 |
| ts 원본의 `mode()` 동률 처리 | `max_by` 로 *order index asc + count desc* (Phase 5.1 Step 1 코드 참조) |
| 셀 평탄 entry 가 중복 (table.cells nested + 평탄 entry) | spec §3.1 예제가 *둘 다* 보여줌 — *모델 입장 두 path 모두 유효*. 회귀 검증은 cell_locator 평탄 entry 존재만 (Phase 6.2 검증 5) |

---

## 참고

- Spec: [task_m200_zephy_bridge_sub3.md](task_m200_zephy_bridge_sub3.md)
- 옛 ts 원본: [rhwp/rhwp-studio/src/llm-replay/ir-builder.ts](../../../rhwp/rhwp-studio/src/llm-replay/ir-builder.ts) · [style-map.ts](../../../rhwp/rhwp-studio/src/llm-replay/style-map.ts) · [types.ts](../../../rhwp/rhwp-studio/src/llm-replay/types.ts)
- 모델 가이드: [init.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md)
- DocumentCore native 메서드: [src/document_core/commands/formatting.rs](../../src/document_core/commands/formatting.rs) · [table_ops.rs](../../src/document_core/commands/table_ops.rs) · [text_editing.rs](../../src/document_core/commands/text_editing.rs)
- 기존 ir-slice endpoint: [server/src/main.rs:983-1057](../../server/src/main.rs#L983-L1057)
- Sub-2 의 helper / 서버 가동 스크립트: [rhwp-studio/e2e/sub2-helpers.mjs](../../rhwp-studio/e2e/sub2-helpers.mjs) · [sub2-server.sh](../../rhwp-studio/e2e/sub2-server.sh)
