//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

#![allow(dead_code)]  // 구현 진행 중 일시 허용. Phase 5 종료 시 제거.

use serde::Serialize;

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
