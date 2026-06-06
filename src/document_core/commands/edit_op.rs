//! SSR 세션용 **양방향 편집 연산(EditOperation) 프로토콜**.
//!
//! 클라이언트(WASM)에서 일어난 결정적 편집을 서버 `DocumentCore` 에 동일하게
//! 재현하기 위한 직렬화 가능한 연산 단위다. 각 연산은 정방향(`apply`)과
//! 역방향(`apply_inverse`) 적용을 모두 지원하도록 **inverse 데이터**(삭제 텍스트,
//! 병합 전 문단 길이 등)를 함께 담는다.
//!
//! 적용기는 새 로직을 만들지 않고 기존 `*_native` 편집 메서드를 그대로 호출한다.
//! → 클라이언트 WASM 경로와 서버 native 경로가 **같은 코드**를 거쳐 결정성이 보장된다.
//!
//! 붙여넣기/객체 삽입/표 행·열 편집 등 역연산을 연산으로 표현할 수 없는 작업은
//! 본 프로토콜이 아니라 **전체 스냅샷 동기화**로 처리한다(서버 `PUT /snapshot`).

use serde::{Deserialize, Serialize};

use crate::document_core::DocumentCore;
use crate::error::HwpError;

// ─── Sub-2: Partial 타입 (옵셔널 필드만 직렬화) ─────────────────

/// 본문 문단의 부분 스타일. None 인 필드는 *현재 값 유지* 의미.
/// JSON 직렬화 시 None 은 제외 (`skip_serializing_if`).
/// 직접 `apply_para_format_native(props_json)` 의 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialParagraphStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,   // "left"|"right"|"center"|"justify"|...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_before: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_after: Option<i16>,
}

/// 셀의 부분 스타일. None 인 필드는 *현재 값 유지*.
/// `set_cell_properties_native(json)` 의 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialCellStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,   // "top"|"middle"|"bottom"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_fill_id: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_header: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_protect: Option<bool>,
    // padding/text_direction 은 Sub-3 에서 추가
}

/// run 의 부분 char 스타일. None 인 필드 유지.
/// `apply_char_format_native(props_json)` 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialRunStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_size: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
}

/// run = 텍스트 한 조각 + (선택) 부분 스타일.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSpec {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PartialRunStyle>,
}

/// delete-element 의 element_type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    Paragraph,
    Table,
}

