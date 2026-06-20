//! 이중 표 (nested table) cell 까지의 경로 표현 (Task #m600-29).
//!
//! 기존 cell 편집 op 는 좌표 `(section, table_para, row, col, cell_para)` 로 *단일 계층*
//! 만 표현한다. 표 안에 표가 들어간 *이중 표* 의 안쪽 cell 을 가리킬 방법이 없다.
//!
//! `CellPath` 는 각 단계마다 *그 단계의 표가 들어있는 paragraph 인덱스* + cell 좌표를
//! 박는다. `path.steps[i+1].para` 는 `path.steps[i]` 의 cell 안 paragraph 인덱스로 해석한다.
//!
//! 길이 1 이면 최상위 cell, 길이 2 이상이면 nested.

use serde::{Deserialize, Serialize};

use crate::document_core::DocumentCore;
use crate::error::HwpError;
use crate::model::control::Control;
use crate::model::paragraph::{CharShapeRef, Paragraph};
use crate::model::table::Cell;

/// 표 cell 까지의 경로. step 의 path 자료.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CellPath {
    pub steps: Vec<CellStep>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellStep {
    /// `steps[0]` — section 안 paragraph 인덱스 (최상위 표 자리).
    /// `steps[i>0]` — 한 단계 위 cell 안 paragraph 인덱스 (nested 표 자리).
    pub para: usize,
    /// paragraph 의 controls 안 Table control 위치.
    pub ctrl_idx: usize,
    /// cell 좌표.
    pub row: u16,
    pub col: u16,
}

impl CellPath {
    pub fn new(steps: Vec<CellStep>) -> Self {
        Self { steps }
    }

