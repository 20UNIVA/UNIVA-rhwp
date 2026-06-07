//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

#![allow(dead_code)]  // 구현 진행 중 일시 허용. Phase 5 종료 시 제거.

use rhwp::model::style::Alignment;
use rhwp::model::ColorRef;
use serde::Serialize;

/// `ColorRef` (0x00BBGGRR `u32`) → CSS hex 문자열 `"#RRGGBB"`.
///
/// 본체 `src/document_core/helpers.rs::color_ref_to_css` 가 `pub(crate)` 이라 server crate
/// 에서 직접 호출 불가 — *결과가 동일* 한 변환을 server 측에 복제. 본체는 소문자 hex (`#ffc107`)
/// 를 출력하지만 init.md spec 의 IR 응답은 대문자 hex (`#FFC107`) — TypeScript 원본
/// (`rhwp-studio/src/llm-replay/style-map.ts`) 정합. 본체 helper 와 *대소문자만* 다르므로
/// 본체 helper 의 결과를 그대로 받는 호출자는 `.to_ascii_uppercase()` 1 줄로 정합.
fn color_ref_to_css(color: ColorRef) -> String {
    // `ColorRef` 는 0x00BBGGRR — R 이 최하위 바이트.
    // 본체 `helpers.rs::color_ref_to_css` 알고리즘 그대로.
    let r = (color & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = ((color >> 16) & 0xFF) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// `Alignment` enum → 소문자 키워드. init.md spec 의 `style.align` 값.
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

/// 글자의 위/아래 첨자 상태 → `style.vertical-align` 값.
/// 둘 다 false 면 `"baseline"`, subscript 가 우선 (HWP 의 CharShape 도 둘이 상호 배타).
fn vertical_align_to_str(subscript: bool, superscript: bool) -> &'static str {
    if subscript {
        "sub"
    } else if superscript {
        "super"
    } else {
        "baseline"
    }
}

/// 셀의 `VerticalAlign` enum (Top=0/Center=1/Bottom=2) → `style.vertical-align` 키워드.
/// 본체는 `rhwp::model::table::VerticalAlign` enum 이지만, init.md spec 은 `top|middle|bottom`
/// 문자열 — Center 를 `middle` 로 매핑 (CSS table 의 vertical-align 관례).
fn cell_vertical_align_to_str(va: rhwp::model::table::VerticalAlign) -> &'static str {
    use rhwp::model::table::VerticalAlign;
    match va {
        VerticalAlign::Top => "top",
        VerticalAlign::Center => "middle",
        VerticalAlign::Bottom => "bottom",
    }
}

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

/// 셀 테두리 — 4면 + `all` 축약 키.
///
/// `all` 은 TypeScript 원본 (`rhwp-studio/src/llm-replay/types.ts::CellBorderSpec`) 에는
/// *없는* compact-단계 확장. Phase 4 빌더는 항상 `None` 으로 채우고, Phase 5 압축이
/// 4면이 동일한 경우에만 `all` 한 칸으로 축약한다.
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
    pub para: i64,
    #[serde(rename = "type")] pub kind: &'static str,
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
    #[serde(rename = "type")] pub kind: &'static str,
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
    pub paragraphs: Vec<serde_json::Value>,
    pub defaults: DocDefaults,
}

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

    #[test]
    fn helper_alignment_to_str() {
        use rhwp::model::style::Alignment;
        assert_eq!(alignment_to_str(Alignment::Justify), "justify");
        assert_eq!(alignment_to_str(Alignment::Left), "left");
        assert_eq!(alignment_to_str(Alignment::Right), "right");
        assert_eq!(alignment_to_str(Alignment::Center), "center");
        assert_eq!(alignment_to_str(Alignment::Distribute), "distribute");
        assert_eq!(alignment_to_str(Alignment::Split), "split");
    }

    #[test]
    fn helper_vertical_align() {
        assert_eq!(vertical_align_to_str(true, false), "sub");
        assert_eq!(vertical_align_to_str(false, true), "super");
        assert_eq!(vertical_align_to_str(false, false), "baseline");
        // 둘 다 true 인 경우 (모델상 상호 배타지만 안전망) — sub 우선.
        assert_eq!(vertical_align_to_str(true, true), "sub");
    }

    #[test]
    fn helper_cell_vertical_align() {
        use rhwp::model::table::VerticalAlign;
        assert_eq!(cell_vertical_align_to_str(VerticalAlign::Top), "top");
        assert_eq!(cell_vertical_align_to_str(VerticalAlign::Center), "middle");
        assert_eq!(cell_vertical_align_to_str(VerticalAlign::Bottom), "bottom");
    }

    #[test]
    fn helper_color_ref() {
        // `ColorRef = u32` (0x00BBGGRR). 빨강 #FF0000 → r=0xFF, g=0x00, b=0x00 → 0x000000FF.
        let red: rhwp::model::ColorRef = 0x000000FF;
        assert_eq!(color_ref_to_css(red), "#FF0000");
        // 노랑 #FFC107 → r=0xFF, g=0xC1, b=0x07 → BGR 순서로 0x0007C1FF.
        let amber: rhwp::model::ColorRef = 0x0007C1FF;
        assert_eq!(color_ref_to_css(amber), "#FFC107");
        // 검정.
        let black: rhwp::model::ColorRef = 0x00000000;
        assert_eq!(color_ref_to_css(black), "#000000");
    }

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
}