/// 양방향 편집 연산.
///
/// `op` 태그로 구분되는 외부 JSON 프로토콜이다.
/// 위치 인덱스는 모두 0-based.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOperation {
    /// 문단 내 글자 오프셋에 텍스트 삽입.
    InsertText {
        section: usize,
        para: usize,
        offset: usize,
        text: String,
    },
    /// 문단 내 글자 오프셋부터 `count` 글자 삭제.
    /// `deleted_text` 는 역적용(복원)을 위해 삭제된 내용을 보존한다.
    DeleteText {
        section: usize,
        para: usize,
        offset: usize,
        count: usize,
        #[serde(default)]
        deleted_text: String,
    },
    /// `para` 를 `offset` 위치에서 둘로 분할(Enter). 분할 결과 `para+1` 이 생긴다.
    SplitParagraph {
        section: usize,
        para: usize,
        offset: usize,
    },
    /// `para` 를 직전 문단(`para-1`)에 병합(문단 시작에서 Backspace).
    /// `prev_len` 은 병합 전 `para-1` 의 글자 길이(역적용 시 분할 지점).
    MergeParagraph {
        section: usize,
        para: usize,
        prev_len: usize,
    },

    // ─── Sub-2: 신규 12 variants (정방향만, inverse 는 sqlite snapshot stash) ───

    /// 문단 내 runs 를 통째 교체.
    ReplaceRuns {
        section: usize,
        para: usize,
        runs: Vec<RunSpec>,
    },
    /// 문단 부분 스타일 적용. None 필드는 현재 값 유지.
    SetParagraphStyle {
        section: usize,
        para: usize,
        style: PartialParagraphStyle,
    },
    /// 본문 범위 텍스트 삭제 (동문단/다문단 모두). `delete_range_native(cell_ctx=None)` 위임.
    DeleteRange {
        section: usize,
        para_start: usize,
        char_start: usize,
        para_end: usize,
        char_end: usize,
    },
    /// `after_para` *위치*에 빈 문단 `count` 개 삽입 (Enter 와 동일 — 기존 `after_para` 문단이
    /// 뒤로 밀림). 즉 `after_para=0` 호출 시 새 문단이 index 0 으로 들어가고 원래의 첫 문단이 index 1 로.
    /// 옵셔널 style 은 각 신규 문단에 *동일하게* 적용.
    InsertParagraph {
        section: usize,
        after_para: usize,
        #[serde(default = "one_count")]
        count: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialParagraphStyle>,
    },
    /// 문단(또는 그 자리의 표 컨트롤) 삭제. element_type 으로 분기.
    /// *한계: `element_type=table` 분기는 `delete_table_control_native(sec, para, control_idx=0)` 호출 —
    /// 즉 해당 paragraph 의 *첫 표 control* 만 삭제. 한 paragraph 에 여러 표 control 이 있는 경우
    /// 두번째 이후는 별도 호출 필요 (Sub-3 에서 control_idx 옵셔널 필드 추가 검토).*
    DeleteElement {
        section: usize,
        para: usize,
        element_type: ElementType,
    },
    /// 표 삽입. insert_after_para 의 끝(char_offset = para 길이)에 create_table_native 호출.
    InsertTable {
        section: usize,
        insert_after_para: usize,
        rows: u16,
        cols: u16,
    },
    /// 셀 부분 스타일 적용. (row, col) → cell_idx 변환 후 set_cell_properties_native.
    SetCellStyle {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        style: PartialCellStyle,
    },
    /// 표 셀 범위 병합. merge_table_cells_native 위임.
    MergeCells {
        section: usize,
        table_para: usize,
        row_start: usize,
        col_start: usize,
        row_end: usize,
        col_end: usize,
    },
    /// 셀 내 문단 runs 통째 교체. replace_cell_runs_native 위임.
    ReplaceCellRuns {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para: usize,
        runs: Vec<RunSpec>,
    },
    /// 셀 내 텍스트 삽입 (옵셔널 style). insert_text_in_cell_native + 옵셔널 apply_char_format_in_cell_native.
    InsertTextInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para: usize,
        offset: usize,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialRunStyle>,
    },
    /// 셀 내 범위 텍스트 삭제 (동·다문단). delete_range_native(cell_ctx=Some(...)) 위임.
    DeleteRangeInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para_start: usize,
        char_start: usize,
        cell_para_end: usize,
        char_end: usize,
    },
}

fn one_count() -> usize { 1 }