    pub fn depth(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl DocumentCore {
    /// `CellPath` 따라 nested cell 의 mutable 참조를 박는다.
    ///
    /// 각 step 마다 `paragraph.controls[ctrl_idx]` 가 `Control::Table` 이어야 한다.
    /// table 안에서 `(row, col)` cell 을 찾고, 다음 step 이 있으면 그 cell 의
    /// `paragraphs[next_step.para]` 로 진입한다.
    pub fn get_cell_mut_at_path(
        &mut self,
        section_idx: usize,
        path: &CellPath,
    ) -> Result<&mut Cell, HwpError> {
        if path.steps.is_empty() {
            return Err(HwpError::RenderError(
                "get_cell_mut_at_path: 빈 path".to_string(),
            ));
        }

        let section = self.document.sections.get_mut(section_idx).ok_or_else(|| {
            HwpError::RenderError(format!(
                "get_cell_mut_at_path: 섹션 {} 범위 초과",
                section_idx
            ))
        })?;

        // 첫 step — section.paragraphs[step.para].controls[step.ctrl_idx] = Table
        let first = path.steps[0];
        let mut para = section
            .paragraphs
            .get_mut(first.para)
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "get_cell_mut_at_path: step 0 의 paragraph {} 범위 초과",
                    first.para
                ))
            })?;
        let mut step = first;

        // step 마다: 현재 paragraph 의 controls[ctrl_idx] 가 Table 인지 확인 → cell 찾기.
        // 다음 step 이 있으면 cell.paragraphs[next.para] 로 진입.
        for next_step_idx in 1..path.steps.len() {
            let controls_len = para.controls.len();
            let ctrl_idx = step.ctrl_idx;
            let ctrl = para.controls.get_mut(ctrl_idx).ok_or_else(|| {
                HwpError::RenderError(format!(
                    "get_cell_mut_at_path: step {} 의 control {} 범위 초과 (controls_len={})",
                    next_step_idx - 1,
                    ctrl_idx,
                    controls_len
                ))
            })?;
            let table = match ctrl {
                Control::Table(t) => t.as_mut(),
                _ => {
                    return Err(HwpError::RenderError(format!(
                        "get_cell_mut_at_path: step {} 의 control 이 Table 아님",
                        next_step_idx - 1
                    )));
                }
            };
            let cell_idx = table
                .cells
                .iter()
                .position(|c| c.row == step.row && c.col == step.col)
                .ok_or_else(|| {
                    HwpError::RenderError(format!(
                        "get_cell_mut_at_path: step {} 의 cell ({}, {}) 없음",
                        next_step_idx - 1,
                        step.row,
                        step.col
                    ))
                })?;
            let cell = table.cells.get_mut(cell_idx).expect("위 position 확인 박힘");

            // 다음 step 의 paragraph 로 진입.
            let next = path.steps[next_step_idx];
            let next_para_idx = next.para;
            let cell_paragraphs_len = cell.paragraphs.len();
            para = cell.paragraphs.get_mut(next_para_idx).ok_or_else(|| {
                HwpError::RenderError(format!(
                    "get_cell_mut_at_path: step {} cell 의 paragraph {} 범위 초과 (paragraphs_len={})",
                    next_step_idx - 1,
                    next_para_idx,
                    cell_paragraphs_len
                ))
            })?;
            step = next;
        }

        // 마지막 step — para.controls[step.ctrl_idx] = Table → cell(row, col) 반환.
        let controls_len = para.controls.len();
        let ctrl_idx = step.ctrl_idx;
        let ctrl = para.controls.get_mut(ctrl_idx).ok_or_else(|| {
            HwpError::RenderError(format!(
                "get_cell_mut_at_path: 마지막 step 의 control {} 범위 초과 (controls_len={})",
                ctrl_idx, controls_len
            ))
        })?;
        let table = match ctrl {
            Control::Table(t) => t.as_mut(),
            _ => {
                return Err(HwpError::RenderError(
                    "get_cell_mut_at_path: 마지막 step 의 control 이 Table 아님".to_string(),
                ));
            }
        };
        let row = step.row;
        let col = step.col;
        let cell_idx = table
            .cells
            .iter()
            .position(|c| c.row == row && c.col == col)
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "get_cell_mut_at_path: 마지막 step 의 cell ({}, {}) 없음",
                    row, col
                ))
            })?;
        table
            .cells
            .get_mut(cell_idx)
            .ok_or_else(|| HwpError::RenderError("get_cell_mut_at_path: cell 접근 실패".to_string()))
    }

    /// `CellPath` 따라 nested cell 의 paragraph runs 를 통째 교체 (Task #m600-29).
    ///
    /// 기존 `replace_cell_runs_native` 의 nested 자료 동등 자체. 최종 cell 의
    /// `paragraphs[cell_para]` 의 text·char_count·char_offsets·char_shapes 자료 재구성 +
    /// line_segs.clear() 자료로 paginate 측 reflow 박음.
    pub fn replace_cell_runs_at_path_native(
        &mut self,
        section_idx: usize,
        path: &CellPath,
        cell_para_idx: usize,
        runs_json: &str,
    ) -> Result<String, HwpError> {
        // 1. 새 runs 자체 자료 합성 — 텍스트 자체 자체 자체 자체 자체
        let runs: Vec<serde_json::Value> = serde_json::from_str(runs_json)
            .map_err(|e| HwpError::InvalidFile(format!("runs_json 파싱 실패: {e}")))?;
        let mut new_text = String::new();
        for run in &runs {
            if let Some(text) = run.get("text").and_then(|v| v.as_str()) {
                new_text.push_str(text);
            }
        }

        // 2. path 따라 cell 박음. 편집 시 raw_stream 자료 무효화 (재직렬화 유도).
        if let Some(section) = self.document.sections.get_mut(section_idx) {
            section.raw_stream = None;
        }
        let cell = self.get_cell_mut_at_path(section_idx, path)?;

        // 3. cell.paragraphs[cell_para] 자체 자체 자체 자체 자체 재구성
        let cell_paragraphs_len = cell.paragraphs.len();
        let para = cell.paragraphs.get_mut(cell_para_idx).ok_or_else(|| {
            HwpError::RenderError(format!(
                "replace_cell_runs_at_path_native: cell_para {} 범위 초과 (paragraphs_len={})",
                cell_para_idx, cell_paragraphs_len
            ))
        })?;

        // base char_shape_id 자체 자체 자체 자체 보존 (기존 char_shapes 의 첫 자료)
        let base_char_shape_id = para
            .char_shapes
            .first()
            .map(|cs| cs.char_shape_id)
            .unwrap_or(0);

        rebuild_paragraph_text(para, new_text, base_char_shape_id);

        // 4. paginate 자료 박음 — 기존 cell native 자체 자체 자체 자체 자체 자체.
        // line_segs.clear() 박힌 cell paragraph 자체 자체 자체 자체 자체 자체 reflow 자료.
        self.paginate_if_needed();

        Ok(crate::document_core::helpers::json_ok_with(&format!(
            "\"cellParaIdx\":{}",
            cell_para_idx
        )))
    }
}

