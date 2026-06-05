//! 모델(AI) 조회용 Document IR JSON 뷰
//!
//! `GET /sessions/{fileId}/ir` 가 반환하는 **안정적 조회 스키마**다.
//! 내부 IR(`Document`)을 직접 직렬화하지 않고, 모델이 읽기 좋은 형태로 투영한다.
//! - 라운드트립용 raw 바이트/캐시 필드 제외
//! - 텍스트·문단 메타·글자모양 런·컨트롤 종류 요약 중심
//!
//! 내부 IR 구조가 바뀌어도 이 뷰의 스키마는 `IR_VIEW_SCHEMA_VERSION` 으로 버전 관리한다.

use serde::Serialize;

use super::control::Control;
use super::document::Document;

/// IR 뷰 스키마 버전. 호환 불가 변경 시 증가시킨다.
pub const IR_VIEW_SCHEMA_VERSION: u32 = 1;

/// 문서 전체 조회 뷰
#[derive(Debug, Clone, Serialize)]
pub struct DocumentIrView {
    pub schema_version: u32,
    pub section_count: usize,
    /// 페이지 필터로 조회한 경우 그 페이지 번호(0-based). 전체 조회면 생략.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// 문서 전체 페이지 수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
    pub sections: Vec<SectionView>,
}

/// 구역 조회 뷰
#[derive(Debug, Clone, Serialize)]
pub struct SectionView {
    pub index: usize,
    pub paragraph_count: usize,
    pub paragraphs: Vec<ParagraphView>,
}

/// 문단 조회 뷰
#[derive(Debug, Clone, Serialize)]
pub struct ParagraphView {
    pub index: usize,
    pub text: String,
    pub char_count: u32,
    pub para_shape_id: u16,
    pub style_id: u8,
    /// 글자모양 런 (start_pos → char_shape_id)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub char_runs: Vec<CharRunView>,
    /// 문단에 포함된 컨트롤(표/그림/도형 등) 요약
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<ControlView>,
}

/// 글자모양 런 뷰
#[derive(Debug, Clone, Serialize)]
pub struct CharRunView {
    pub start: u32,
    pub char_shape_id: u32,
}

/// 컨트롤 요약 뷰
#[derive(Debug, Clone, Serialize)]
pub struct ControlView {
    /// 컨트롤 종류 ("table", "picture", "shape" 등)
    pub kind: &'static str,
    /// 표인 경우 행 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
    /// 표인 경우 열 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
}

impl Document {
    /// 모델 조회용 IR 뷰를 생성한다(전체 문단).
    pub fn to_ir_view(&self) -> DocumentIrView {
        let sections = self
            .sections
            .iter()
            .enumerate()
            .map(|(si, section)| SectionView {
                index: si,
                paragraph_count: section.paragraphs.len(),
                paragraphs: section
                    .paragraphs
                    .iter()
                    .enumerate()
                    .map(|(pi, para)| paragraph_view(pi, para))
                    .collect(),
            })
            .collect();

        DocumentIrView {
            schema_version: IR_VIEW_SCHEMA_VERSION,
            section_count: self.sections.len(),
            page: None,
            page_count: None,
            sections,
        }
    }

    /// `keep(section_idx, para_idx)` 가 true인 문단만 포함하는 IR 뷰를 생성한다.
    ///
    /// 페이지 필터 조회에 사용한다. 포함된 문단의 `index`(section/paragraph)는
    /// **절대 인덱스 그대로** 유지되므로, 모델이 본 좌표로 그대로 편집 op를 보낼 수 있다.
    /// (`paragraph_count` 도 섹션 전체 문단 수를 유지한다.)
    pub fn to_ir_view_filtered(
        &self,
        keep: &std::collections::HashSet<(usize, usize)>,
    ) -> DocumentIrView {
        let sections = self
            .sections
            .iter()
            .enumerate()
            .filter_map(|(si, section)| {
                let paragraphs: Vec<ParagraphView> = section
                    .paragraphs
                    .iter()
                    .enumerate()
                    .filter(|(pi, _)| keep.contains(&(si, *pi)))
                    .map(|(pi, para)| paragraph_view(pi, para))
                    .collect();
                if paragraphs.is_empty() {
                    None
                } else {
                    Some(SectionView {
                        index: si,
                        paragraph_count: section.paragraphs.len(),
                        paragraphs,
                    })
                }
            })
            .collect();

        DocumentIrView {
            schema_version: IR_VIEW_SCHEMA_VERSION,
            section_count: self.sections.len(),
            page: None,
            page_count: None,
            sections,
        }
    }

    /// 모델 조회용 IR 뷰를 JSON 문자열로 직렬화한다(전체 문단).
    pub fn to_ir_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.to_ir_view())
    }
}