impl DocumentCore {
    /// (row, col) 좌표를 `Table.cells` 의 선형 인덱스로 변환한다.
    /// 셀 단위 편집 variant (SetCellStyle, ReplaceCellRuns, InsertTextInCell, DeleteRangeInCell)
    /// 가 native 호출 전에 사용한다.
    pub fn find_cell_idx(
        &self,
        section_idx: usize,
        table_para_idx: usize,
        control_idx: usize,
        row: u16,
        col: u16,
    ) -> Result<usize, HwpError> {
        let para = self
            .document
            .sections
            .get(section_idx)
            .and_then(|s| s.paragraphs.get(table_para_idx))
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "find_cell_idx: 좌표 부적합 (sec={}, table_para={})",
                    section_idx, table_para_idx
                ))
            })?;
        let table = match para.controls.get(control_idx) {
            Some(crate::model::control::Control::Table(t)) => t,
            _ => {
                return Err(HwpError::RenderError(format!(
                    "find_cell_idx: control_idx={} 가 Table 아님",
                    control_idx
                )))
            }
        };
        table
            .cells
            .iter()
            .position(|c| c.row == row && c.col == col)
            .ok_or_else(|| {
                HwpError::RenderError(format!("find_cell_idx: ({}, {}) 셀 없음", row, col))
            })
    }

    /// 편집 연산을 정방향 적용한다.
    pub fn apply_edit_op(&mut self, op: &EditOperation) -> Result<(), HwpError> {
        match op {
            EditOperation::InsertText {
                section,
                para,
                offset,
                text,
            } => {
                self.insert_text_native(*section, *para, *offset, text)?;
            }
            EditOperation::DeleteText {
                section,
                para,
                offset,
                count,
                ..
            } => {
                self.delete_text_native(*section, *para, *offset, *count)?;
            }
            EditOperation::SplitParagraph {
                section,
                para,
                offset,
            } => {
                self.split_paragraph_native(*section, *para, *offset)?;
            }
            EditOperation::MergeParagraph { section, para, .. } => {
                self.merge_paragraph_native(*section, *para)?;
            }
            EditOperation::ReplaceRuns { section, para, runs } => {
                let runs_json = serde_json::to_string(runs)
                    .map_err(|e| HwpError::RenderError(format!("runs 직렬화: {e}")))?;
                self.replace_runs_native(*section, *para, &runs_json)?;
            }
            EditOperation::SetParagraphStyle { section, para, style } => {
                let props_json = serde_json::to_string(style)
                    .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                self.apply_para_format_native(*section, *para, &props_json)?;
            }
            EditOperation::DeleteRange { section, para_start, char_start, para_end, char_end } => {
                self.delete_range_native(*section, *para_start, *char_start, *para_end, *char_end, None)?;
            }
            EditOperation::InsertParagraph { section, after_para, count, style } => {
                for i in 0..*count {
                    self.insert_paragraph_native(*section, *after_para + i)?;
                    if let Some(s) = style {
                        let props_json = serde_json::to_string(s)
                            .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                        self.apply_para_format_native(*section, *after_para + i + 1, &props_json)?;
                    }
                }
            }
            EditOperation::DeleteElement { section, para, element_type } => {
                match element_type {
                    ElementType::Paragraph => {
                        self.delete_paragraph_native(*section, *para)?;
                    }
                    ElementType::Table => {
                        // delete_table_control_native(section, parent_para, control_idx)
                        self.delete_table_control_native(*section, *para, 0)?;
                    }
                }
            }
            EditOperation::InsertTable { section, insert_after_para, rows, cols } => {
                let para_len = self.document.sections[*section]
                    .paragraphs[*insert_after_para]
                    .text
                    .chars()
                    .count();
                self.create_table_native(*section, *insert_after_para, para_len, *rows, *cols)?;
            }
            EditOperation::SetCellStyle { section, table_para, row, col, style } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let json = serde_json::to_string(style)
                    .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                self.set_cell_properties_native(*section, *table_para, ctrl_idx, cell_idx, &json)?;
            }
            EditOperation::MergeCells { section, table_para, row_start, col_start, row_end, col_end } => {
                let ctrl_idx = 0usize;
                self.merge_table_cells_native(
                    *section, *table_para, ctrl_idx,
                    *row_start as u16, *col_start as u16,
                    *row_end as u16, *col_end as u16,
                )?;
            }
            EditOperation::ReplaceCellRuns { section, table_para, row, col, cell_para, runs } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let runs_json = serde_json::to_string(runs)
                    .map_err(|e| HwpError::RenderError(format!("runs 직렬화: {e}")))?;
                self.replace_cell_runs_native(*section, *table_para, ctrl_idx, cell_idx, *cell_para, &runs_json)?;
            }
            EditOperation::InsertTextInCell { section, table_para, row, col, cell_para, offset, text, style } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let text_len = text.chars().count();
                self.insert_text_in_cell_native(
                    *section, *table_para, ctrl_idx, cell_idx, *cell_para, *offset, text,
                )?;
                if let Some(s) = style {
                    let json = serde_json::to_string(s)
                        .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                    self.apply_char_format_in_cell_native(
                        *section, *table_para, ctrl_idx, cell_idx, *cell_para,
                        *offset, *offset + text_len, &json,
                    )?;
                }
            }
            EditOperation::DeleteRangeInCell { section, table_para, row, col, cell_para_start, char_start, cell_para_end, char_end } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                self.delete_range_native(
                    *section, *cell_para_start, *char_start, *cell_para_end, *char_end,
                    Some((*table_para, ctrl_idx, cell_idx)),
                )?;
            }
        }
        Ok(())
    }

    /// 편집 연산을 역방향 적용한다(undo).
    pub fn apply_inverse_edit_op(&mut self, op: &EditOperation) -> Result<(), HwpError> {
        match op {
            // 삽입의 역 = 같은 위치에서 삽입한 글자 수만큼 삭제
            EditOperation::InsertText {
                section,
                para,
                offset,
                text,
            } => {
                let count = text.chars().count();
                self.delete_text_native(*section, *para, *offset, count)?;
            }
            // 삭제의 역 = 보존해 둔 텍스트를 같은 위치에 재삽입
            EditOperation::DeleteText {
                section,
                para,
                offset,
                deleted_text,
                ..
            } => {
                self.insert_text_native(*section, *para, *offset, deleted_text)?;
            }
            // 분할의 역 = 분할로 생긴 para+1 을 para 에 병합
            EditOperation::SplitParagraph { section, para, .. } => {
                self.merge_paragraph_native(*section, *para + 1)?;
            }
            // 병합의 역 = 병합 대상이던 para-1 을 prev_len 위치에서 다시 분할
            EditOperation::MergeParagraph {
                section,
                para,
                prev_len,
            } => {
                self.split_paragraph_native(*section, *para - 1, *prev_len)?;
            }
            EditOperation::ReplaceRuns { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::SetParagraphStyle { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::DeleteRange { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::InsertParagraph { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::DeleteElement { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::InsertTable { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::SetCellStyle { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::MergeCells { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::ReplaceCellRuns { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::InsertTextInCell { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
            EditOperation::DeleteRangeInCell { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
        }
        Ok(())
    }

    /// 편집 연산 배치를 순차 정방향 적용한다.
    pub fn apply_edit_ops(&mut self, ops: &[EditOperation]) -> Result<(), HwpError> {
        for op in ops {
            self.apply_edit_op(op)?;
        }
        Ok(())
    }

    /// JSON 배열(`[EditOperation, ...]`)을 파싱하여 순차 적용한다.
    pub fn apply_edit_ops_json(&mut self, json: &str) -> Result<(), HwpError> {
        let ops: Vec<EditOperation> = serde_json::from_str(json)
            .map_err(|e| HwpError::RenderError(format!("EditOperation JSON 파싱 실패: {e}")))?;
        self.apply_edit_ops(&ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 빈 문서(섹션1/문단1 + 스타일·페이지네이션 초기화) 위에 텍스트를 올린 코어를 만든다.
    fn core_with_text(text: &str) -> DocumentCore {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        if !text.is_empty() {
            core.insert_text_native(0, 0, 0, text).unwrap();
        }
        core
    }

    fn para_text(core: &DocumentCore, section: usize, para: usize) -> String {
        core.document.sections[section].paragraphs[para].text.clone()
    }

    #[test]
    fn test_insert_text_roundtrip() {
        let mut core = core_with_text("AC");
        let op = EditOperation::InsertText {
            section: 0,
            para: 0,
            offset: 1,
            text: "B".to_string(),
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "ABC");
        core.apply_inverse_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "AC");
    }

    #[test]
    fn test_delete_text_roundtrip() {
        let mut core = core_with_text("ABCDE");
        let op = EditOperation::DeleteText {
            section: 0,
            para: 0,
            offset: 1,
            count: 2,
            deleted_text: "BC".to_string(),
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "ADE");
        core.apply_inverse_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "ABCDE");
    }

    #[test]
    fn test_split_merge_roundtrip() {
        let mut core = core_with_text("HelloWorld");
        let split = EditOperation::SplitParagraph {
            section: 0,
            para: 0,
            offset: 5,
        };
        core.apply_edit_op(&split).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 2);
        assert_eq!(para_text(&core, 0, 0), "Hello");
        assert_eq!(para_text(&core, 0, 1), "World");
        // 역적용 → 다시 한 문단
        core.apply_inverse_edit_op(&split).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 1);
        assert_eq!(para_text(&core, 0, 0), "HelloWorld");
    }

    /// 전체 문서 텍스트(문단 join) — 결정성 비교용.
    fn doc_text(core: &DocumentCore) -> Vec<String> {
        core.document().sections[0]
            .paragraphs
            .iter()
            .map(|p| p.text.clone())
            .collect()
    }

    /// 결정성 회귀: EditOperation 적용 결과 == 동일 시퀀스의 native 직접 호출 결과.
    /// (native 직접 호출은 클라이언트 WASM `insertText`/`splitParagraph` 등이 거치는 경로와 동일)
    #[test]
    fn test_op_apply_equals_direct_native() {
        // (a) op 적용 경로
        let mut a = core_with_text("Hello");
        a.apply_edit_op(&EditOperation::InsertText {
            section: 0,
            para: 0,
            offset: 5,
            text: " World".to_string(),
        })
        .unwrap();
        a.apply_edit_op(&EditOperation::SplitParagraph {
            section: 0,
            para: 0,
            offset: 5,
        })
        .unwrap();

        // (b) native 직접 호출 경로 (= WASM 편집 경로)
        let mut b = core_with_text("Hello");
        b.insert_text_native(0, 0, 5, " World").unwrap();
        b.split_paragraph_native(0, 0, 5).unwrap();

        assert_eq!(doc_text(&a), doc_text(&b), "op 적용과 native 직접 호출 결과가 일치해야 함");
        assert_eq!(doc_text(&a), vec!["Hello".to_string(), " World".to_string()]);
    }

    #[test]
    fn test_apply_ops_json() {
        let mut core = core_with_text("");
        let json = r#"[
            {"op":"insert_text","section":0,"para":0,"offset":0,"text":"가"},
            {"op":"insert_text","section":0,"para":0,"offset":1,"text":"나"}
        ]"#;
        core.apply_edit_ops_json(json).unwrap();
        assert_eq!(para_text(&core, 0, 0), "가나");
    }

    #[test]
    fn test_partial_paragraph_style_serialize_skip_none() {
        let partial = PartialParagraphStyle {
            alignment: Some("right".to_string()),
            line_spacing: None,
            margin_left: None,
            margin_right: None,
            indent: None,
            spacing_before: None,
            spacing_after: None,
        };
        let json = serde_json::to_string(&partial).unwrap();
        assert_eq!(json, r#"{"alignment":"right"}"#);
    }

    #[test]
    fn test_run_spec_deserialize() {
        let json = r#"{"text":"안녕","style":{"bold":true}}"#;
        let run: RunSpec = serde_json::from_str(json).unwrap();
        assert_eq!(run.text, "안녕");
        assert!(run.style.is_some());
    }

    #[test]
    fn test_element_type_serialize() {
        assert_eq!(serde_json::to_string(&ElementType::Paragraph).unwrap(), r#""paragraph""#);
        assert_eq!(serde_json::to_string(&ElementType::Table).unwrap(), r#""table""#);
    }

    #[test]
    fn test_replace_runs_op_apply() {
        let mut core = core_with_text("원본");
        let op = EditOperation::ReplaceRuns {
            section: 0,
            para: 0,
            runs: vec![
                RunSpec { text: "변경".to_string(), style: Some(PartialRunStyle { bold: Some(true), ..Default::default() }) },
                RunSpec { text: " 보통".to_string(), style: None },
            ],
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "변경 보통");
    }

    #[test]
    fn test_replace_runs_op_json_roundtrip() {
        let json = r#"{"op":"replace_runs","section":0,"para":3,"runs":[{"text":"hi","style":{"bold":true}}]}"#;
        let op: EditOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, EditOperation::ReplaceRuns { section: 0, para: 3, .. }));
        let back = serde_json::to_string(&op).unwrap();
        let op2: EditOperation = serde_json::from_str(&back).unwrap();
        assert_eq!(op, op2);
    }

    #[test]
    fn test_set_paragraph_style_op_apply_partial() {
        let mut core = core_with_text("hello");
        let op = EditOperation::SetParagraphStyle {
            section: 0,
            para: 0,
            style: PartialParagraphStyle {
                alignment: Some("right".to_string()),
                ..Default::default()
            },
        };
        core.apply_edit_op(&op).unwrap();
        let result = core.get_para_properties_at_native(0, 0).unwrap();
        assert!(result.contains(r#""alignment":"right""#));
    }

    #[test]
    fn test_set_paragraph_style_op_json() {
        let json = r#"{"op":"set_paragraph_style","section":0,"para":0,"style":{"alignment":"center"}}"#;
        let op: EditOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, EditOperation::SetParagraphStyle { section: 0, para: 0, .. }));
    }

    #[test]
    fn test_delete_range_op_apply_same_para() {
        let mut core = core_with_text("ABCDE");
        let op = EditOperation::DeleteRange {
            section: 0, para_start: 0, char_start: 1, para_end: 0, char_end: 3,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "ADE");
    }

    #[test]
    fn test_delete_range_op_apply_multi_para() {
        let mut core = core_with_text("AAA");
        core.apply_edit_op(&EditOperation::SplitParagraph { section: 0, para: 0, offset: 3 }).unwrap();
        core.insert_text_native(0, 1, 0, "BBB").unwrap();
        let op = EditOperation::DeleteRange {
            section: 0, para_start: 0, char_start: 2, para_end: 1, char_end: 2,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 1);
        assert_eq!(para_text(&core, 0, 0), "AAB");
    }

    #[test]
    fn test_insert_paragraph_op_apply() {
        let mut core = core_with_text("first");
        let op = EditOperation::InsertParagraph {
            section: 0,
            after_para: 0,
            count: 1,
            style: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 2);
    }

    #[test]
    fn test_insert_paragraph_op_default_count() {
        let json = r#"{"op":"insert_paragraph","section":0,"after_para":0}"#;
        let op: EditOperation = serde_json::from_str(json).unwrap();
        if let EditOperation::InsertParagraph { count, .. } = op {
            assert_eq!(count, 1);
        } else { panic!("Wrong variant"); }
    }

    #[test]
    fn test_delete_element_op_apply_paragraph() {
        let mut core = core_with_text("first");
        core.apply_edit_op(&EditOperation::SplitParagraph { section: 0, para: 0, offset: 5 }).unwrap();
        core.insert_text_native(0, 1, 0, "second").unwrap();
        let op = EditOperation::DeleteElement {
            section: 0, para: 0, element_type: ElementType::Paragraph,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 1);
        assert_eq!(para_text(&core, 0, 0), "second");
    }

    #[test]
    fn test_delete_element_op_apply_table() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 3).unwrap();
        // create_table_native 가 빈 문서에서 paragraphs[1] 에 table 배치 (Task 2a.3 발견)
        let op = EditOperation::DeleteElement {
            section: 0, para: 1, element_type: ElementType::Table,
        };
        core.apply_edit_op(&op).unwrap();
        // table 컨트롤 사라짐 확인
        let has_table = core.document.sections[0].paragraphs.iter().any(|p| {
            p.controls.iter().any(|c| matches!(c, crate::model::control::Control::Table(_)))
        });
        assert!(!has_table, "Table 컨트롤이 모두 삭제되어야 함");
    }

    #[test]
    fn test_insert_table_op_apply() {
        let mut core = core_with_text("hello");
        let op = EditOperation::InsertTable {
            section: 0,
            insert_after_para: 0,
            rows: 2,
            cols: 3,
        };
        core.apply_edit_op(&op).unwrap();
        let has_table = core.document.sections[0].paragraphs.iter().any(|p| {
            p.controls.iter().any(|c| matches!(c, crate::model::control::Control::Table(_)))
        });
        assert!(has_table, "Table 컨트롤이 삽입되어야 함");
    }

    #[test]
    fn test_set_cell_style_op_apply() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 2, 2).unwrap();
        // 빈 문서 인라인 표 → table_para = 1 (Task 2a.3 발견)
        let op = EditOperation::SetCellStyle {
            section: 0,
            table_para: 1,
            row: 0,
            col: 0,
            style: PartialCellStyle {
                vertical_align: Some("middle".to_string()),
                ..Default::default()
            },
        };
        core.apply_edit_op(&op).unwrap();
        // set_cell_properties_native 가 panic 안 하면 통과 (호출 자체 검증).
    }

    /// 셀 내부 첫 문단 텍스트를 가져온다.
    fn cell_text(core: &DocumentCore, section: usize, table_para: usize, ctrl: usize, cell_idx: usize, cell_para: usize) -> String {
        let para = &core.document.sections[section].paragraphs[table_para];
        match &para.controls[ctrl] {
            crate::model::control::Control::Table(t) => t.cells[cell_idx].paragraphs[cell_para].text.clone(),
            _ => panic!("Table 컨트롤 아님"),
        }
    }

    #[test]
    fn test_delete_range_in_cell_op_apply() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 1, 1).unwrap();
        core.insert_text_in_cell_native(0, 1, 0, 0, 0, 0, "ABCDE").unwrap();
        let op = EditOperation::DeleteRangeInCell {
            section: 0,
            table_para: 1,
            row: 0,
            col: 0,
            cell_para_start: 0,
            char_start: 1,
            cell_para_end: 0,
            char_end: 3,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(cell_text(&core, 0, 1, 0, 0, 0), "ADE");
    }

    #[test]
    fn test_insert_text_in_cell_op_apply() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 1, 1).unwrap();
        let op = EditOperation::InsertTextInCell {
            section: 0,
            table_para: 1,
            row: 0,
            col: 0,
            cell_para: 0,
            offset: 0,
            text: "셀텍스트".to_string(),
            style: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(cell_text(&core, 0, 1, 0, 0, 0), "셀텍스트");
    }

    #[test]
    fn test_replace_cell_runs_op_apply() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 1, 2).unwrap();
        let ctrl_idx = 0;
        let cell_idx = 0;
        core.insert_text_in_cell_native(0, 1, ctrl_idx, cell_idx, 0, 0, "원본").unwrap();
        let op = EditOperation::ReplaceCellRuns {
            section: 0,
            table_para: 1,
            row: 0,
            col: 0,
            cell_para: 0,
            runs: vec![RunSpec { text: "변경".to_string(), style: None }],
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(cell_text(&core, 0, 1, 0, 0, 0), "변경");
    }

    #[test]
    fn test_merge_cells_op_apply() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 3, 3).unwrap();
        let cells_before = {
            let para = &core.document.sections[0].paragraphs[1];
            match &para.controls[0] {
                crate::model::control::Control::Table(t) => t.cells.len(),
                _ => panic!("Table 컨트롤 없음"),
            }
        };
        let op = EditOperation::MergeCells {
            section: 0,
            table_para: 1,
            row_start: 0,
            col_start: 0,
            row_end: 0,
            col_end: 1,
        };
        core.apply_edit_op(&op).unwrap();
        let cells_after = {
            let para = &core.document.sections[0].paragraphs[1];
            match &para.controls[0] {
                crate::model::control::Control::Table(t) => t.cells.len(),
                _ => panic!("Table 컨트롤 없음"),
            }
        };
        assert!(cells_after < cells_before, "병합 후 cells 수 감소해야 함 (before={}, after={})", cells_before, cells_after);
    }
}