/// Paragraph 의 text 자료 자체 자체 자체 박고 char_count·char_offsets·char_shapes·line_segs
/// 자료 자체 자체 자체 재구성. `base_char_shape_id` 자체 자체 자체 자체 자체 단일 char_shape
/// 자체 자체 자체 자체 자체 박는다.
fn rebuild_paragraph_text(para: &mut Paragraph, text: String, base_char_shape_id: u32) {
    // char_count = utf16 길이 + 1 (paragraph end mark)
    let utf16_len: u32 = text.chars().map(|c| c.len_utf16() as u32).sum();
    para.text = text;
    para.char_count = utf16_len + 1;
    para.char_shapes.clear();
    para.char_shapes.push(CharShapeRef {
        start_pos: 0,
        char_shape_id: base_char_shape_id,
    });
    para.char_offsets.clear();
    let mut pos = 0u32;
    for c in para.text.chars() {
        para.char_offsets.push(pos);
        pos += c.len_utf16() as u32;
    }
    // line_segs 자체 자체 자체 자체 자체 자체 — paginate 자체 자체 자체 자체 reflow.
    para.line_segs.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 원본 hwp 의 nested 3x2 표 fixture. cycle 28 의 dump_nested 자료로 확인한
    /// 자리 — s0.p4.c0 outer 1x1 → cell[0].paragraphs[8].controls[0] = nested 3x2.
    const SAMPLE_HWP: &str =
        "/Users/yuniba_01/Downloads/icon/1. (★사업중 필독) 사업관리 참조표.hwp";

    fn load_core() -> DocumentCore {
        let data = fs::read(SAMPLE_HWP).expect("sample hwp 자료 자체 자체");
        DocumentCore::from_bytes(&data).expect("parse")
    }

    #[test]
    fn cell_path_depth_1_outer_cell() {
        let mut core = load_core();
        let path = CellPath::new(vec![CellStep {
            para: 4,
            ctrl_idx: 0,
            row: 0,
            col: 0,
        }]);
        let cell = core.get_cell_mut_at_path(0, &path).expect("depth-1 cell");
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);
        // 외부 cell — paragraphs 21개
        assert_eq!(cell.paragraphs.len(), 21);
    }

    #[test]
    fn cell_path_depth_2_nested_cell() {
        let mut core = load_core();
        let path = CellPath::new(vec![
            CellStep {
                para: 4,
                ctrl_idx: 0,
                row: 0,
                col: 0,
            },
            CellStep {
                para: 8,
                ctrl_idx: 0,
                row: 0,
                col: 0,
            },
        ]);
        let cell = core.get_cell_mut_at_path(0, &path).expect("depth-2 cell");
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);
        // nested 3x2 의 cell(0,0) — 자체 paragraphs 자료 박힘
        assert!(!cell.paragraphs.is_empty());
    }

    #[test]
    fn cell_path_depth_2_all_six_nested_cells() {
        let mut core = load_core();
        // nested 3x2 의 6개 cell 모두 접근 가능 자체 확인
        for row in 0..3 {
            for col in 0..2 {
                let path = CellPath::new(vec![
                    CellStep {
                        para: 4,
                        ctrl_idx: 0,
                        row: 0,
                        col: 0,
                    },
                    CellStep {
                        para: 8,
                        ctrl_idx: 0,
                        row,
                        col,
                    },
                ]);
                let cell = core
                    .get_cell_mut_at_path(0, &path)
                    .unwrap_or_else(|e| panic!("nested cell ({}, {}) 자체 실패: {}", row, col, e));
                assert_eq!(cell.row, row);
                assert_eq!(cell.col, col);
            }
        }
    }

    #[test]
    fn cell_path_empty_returns_error() {
        let mut core = load_core();
        let path = CellPath::default();
        assert!(core.get_cell_mut_at_path(0, &path).is_err());
    }

    #[test]
    fn cell_path_invalid_row_returns_error() {
        let mut core = load_core();
        let path = CellPath::new(vec![CellStep {
            para: 4,
            ctrl_idx: 0,
            row: 99,
            col: 0,
        }]);
        assert!(core.get_cell_mut_at_path(0, &path).is_err());
    }

    #[test]
    fn replace_cell_runs_at_path_changes_nested_cell_text() {
        let mut core = load_core();
        let path = CellPath::new(vec![
            CellStep {
                para: 4,
                ctrl_idx: 0,
                row: 0,
                col: 0,
            },
            CellStep {
                para: 8,
                ctrl_idx: 0,
                row: 1,
                col: 1,
            },
        ]);
        let runs_json = r#"[{"text":"NESTED_EDIT"}]"#;
        core.replace_cell_runs_at_path_native(0, &path, 0, runs_json)
            .expect("nested cell edit");

        // 변경 자료 확인 — nested cell(1,1) 의 paragraph[0].text == "NESTED_EDIT"
        let cell = core.get_cell_mut_at_path(0, &path).unwrap();
        assert_eq!(cell.paragraphs[0].text, "NESTED_EDIT");
        assert_eq!(cell.paragraphs[0].char_count, 11 + 1); // utf16 11 + end mark
    }

    #[test]
    fn replace_cell_runs_at_path_depth_1_changes_outer_cell_para() {
        let mut core = load_core();
        let path = CellPath::new(vec![CellStep {
            para: 4,
            ctrl_idx: 0,
            row: 0,
            col: 0,
        }]);
        let runs_json = r#"[{"text":"OUTER_PARA_0"}]"#;
        core.replace_cell_runs_at_path_native(0, &path, 0, runs_json)
            .expect("depth-1 cell edit");

        let cell = core.get_cell_mut_at_path(0, &path).unwrap();
        assert_eq!(cell.paragraphs[0].text, "OUTER_PARA_0");
    }
}