/// 단일 문단을 조회 뷰로 변환한다(인덱스는 절대값).
fn paragraph_view(pi: usize, para: &super::paragraph::Paragraph) -> ParagraphView {
    ParagraphView {
        index: pi,
        text: para.text.clone(),
        char_count: para.char_count,
        para_shape_id: para.para_shape_id,
        style_id: para.style_id,
        char_runs: para
            .char_shapes
            .iter()
            .map(|cs| CharRunView {
                start: cs.start_pos,
                char_shape_id: cs.char_shape_id,
            })
            .collect(),
        controls: para.controls.iter().map(control_view).collect(),
    }
}

/// 컨트롤을 조회 뷰로 변환한다.
fn control_view(c: &Control) -> ControlView {
    match c {
        Control::Table(t) => ControlView {
            kind: "table",
            rows: Some(t.row_count),
            cols: Some(t.col_count),
        },
        _ => ControlView {
            kind: control_kind(c),
            rows: None,
            cols: None,
        },
    }
}

/// 컨트롤 종류 식별자.
fn control_kind(c: &Control) -> &'static str {
    match c {
        Control::SectionDef(_) => "section_def",
        Control::ColumnDef(_) => "column_def",
        Control::Table(_) => "table",
        Control::Shape(_) => "shape",
        Control::Picture(_) => "picture",
        Control::Header(_) => "header",
        Control::Footer(_) => "footer",
        Control::Footnote(_) => "footnote",
        Control::Endnote(_) => "endnote",
        Control::AutoNumber(_) => "auto_number",
        Control::NewNumber(_) => "new_number",
        Control::PageNumberPos(_) => "page_number_pos",
        Control::Bookmark(_) => "bookmark",
        Control::Hyperlink(_) => "hyperlink",
        Control::Ruby(_) => "ruby",
        Control::CharOverlap(_) => "char_overlap",
        Control::PageHide(_) => "page_hide",
        Control::HiddenComment(_) => "hidden_comment",
        Control::Equation(_) => "equation",
        Control::Field(_) => "field",
        Control::Form(_) => "form",
        Control::Unknown(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::paragraph::{CharShapeRef, Paragraph};
    use crate::model::document::Section;

    #[test]
    fn test_to_ir_view_text_and_meta() {
        let mut doc = Document::default();
        let para = Paragraph {
            char_count: 5,
            para_shape_id: 3,
            style_id: 1,
            text: "안녕하세요".to_string(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 7,
            }],
            ..Default::default()
        };
        doc.sections.push(Section {
            paragraphs: vec![para],
            ..Default::default()
        });

        let view = doc.to_ir_view();
        assert_eq!(view.schema_version, IR_VIEW_SCHEMA_VERSION);
        assert_eq!(view.section_count, 1);
        assert_eq!(view.sections[0].paragraph_count, 1);
        let pv = &view.sections[0].paragraphs[0];
        assert_eq!(pv.text, "안녕하세요");
        assert_eq!(pv.char_count, 5);
        assert_eq!(pv.para_shape_id, 3);
        assert_eq!(pv.style_id, 1);
        assert_eq!(pv.char_runs.len(), 1);
        assert_eq!(pv.char_runs[0].char_shape_id, 7);
    }

    #[test]
    fn test_to_ir_view_filtered_keeps_absolute_index() {
        use std::collections::HashSet;
        let mut doc = Document::default();
        doc.sections.push(Section {
            paragraphs: vec![
                Paragraph { text: "A".into(), char_count: 1, ..Default::default() },
                Paragraph { text: "B".into(), char_count: 1, ..Default::default() },
                Paragraph { text: "C".into(), char_count: 1, ..Default::default() },
            ],
            ..Default::default()
        });
        // 문단 인덱스 2만 남긴다.
        let mut keep = HashSet::new();
        keep.insert((0usize, 2usize));
        let v = doc.to_ir_view_filtered(&keep);
        assert_eq!(v.sections.len(), 1);
        // 섹션 전체 문단 수는 유지(절대 맥락 제공)
        assert_eq!(v.sections[0].paragraph_count, 3);
        // 필터된 문단만 포함, index는 절대값 2
        assert_eq!(v.sections[0].paragraphs.len(), 1);
        assert_eq!(v.sections[0].paragraphs[0].index, 2);
        assert_eq!(v.sections[0].paragraphs[0].text, "C");
    }

    #[test]
    fn test_to_ir_json_roundtrip_parse() {
        let mut doc = Document::default();
        doc.sections.push(Section {
            paragraphs: vec![Paragraph {
                text: "hello".to_string(),
                char_count: 5,
                ..Default::default()
            }],
            ..Default::default()
        });

        let json = doc.to_ir_json().expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["schema_version"], IR_VIEW_SCHEMA_VERSION);
        assert_eq!(parsed["sections"][0]["paragraphs"][0]["text"], "hello");
    }
}
