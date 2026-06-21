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

use serde::{Deserialize, Deserializer, Serialize};

use crate::document_core::DocumentCore;
use crate::error::HwpError;

// font-size 자리: 외부 인터페이스 = pt 실수, 내부 저장 = 1/100 pt 정수 (u16).
// 호출자가 `15.5` 를 보내면 1550 으로 저장. ir-slice 응답도 pt 실수로 노출 (ir_compact.rs)
// 라 *요청·응답 단위가 모두 pt 실수* 로 통일된다.
//
// 입력 범위는 *0 ~ 655.35 pt* — u16 으로 표현 가능한 한컴 spec 상한. 그 밖의 값은
// 호출자의 단위 혼동 (예: raw 0.01pt 정수 1400 을 *1400 pt* 로 잘못 보냄) 일 가능성이
// 높아 *silent saturate 대신 명시적 에러* 로 거부한다. 종전 silent clamp 가 char_shape
// base_size 에 65535 sentinel 을 박아 paragraph line_height 가 페이지 본문 영역의 *47배*
// 까지 부풀어 paginate 결과를 비정상으로 만든 사고가 있었다 — 단위 혼동을 silent 로
// 묻어두지 않는 게 안전.
fn deserialize_font_size_pt<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let opt = Option::<f64>::deserialize(deserializer)?;
    opt.map(|pt| {
        if !pt.is_finite() || pt < 0.0 || pt > 655.35 {
            return Err(D::Error::custom(format!(
                "fontSize 가 허용 범위 (0 ~ 655.35 pt) 를 벗어남: {pt}. \
                 *pt 단위 실수* 로 보내야 함 (예: 14.0 = 14pt). \
                 raw 0.01pt 정수 (예: 1400) 가 아닙니다."
            )));
        }
        Ok((pt * 100.0).round() as u16)
    })
    .transpose()
}

// ─── Sub-2: Partial 타입 (옵셔널 필드만 직렬화) ─────────────────

/// 본문 문단의 부분 스타일. None 인 필드는 *현재 값 유지* 의미.
/// JSON 직렬화 시 None 은 제외 (`skip_serializing_if`).
/// `apply_para_format_native(props_json)` 의 입력으로 *변환 후* 사용된다 —
/// 직접 serialize 한 결과는 SKILL.md 광고 키이며 native 키와 일부 다름
/// (`align` → `alignment`, `line_height` → `lineSpacing`). 변환은
/// [`partial_paragraph_style_to_native_json`] 가 수행.
///
/// `deny_unknown_fields` — 광고되지 않은/오타 키는 400 반환 (silent drop 사고 예방).
///
/// 키 변형 정책: *snake_case · camelCase · kebab-case · 기존 별칭* 모두 alias 로 허용.
/// 광고 문서 (SKILL.md) 는 kebab-case (`align`, `line-height`), 기존 e2e/클라는
/// snake_case (`alignment`, `line_spacing`), camelCase 는 serde rename_all 기본값.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialParagraphStyle {
    // [Sub-7] rename: alignment → align + alias 호환 (kebab/snake/camel/기존)
    #[serde(
        default,
        alias = "alignment",
        skip_serializing_if = "Option::is_none"
    )]
    pub align: Option<String>,   // "left"|"right"|"center"|"justify"|"distribute"
    // [Sub-7] rename: line_spacing → line_height + alias 호환
    #[serde(
        default,
        alias = "lineSpacing",
        alias = "line_spacing",
        alias = "line_height",
        alias = "line-height",
        skip_serializing_if = "Option::is_none"
    )]
    pub line_height: Option<f64>,
    #[serde(default, alias = "margin_left", alias = "margin-left", skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<i16>,
    #[serde(default, alias = "margin_right", alias = "margin-right", skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<i16>,
    #[serde(default, alias = "spacing_before", alias = "spacing-before", skip_serializing_if = "Option::is_none")]
    pub spacing_before: Option<i16>,
    #[serde(default, alias = "spacing_after", alias = "spacing-after", skip_serializing_if = "Option::is_none")]
    pub spacing_after: Option<i16>,
}

/// 셀의 부분 스타일. None 인 필드는 *현재 값 유지*.
/// `set_cell_properties_native(json)` 의 입력으로 *변환 후* 사용된다 —
/// 변환은 [`partial_cell_style_to_native_json`] 가 수행 (bgcolor → fillType+fillColor,
/// border.all → 4 방향 펼침, vertical_align 문자열 → u8 등).
///
/// `deny_unknown_fields` — 광고되지 않은/오타 키는 400 반환.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialCellStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(
        default,
        alias = "vertical_align",
        alias = "vertical-align",
        skip_serializing_if = "Option::is_none"
    )]
    pub vertical_align: Option<String>,   // "top"|"middle"|"center"|"bottom"
    #[serde(default, alias = "border_fill_id", skip_serializing_if = "Option::is_none")]
    pub border_fill_id: Option<u16>,
    #[serde(default, alias = "is_header", skip_serializing_if = "Option::is_none")]
    pub is_header: Option<bool>,
    #[serde(default, alias = "cell_protect", skip_serializing_if = "Option::is_none")]
    pub cell_protect: Option<bool>,

    // ─── [Sub-7] 신규 ──────────────────────────────────────────────────────
    /// 셀 배경 색 — CSS hex "#RRGGBB". native 직렬화 시 `fillType=solid` + `fillColor=hex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgcolor: Option<String>,
    /// 4 방향 테두리. `all` 우선 적용 후 left/right/top/bottom 으로 개별 override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderSpec>,
    #[serde(default, alias = "padding_left", alias = "padding-left", skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<i16>,
    #[serde(default, alias = "padding_right", alias = "padding-right", skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<i16>,
    #[serde(default, alias = "padding_top", alias = "padding-top", skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<i16>,
    #[serde(default, alias = "padding_bottom", alias = "padding-bottom", skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<i16>,
}

/// 셀 테두리 4 방향 묶음. `all` 지정 시 4 방향 일괄, 그 외 키는 해당 방향만 override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BorderSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<BorderLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<BorderLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<BorderLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<BorderLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<BorderLine>,
}

/// 한 방향 테두리 한 줄 사양.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BorderLine {
    /// CSS hex "#RRGGBB".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 선 두께. HWP 단위 또는 mm × 100 (native fn 약속에 의존).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// 선 종류 — 1=solid, 2=dotted, ... (native 약속).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub line_type: Option<u8>,
}

/// run 의 부분 char 스타일. None 인 필드 유지.
/// `apply_char_format_native(props_json)` 입력으로 *변환 후* 사용된다 —
/// 변환은 [`partial_run_style_to_native_json`] 가 수행 (font_size → fontSize,
/// color → textColor (hex 문자열 그대로), highlight → shadeColor,
/// font_name → fontId (DocumentCore lookup 필요 — apply 분기에서 별도 주입)).
///
/// `deny_unknown_fields` — 광고되지 않은/오타 키는 400 반환.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialRunStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,

    // [Sub-7] rename: base_size → font_size + alias 호환 (camelCase 변환과 snake_case 모두).
    // 외부 인터페이스는 *pt 단위 실수* — `15.5` 가 그대로 15.5 pt 로 해석된다.
    // deserialize_font_size_pt 가 1/100 pt 정수 (u16) 로 변환해 저장.
    #[serde(
        default,
        alias = "baseSize",
        alias = "base_size",
        alias = "fontSize",
        alias = "font_size",
        alias = "font-size",
        deserialize_with = "deserialize_font_size_pt",
        skip_serializing_if = "Option::is_none"
    )]
    pub font_size: Option<u16>,

    // [Sub-7] rename: text_color (u32) → color (CSS hex 문자열) + alias 호환
    //   기존 u32 호출처는 e2e 외부에 *없음* (server/rhwp-studio grep 0건) — 안전한 타입 교체.
    #[serde(
        default,
        alias = "textColor",
        alias = "text_color",
        skip_serializing_if = "Option::is_none"
    )]
    pub color: Option<String>,

    /// 형광펜 색. CSS hex "#RRGGBB". native 의 `shadeColor` 키로 직렬화.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,

    /// 폰트 이름. apply 분기에서 `DocumentCore::find_or_create_font_id_native` 로
    /// fontId 변환 후 native JSON 의 `fontId` 키로 보낸다 — 직렬화 함수 단계에선
    /// `fontName` 키를 임시 보관, apply 가 후처리.
    #[serde(
        default,
        alias = "font_name",
        alias = "font-name",
        skip_serializing_if = "Option::is_none"
    )]
    pub font_name: Option<String>,
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
    /// `cell_idx` 가 채워져 있으면 변환 생략하고 그대로 사용 — 서버가 broadcast 전에 미리
    /// 채워서 다중 사용자 race (셀 추가/삭제) 시 클라 재계산과 결과 어긋남 방지.
    SetCellStyle {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        /// paragraph 안 Table control 인덱스. 서버 workbench 가 broadcast 전에
        /// `find_table_ctrl_idx` 결과로 채운다. 미지정 시 native 적용 단계에서 자동 탐색.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
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
        /// paragraph 안 Table control 인덱스. 미지정 시 자동 탐색.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
    },
    /// 셀 내 문단 runs 통째 교체. replace_cell_runs_native 위임.
    /// `cell_idx` 가 채워져 있으면 변환 생략하고 그대로 사용.
    ReplaceCellRuns {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
        cell_para: usize,
        runs: Vec<RunSpec>,
    },
    /// 셀 내 텍스트 삽입 (옵셔널 style). insert_text_in_cell_native + 옵셔널 apply_char_format_in_cell_native.
    /// `cell_idx` 가 채워져 있으면 변환 생략하고 그대로 사용.
    InsertTextInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
        cell_para: usize,
        offset: usize,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialRunStyle>,
    },
    /// 셀 내 범위 텍스트 삭제 (동·다문단). delete_range_native(cell_ctx=Some(...)) 위임.
    /// `cell_idx` 가 채워져 있으면 변환 생략하고 그대로 사용.
    DeleteRangeInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
        cell_para_start: usize,
        char_start: usize,
        cell_para_end: usize,
        char_end: usize,
    },

    // ─── Sub-8: 강제 쪽 나누기 (Ctrl+Enter 동등) ───────────────────────────────

    /// 강제 쪽 나누기. `insert_page_break_native` 위임.
    /// 동작: `(section, para)` 의 `offset` 자리에서 문단 분할 + 새 문단 (`para+1`) 에
    /// `ColumnBreakType::Page` 설정 + `recompose_section` + `paginate_if_needed`.
    /// 분할 결과 새 페이지가 시작되는 문단이 `para+1` 자리에 들어간다.
    InsertPageBreak {
        section: usize,
        para: usize,
        offset: usize,
    },

    // ─── Task #m600-29: nested table (이중 표) cell 편집 ─────────────────────

    /// nested cell paragraph 의 runs 통째 교체. `CellPath` 따라 임의 깊이 표현.
    /// 기존 `ReplaceCellRuns` 는 최상위 cell 만 가리키지만 이 variant 는 path 길이로
    /// nested depth 표현 (길이 1 = 최상위, 길이 2 = 한 단계 nested, ...).
    /// `replace_cell_runs_at_path_native` 위임.
    ReplaceCellRunsAtPath {
        section: usize,
        path: super::cell_path::CellPath,
        /// 최종 cell 안 paragraph 인덱스 (편집 자리).
        cell_para: usize,
        runs: Vec<RunSpec>,
    },
    /// 한컴 Enter / Ctrl+Enter 와 동등. 커서 위치 (char_offset) 에서 Enter.
    ///
    /// payload 키 분기 (단일 variant 안에서 본문/셀 모드):
    /// - `table_para` 부재 → 본문 모드. 필수: section, para
    /// - `table_para` 박힘 → 셀 모드. 필수: section, table_para, row, col, cell_para
    ///
    /// char_offset (i64):
    /// - `-1` 또는 음수 → 본문/셀 paragraph 끝 자리 (default)
    /// - `0` → 시작 자리
    /// - `len` 이상 → clamp to len (= 끝 자리. silent fail 0건)
    /// - 중간값 → split
    ///
    /// count: 같은 자리 Enter N 회 (N 개 빈 paragraph 누적). default 1.
    /// page_break: true → 첫 번째 새 paragraph 만 페이지 분리. 셀 모드 + true 면 RenderError("INVALID_PAYLOAD: ...").
    PressEnter {
        section: usize,
        // 본문 모드 키 (table_para 부재 시 필수)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        para: Option<usize>,
        // 셀 모드 키 (table_para 박힘 시 필수)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        table_para: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        col: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_para: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        // 공통 키
        #[serde(default = "default_char_offset")]
        char_offset: i64,
        #[serde(default = "one_count")]
        count: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialParagraphStyle>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_break: Option<bool>,
    },
}

fn one_count() -> usize { 1 }

fn default_char_offset() -> i64 { -1 }

// ─── Sub-7: Partial*Style → native fn JSON 변환 ─────────────────────────────
//
// SKILL.md 광고 키 (camelCase 친화적) 와 native fn 이 받는 키 (parse_* helper 들이
// 인식하는 키 — `parse_cell_props_native` / `parse_char_shape_mods` /
// `parse_para_shape_mods` 참조) 가 일치하지 않는 경우의 정합 layer.
//
// 직접 `serde_json::to_string(style)` 하면 `align` / `font_size` / `color` /
// `bgcolor` / `border.all` 같은 *광고 키* 가 native 키 (각각 `alignment` /
// `fontSize` / `textColor` / `fillType+fillColor` / `borderLeft/Right/Top/Bottom`)
// 와 안 맞아 *silent drop* 사고가 발생.

/// PartialCellStyle → `set_cell_properties_native` 입력 JSON.
///
/// 변환 규칙:
/// - `bgcolor: "#RRGGBB"` → `fillType: "solid"` + `fillColor: "#RRGGBB"`
/// - `border.all` → 4 방향(`borderLeft`/`Right`/`Top`/`Bottom`) 일괄 적용, 그 외 개별 키는 override
/// - `vertical_align: "top"|"middle"|"center"|"bottom"` → u8 (0/1/1/2)
/// - 나머지: camelCase 그대로
pub(crate) fn partial_cell_style_to_native_json(style: &PartialCellStyle) -> String {
    use serde_json::{Map, Value};
    let mut obj: Map<String, Value> = Map::new();

    if let Some(v) = style.width {
        obj.insert("width".into(), Value::from(v));
    }
    if let Some(v) = style.height {
        obj.insert("height".into(), Value::from(v));
    }
    if let Some(ref s) = style.vertical_align {
        let u: u8 = match s.as_str() {
            "middle" | "center" => 1,
            "bottom" => 2,
            _ => 0,   // "top" 기본
        };
        obj.insert("verticalAlign".into(), Value::from(u));
    }
    if let Some(v) = style.border_fill_id {
        obj.insert("borderFillId".into(), Value::from(v));
    }
    if let Some(v) = style.is_header {
        obj.insert("isHeader".into(), Value::from(v));
    }
    if let Some(v) = style.cell_protect {
        obj.insert("cellProtect".into(), Value::from(v));
    }
    if let Some(ref bg) = style.bgcolor {
        obj.insert("fillType".into(), Value::from("solid"));
        obj.insert("fillColor".into(), Value::from(bg.clone()));
    }
    if let Some(ref border) = style.border {
        // all 먼저 4 방향에 일괄 → 개별 override
        let base = border.all.as_ref();
        for (key, side) in [
            ("borderLeft", border.left.as_ref().or(base)),
            ("borderRight", border.right.as_ref().or(base)),
            ("borderTop", border.top.as_ref().or(base)),
            ("borderBottom", border.bottom.as_ref().or(base)),
        ] {
            if let Some(b) = side {
                obj.insert(key.into(), border_line_to_json(b));
            }
        }
    }
    if let Some(v) = style.padding_left {
        obj.insert("paddingLeft".into(), Value::from(v));
    }
    if let Some(v) = style.padding_right {
        obj.insert("paddingRight".into(), Value::from(v));
    }
    if let Some(v) = style.padding_top {
        obj.insert("paddingTop".into(), Value::from(v));
    }
    if let Some(v) = style.padding_bottom {
        obj.insert("paddingBottom".into(), Value::from(v));
    }

    Value::Object(obj).to_string()
}

fn border_line_to_json(b: &BorderLine) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut m: Map<String, Value> = Map::new();
    if let Some(ref c) = b.color {
        m.insert("color".into(), Value::from(c.clone()));
    }
    if let Some(w) = b.width {
        m.insert("width".into(), Value::from(w));
    }
    if let Some(t) = b.line_type {
        m.insert("type".into(), Value::from(t));
    }
    Value::Object(m)
}

/// PartialRunStyle → `apply_char_format_native` 입력 JSON.
///
/// 변환 규칙:
/// - `font_size` → `fontSize`
/// - `color: "#RRGGBB"` → `textColor: "#RRGGBB"` (helpers.rs::json_color 가 CSS hex → BGR 처리)
/// - `highlight: "#RRGGBB"` → `shadeColor: "#RRGGBB"`
/// - `font_name` → 변환 단계에서는 보관만 (native 는 `fontId` u16 요구) — apply 분기에서
///   `find_or_create_font_id_native` 호출 후 `fontId` 키로 추가 주입.
///   본 함수는 `fontName` 도 함께 출력 (native 가 인식하진 않지만 hint 용도),
///   apply 후처리에서 fontId 가 함께 들어가 native 가 사용.
pub(crate) fn partial_run_style_to_native_json(style: &PartialRunStyle) -> String {
    use serde_json::{Map, Value};
    let mut obj: Map<String, Value> = Map::new();
    if let Some(v) = style.bold {
        obj.insert("bold".into(), Value::from(v));
    }
    if let Some(v) = style.italic {
        obj.insert("italic".into(), Value::from(v));
    }
    if let Some(v) = style.underline {
        obj.insert("underline".into(), Value::from(v));
    }
    if let Some(v) = style.strikethrough {
        obj.insert("strikethrough".into(), Value::from(v));
    }
    if let Some(v) = style.font_size {
        obj.insert("fontSize".into(), Value::from(v));
    }
    if let Some(ref c) = style.color {
        obj.insert("textColor".into(), Value::from(c.clone()));
    }
    if let Some(ref h) = style.highlight {
        obj.insert("shadeColor".into(), Value::from(h.clone()));
    }
    // font_name 은 apply 단계에서 fontId 로 변환되어 별도 주입 — 여기선 출력 안 함.
    Value::Object(obj).to_string()
}

/// `font_name` 이 있으면 DocumentCore 의 폰트 테이블에서 ID 를 조회/등록 후
/// native JSON 에 `fontId` 키를 주입. 다른 키는 그대로 유지.
///
/// 입력 `native_json` 은 [`partial_run_style_to_native_json`] 출력 (object).
pub(crate) fn inject_font_id_into_run_style_json(
    core: &mut DocumentCore,
    native_json: &str,
    font_name: Option<&str>,
) -> Result<String, HwpError> {
    let Some(name) = font_name else {
        return Ok(native_json.to_string());
    };
    if name.is_empty() {
        return Ok(native_json.to_string());
    }
    let font_id = core.find_or_create_font_id_native(name);
    if font_id < 0 {
        return Err(HwpError::RenderError(format!(
            "font_name 변환 실패: {name}"
        )));
    }
    let mut value: serde_json::Value = serde_json::from_str(native_json)
        .map_err(|e| HwpError::RenderError(format!("native_json 재파싱: {e}")))?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "fontId".into(),
            serde_json::Value::from(font_id as u16),
        );
    }
    Ok(value.to_string())
}

/// PartialParagraphStyle → `apply_para_format_native` 입력 JSON.
///
/// 변환 규칙:
/// - `align` → `alignment`
/// - `line_height` → `lineSpacing`
/// - 나머지 camelCase 그대로 (`marginLeft`/`marginRight`/`indent`/`spacingBefore`/`spacingAfter`)
pub(crate) fn partial_paragraph_style_to_native_json(style: &PartialParagraphStyle) -> String {
    use serde_json::{Map, Value};
    let mut obj: Map<String, Value> = Map::new();
    if let Some(ref a) = style.align {
        obj.insert("alignment".into(), Value::from(a.clone()));
    }
    if let Some(v) = style.line_height {
        // parse_para_shape_mods 는 lineSpacing 을 i32 로 읽음. f64 → i32 변환.
        obj.insert(
            "lineSpacing".into(),
            Value::from(v.round() as i32),
        );
    }
    if let Some(v) = style.margin_left {
        obj.insert("marginLeft".into(), Value::from(v));
    }
    if let Some(v) = style.margin_right {
        obj.insert("marginRight".into(), Value::from(v));
    }
    if let Some(v) = style.indent {
        obj.insert("indent".into(), Value::from(v));
    }
    if let Some(v) = style.spacing_before {
        obj.insert("spacingBefore".into(), Value::from(v));
    }
    if let Some(v) = style.spacing_after {
        obj.insert("spacingAfter".into(), Value::from(v));
    }
    Value::Object(obj).to_string()
}

/// RunSpec 배열을 *native JSON 으로 변환* 한 결과를 만든다 —
/// 각 run 의 `style` 필드는 [`partial_run_style_to_native_json`] 로 native 키로 매핑.
///
/// `font_name` 키가 들어 있으면 DocumentCore 에 lookup/등록 후 `fontId` 도 함께 주입.
pub(crate) fn runs_to_native_json(
    core: &mut DocumentCore,
    runs: &[RunSpec],
) -> Result<String, HwpError> {
    let mut arr = Vec::with_capacity(runs.len());
    for run in runs {
        let mut obj = serde_json::Map::new();
        obj.insert("text".into(), serde_json::Value::from(run.text.clone()));
        if let Some(ref style) = run.style {
            let style_json = partial_run_style_to_native_json(style);
            let style_with_font = inject_font_id_into_run_style_json(
                core,
                &style_json,
                style.font_name.as_deref(),
            )?;
            let style_value: serde_json::Value = serde_json::from_str(&style_with_font)
                .map_err(|e| HwpError::RenderError(format!("style 재파싱: {e}")))?;
            obj.insert("style".into(), style_value);
        }
        arr.push(serde_json::Value::Object(obj));
    }
    Ok(serde_json::Value::Array(arr).to_string())
}

// ─── Sub-4: 영향 범위 헬퍼 (patch diff 캡처용) ────────────────────────────────

/// 편집 연산이 영향을 미치는 문단 범위와 셀 좌표.
///
/// `before` 는 *적용 전* 캡처해야 할 문단 범위, `after` 는 *적용 후* 캡처할 범위다.
/// insert 계열은 after 가 늘어나고, delete 계열은 줄어든다.
/// `cell` 이 채워져 있으면 표 셀 단위 편집임을 표시하며 IR 슬라이스 내부에서
/// 해당 셀만 강조해 보여 줄 수 있다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedRange {
    pub section: usize,
    pub before: ParaRange,
    pub after: ParaRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellFocus>,
}

/// 0-based 문단 인덱스의 반열린 범위 `[start, end)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParaRange {
    pub start: usize,
    pub end: usize,
}

impl ParaRange {
    pub fn single(para: usize) -> Self { Self { start: para, end: para + 1 } }
    pub fn empty(at: usize) -> Self { Self { start: at, end: at } }
}

/// 표 셀 편집의 좌표.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellFocus {
    pub table_para: usize,
    pub row: usize,
    pub col: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_idx: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_para: Option<usize>,
}

impl EditOperation {
    /// 편집 연산이 영향을 미치는 문단 범위 / 셀 좌표.
    ///
    /// 적용 전 (`before`) 과 적용 후 (`after`) 범위가 다를 수 있다.
    /// IR 슬라이스 캡처 (patch diff) 에서 사용한다.
    pub fn affected_range(&self) -> AffectedRange {
        match self {
            EditOperation::InsertText { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange::single(*para),
                cell: None,
            },
            EditOperation::DeleteText { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange::single(*para),
                cell: None,
            },
            EditOperation::SplitParagraph { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange { start: *para, end: *para + 2 },
                cell: None,
            },
            EditOperation::MergeParagraph { section, para, .. } => {
                let prev = para.saturating_sub(1);
                AffectedRange {
                    section: *section,
                    before: ParaRange { start: prev, end: *para + 1 },
                    after: ParaRange::single(prev),
                    cell: None,
                }
            }
            EditOperation::ReplaceRuns { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange::single(*para),
                cell: None,
            },
            EditOperation::SetParagraphStyle { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange::single(*para),
                cell: None,
            },
            EditOperation::DeleteRange { section, para_start, para_end, .. } => {
                let end_exclusive = para_end.saturating_add(1);
                AffectedRange {
                    section: *section,
                    before: ParaRange { start: *para_start, end: end_exclusive },
                    after: ParaRange::single(*para_start),
                    cell: None,
                }
            }
            EditOperation::InsertParagraph { section, after_para, count, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*after_para),
                after: ParaRange { start: *after_para, end: *after_para + 1 + *count },
                cell: None,
            },
            EditOperation::DeleteElement { section, para, element_type } => {
                match element_type {
                    ElementType::Paragraph => AffectedRange {
                        section: *section,
                        before: ParaRange::single(*para),
                        after: ParaRange::empty(*para),
                        cell: None,
                    },
                    ElementType::Table => AffectedRange {
                        section: *section,
                        before: ParaRange::single(*para),
                        after: ParaRange::single(*para),
                        cell: None,
                    },
                }
            }
            EditOperation::InsertTable { section, insert_after_para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*insert_after_para),
                after: ParaRange { start: *insert_after_para, end: *insert_after_para + 2 },
                cell: None,
            },
            EditOperation::SetCellStyle { section, table_para, row, col, cell_idx, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*table_para),
                after: ParaRange::single(*table_para),
                cell: Some(CellFocus {
                    table_para: *table_para,
                    row: *row,
                    col: *col,
                    cell_idx: *cell_idx,
                    cell_para: None,
                }),
            },
            EditOperation::MergeCells { section, table_para, row_start, col_start, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*table_para),
                after: ParaRange::single(*table_para),
                cell: Some(CellFocus {
                    table_para: *table_para,
                    row: *row_start,
                    col: *col_start,
                    cell_idx: None,
                    cell_para: None,
                }),
            },
            EditOperation::ReplaceCellRuns { section, table_para, row, col, cell_idx, cell_para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*table_para),
                after: ParaRange::single(*table_para),
                cell: Some(CellFocus {
                    table_para: *table_para,
                    row: *row,
                    col: *col,
                    cell_idx: *cell_idx,
                    cell_para: Some(*cell_para),
                }),
            },
            EditOperation::InsertTextInCell { section, table_para, row, col, cell_idx, cell_para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*table_para),
                after: ParaRange::single(*table_para),
                cell: Some(CellFocus {
                    table_para: *table_para,
                    row: *row,
                    col: *col,
                    cell_idx: *cell_idx,
                    cell_para: Some(*cell_para),
                }),
            },
            EditOperation::DeleteRangeInCell { section, table_para, row, col, cell_idx, cell_para_start, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*table_para),
                after: ParaRange::single(*table_para),
                cell: Some(CellFocus {
                    table_para: *table_para,
                    row: *row,
                    col: *col,
                    cell_idx: *cell_idx,
                    cell_para: Some(*cell_para_start),
                }),
            },
            EditOperation::InsertPageBreak { section, para, .. } => AffectedRange {
                section: *section,
                before: ParaRange::single(*para),
                after: ParaRange { start: *para, end: *para + 2 },
                cell: None,
            },
            // Task #m600-29 — path 의 첫 step 의 para 가 최상위 표가 들어있는 paragraph.
            // CellFocus 자료는 path 깊이 1 일 때만 의미가 있어 None 박음 — broadcast 클라이언트가
            // path 자체 자체 자체 자체 자체 nested cell 위치 박음.
            EditOperation::ReplaceCellRunsAtPath { section, path, .. } => {
                let outer_para = path.steps.first().map(|s| s.para).unwrap_or(0);
                AffectedRange {
                    section: *section,
                    before: ParaRange::single(outer_para),
                    after: ParaRange::single(outer_para),
                    cell: None,
                }
            }
            EditOperation::PressEnter { section, para, table_para, row, col, cell_para, cell_idx, count, .. } => {
                if let Some(tp) = table_para {
                    // 셀 모드 — cell focus
                    AffectedRange {
                        section: *section,
                        before: ParaRange::single(*tp),
                        after: ParaRange::single(*tp),
                        cell: Some(CellFocus {
                            table_para: *tp,
                            row: row.unwrap_or(0),
                            col: col.unwrap_or(0),
                            cell_idx: *cell_idx,
                            cell_para: (*cell_para).map(|cp| cp + *count),
                        }),
                    }
                } else {
                    // 본문 모드 — para .. para + count + 1 (새 paragraph 들 자리)
                    let p = para.unwrap_or(0);
                    AffectedRange {
                        section: *section,
                        before: ParaRange::single(p),
                        after: ParaRange { start: p, end: p + 1 + *count },
                        cell: None,
                    }
                }
            }
        }
    }
}

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
        // control_idx 자리 우선 시도. 실패하면 paragraph 안의 *첫 Table control* 자동 검색.
        // 호출자가 control_idx=0 하드코딩 자리에서 paragraph 가 SectionDef/ColumnDef 같은 다른
        // control 을 먼저 가질 때 (섹션의 첫 문단 자리) 자동 우회. m400 sub-1.
        let table = para
            .controls
            .get(control_idx)
            .and_then(|c| match c {
                crate::model::control::Control::Table(t) => Some(t.as_ref()),
                _ => None,
            })
            .or_else(|| {
                para.controls.iter().find_map(|c| match c {
                    crate::model::control::Control::Table(t) => Some(t.as_ref()),
                    _ => None,
                })
            })
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "find_cell_idx: table_para={} 에 Table control 없음 (controls_len={})",
                    table_para_idx,
                    para.controls.len()
                ))
            })?;
        table
            .cells
            .iter()
            .position(|c| c.row == row && c.col == col)
            .ok_or_else(|| {
                HwpError::RenderError(format!("find_cell_idx: ({}, {}) 셀 없음", row, col))
            })
    }

    /// paragraph 안 *첫 Table control* 의 인덱스를 찾는다. 섹션의 첫 문단처럼
    /// `SectionDef`/`ColumnDef` 같은 다른 control 이 Table 앞에 동거하는 자리에서도
    /// 셀 단위 native 호출의 `ctrl_idx` 가 정합 동작하도록 자동 보정한다.
    /// `find_cell_idx` 의 fallback 과 동일 규약 (가장 앞 Table) 을 따라 한 paragraph
    /// 안에서 두 함수가 같은 Table 을 가리킨다.
    pub fn find_table_ctrl_idx(
        &self,
        section_idx: usize,
        table_para_idx: usize,
    ) -> Result<usize, HwpError> {
        let para = self
            .document
            .sections
            .get(section_idx)
            .and_then(|s| s.paragraphs.get(table_para_idx))
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "find_table_ctrl_idx: 좌표 부적합 (sec={}, table_para={})",
                    section_idx, table_para_idx
                ))
            })?;
        para.controls
            .iter()
            .position(|c| matches!(c, crate::model::control::Control::Table(_)))
            .ok_or_else(|| {
                HwpError::RenderError(format!(
                    "find_table_ctrl_idx: table_para={} 에 Table control 없음 (controls_len={})",
                    table_para_idx,
                    para.controls.len()
                ))
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
                // [Sub-7] PartialRunStyle → native JSON 변환 (font_size → fontSize, color → textColor,
                // highlight → shadeColor, font_name → fontId lookup).
                let runs_json = runs_to_native_json(self, runs)?;
                self.replace_runs_native(*section, *para, &runs_json)?;
            }
            EditOperation::SetParagraphStyle { section, para, style } => {
                // [Sub-7] PartialParagraphStyle → native JSON (align → alignment, line_height → lineSpacing).
                let props_json = partial_paragraph_style_to_native_json(style);
                self.apply_para_format_native(*section, *para, &props_json)?;
            }
            EditOperation::DeleteRange { section, para_start, char_start, para_end, char_end } => {
                self.delete_range_native(*section, *para_start, *char_start, *para_end, *char_end, None)?;
            }
            EditOperation::InsertParagraph { section, after_para, count, style } => {
                for i in 0..*count {
                    self.insert_paragraph_native(*section, *after_para + i)?;
                    if let Some(s) = style {
                        // [Sub-7] PartialParagraphStyle → native JSON 변환.
                        let props_json = partial_paragraph_style_to_native_json(s);
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
            EditOperation::SetCellStyle { section, table_para, row, col, cell_idx, ctrl_idx, style } => {
                let ctrl_idx = match ctrl_idx {
                    Some(idx) => *idx,
                    None => self.find_table_ctrl_idx(*section, *table_para)?,
                };
                let resolved_cell_idx = match cell_idx {
                    Some(idx) => *idx,
                    None => self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?,
                };
                // [Sub-7] PartialCellStyle → native JSON (bgcolor → fillType+fillColor,
                // border.all → 4 방향 펼침, vertical_align 문자열 → u8).
                let json = partial_cell_style_to_native_json(style);
                self.set_cell_properties_native(*section, *table_para, ctrl_idx, resolved_cell_idx, &json)?;
            }
            EditOperation::MergeCells { section, table_para, row_start, col_start, row_end, col_end, ctrl_idx } => {
                let ctrl_idx = match ctrl_idx {
                    Some(idx) => *idx,
                    None => self.find_table_ctrl_idx(*section, *table_para)?,
                };
                self.merge_table_cells_native(
                    *section, *table_para, ctrl_idx,
                    *row_start as u16, *col_start as u16,
                    *row_end as u16, *col_end as u16,
                )?;
            }
            EditOperation::ReplaceCellRuns { section, table_para, row, col, cell_idx, ctrl_idx, cell_para, runs } => {
                let ctrl_idx = match ctrl_idx {
                    Some(idx) => *idx,
                    None => self.find_table_ctrl_idx(*section, *table_para)?,
                };
                let resolved_cell_idx = match cell_idx {
                    Some(idx) => *idx,
                    None => self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?,
                };
                // [Sub-7] runs 의 PartialRunStyle 들도 native 키로 변환.
                let runs_json = runs_to_native_json(self, runs)?;
                self.replace_cell_runs_native(*section, *table_para, ctrl_idx, resolved_cell_idx, *cell_para, &runs_json)?;
            }
            EditOperation::InsertTextInCell { section, table_para, row, col, cell_idx, ctrl_idx, cell_para, offset, text, style } => {
                let ctrl_idx = match ctrl_idx {
                    Some(idx) => *idx,
                    None => self.find_table_ctrl_idx(*section, *table_para)?,
                };
                let resolved_cell_idx = match cell_idx {
                    Some(idx) => *idx,
                    None => self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?,
                };
                let text_len = text.chars().count();
                self.insert_text_in_cell_native(
                    *section, *table_para, ctrl_idx, resolved_cell_idx, *cell_para, *offset, text,
                )?;
                if let Some(s) = style {
                    // [Sub-7] PartialRunStyle → native JSON + font_name → fontId 주입.
                    let json = partial_run_style_to_native_json(s);
                    let json = inject_font_id_into_run_style_json(
                        self, &json, s.font_name.as_deref(),
                    )?;
                    self.apply_char_format_in_cell_native(
                        *section, *table_para, ctrl_idx, resolved_cell_idx, *cell_para,
                        *offset, *offset + text_len, &json,
                    )?;
                }
            }
            EditOperation::DeleteRangeInCell { section, table_para, row, col, cell_idx, ctrl_idx, cell_para_start, char_start, cell_para_end, char_end } => {
                let ctrl_idx = match ctrl_idx {
                    Some(idx) => *idx,
                    None => self.find_table_ctrl_idx(*section, *table_para)?,
                };
                let resolved_cell_idx = match cell_idx {
                    Some(idx) => *idx,
                    None => self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?,
                };
                self.delete_range_native(
                    *section, *cell_para_start, *char_start, *cell_para_end, *char_end,
                    Some((*table_para, ctrl_idx, resolved_cell_idx)),
                )?;
            }
            EditOperation::InsertPageBreak { section, para, offset } => {
                self.insert_page_break_native(*section, *para, *offset)?;
            }
            // Task #m600-29 — nested cell path 따라 cell paragraph 의 runs 교체.
            EditOperation::ReplaceCellRunsAtPath { section, path, cell_para, runs } => {
                let runs_json = runs_to_native_json(self, runs)?;
                self.replace_cell_runs_at_path_native(*section, path, *cell_para, &runs_json)?;
            }
            // 한컴 Enter / Ctrl+Enter. 본문/셀 모드 분기는 payload key 자체로.
            EditOperation::PressEnter {
                section, para, table_para, row, col, cell_para,
                ctrl_idx, cell_idx, char_offset, count, style, page_break,
            } => {
                if let Some(tp) = table_para {
                    // ─── 셀 모드 ───────────────────────────────────────────
                    // page_break + 셀 모드 = 미지원. silent 무시 0건.
                    if page_break.unwrap_or(false) {
                        return Err(HwpError::RenderError(
                            "INVALID_PAYLOAD: 셀 안 page_break 미지원. 셀 모드에서 page_break:true 박지 마세요.".to_string()
                        ));
                    }
                    let row = row.ok_or_else(|| HwpError::RenderError(
                        "INVALID_PAYLOAD: 셀 모드 row 누락".to_string()
                    ))?;
                    let col = col.ok_or_else(|| HwpError::RenderError(
                        "INVALID_PAYLOAD: 셀 모드 col 누락".to_string()
                    ))?;
                    let cell_para = cell_para.ok_or_else(|| HwpError::RenderError(
                        "INVALID_PAYLOAD: 셀 모드 cell_para 누락".to_string()
                    ))?;
                    let ctrl_idx = match ctrl_idx {
                        Some(idx) => *idx,
                        None => self.find_table_ctrl_idx(*section, *tp)?,
                    };
                    let resolved_cell_idx = match cell_idx {
                        Some(idx) => *idx,
                        None => self.find_cell_idx(*section, *tp, ctrl_idx, row as u16, col as u16)?,
                    };

                    // 셀 paragraph 의 본문 길이 추출
                    let cell_text_len = {
                        let table_p = &self.document.sections[*section].paragraphs[*tp];
                        let ctrl = table_p.controls.get(ctrl_idx).ok_or_else(|| HwpError::RenderError(
                            format!("ctrl_idx {} 범위 초과", ctrl_idx)
                        ))?;
                        match ctrl {
                            crate::model::control::Control::Table(t) => {
                                let cell = t.cells.get(resolved_cell_idx).ok_or_else(|| HwpError::RenderError(
                                    format!("cell_idx {} 범위 초과", resolved_cell_idx)
                                ))?;
                                let para = cell.paragraphs.get(cell_para).ok_or_else(|| HwpError::RenderError(
                                    format!("cell_para {} 범위 초과", cell_para)
                                ))?;
                                para.text.chars().count()
                            }
                            _ => return Err(HwpError::RenderError(
                                format!("ctrl_idx {} 은 Table 이 아닙니다", ctrl_idx)
                            )),
                        }
                    };

                    let resolved_offset = if *char_offset < 0 {
                        cell_text_len
                    } else {
                        (*char_offset as usize).min(cell_text_len)
                    };

                    for i in 0..*count {
                        let target_cell_para = cell_para + i;
                        let target_offset = if i == 0 { resolved_offset } else { 0 };
                        self.split_paragraph_in_cell_native(
                            *section, *tp, ctrl_idx, resolved_cell_idx, target_cell_para, target_offset,
                        )?;
                        // style 옵션 — 셀 자리에는 별도 cell paragraph format helper 부재.
                        // 본문 paragraph apply_para_format 자세는 셀 자리에 직접 적용 못함.
                        // 향후 사이클에서 apply_cell_para_format_native 신설 자세 필요.
                        let _ = style;
                    }
                } else {
                    // ─── 본문 모드 ─────────────────────────────────────────
                    let para = para.ok_or_else(|| HwpError::RenderError(
                        "INVALID_PAYLOAD: 본문 모드 para 누락".to_string()
                    ))?;

                    let para_text_len = self.document.sections.get(*section)
                        .ok_or_else(|| HwpError::RenderError(format!("section {} 범위 초과", section)))?
                        .paragraphs.get(para)
                        .ok_or_else(|| HwpError::RenderError(format!("para {} 범위 초과", para)))?
                        .text.chars().count();

                    let resolved_offset = if *char_offset < 0 {
                        para_text_len
                    } else {
                        (*char_offset as usize).min(para_text_len)
                    };

                    for i in 0..*count {
                        let target_para = para + i;
                        let target_offset = if i == 0 { resolved_offset } else { 0 };
                        self.split_paragraph_native(*section, target_para, target_offset)?;
                        if let Some(s) = style {
                            // 새 paragraph 자리 = target_para + 1
                            let props_json = partial_paragraph_style_to_native_json(s);
                            self.apply_para_format_native(*section, target_para + 1, &props_json)?;
                        }
                        // page_break — 첫 번째 새 paragraph 만 페이지 분리
                        if i == 0 && page_break.unwrap_or(false) {
                            self.insert_page_break_native(*section, target_para + 1, 0)?;
                        }
                    }
                }
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
            EditOperation::PressEnter { .. } => {
                unreachable!("PressEnter variant uses snapshot stash for inverse");
            }
            EditOperation::InsertPageBreak { .. } => {
                unreachable!("Sub-8 variant uses snapshot stash for inverse");
            }
            EditOperation::ReplaceCellRunsAtPath { .. } => {
                unreachable!("Task #m600-29 variant uses snapshot stash for inverse");
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
        // [Sub-7] alignment → align rename. 직렬화 키는 camelCase `align`,
        // alias 로 `alignment` 도 deserialize 호환.
        let partial = PartialParagraphStyle {
            align: Some("right".to_string()),
            line_height: None,
            margin_left: None,
            margin_right: None,
            indent: None,
            spacing_before: None,
            spacing_after: None,
        };
        let json = serde_json::to_string(&partial).unwrap();
        assert_eq!(json, r#"{"align":"right"}"#);
    }

    #[test]
    fn test_run_spec_deserialize() {
        let json = r#"{"text":"안녕","style":{"bold":true}}"#;
        let run: RunSpec = serde_json::from_str(json).unwrap();
        assert_eq!(run.text, "안녕");
        assert!(run.style.is_some());
    }

    /// font-size 외부 인터페이스는 pt 실수. 서버 내부에서 1/100 pt 정수 (u16) 로 변환된다.
    #[test]
    fn test_font_size_pt_to_internal_u16() {
        // 15.5 pt → 1550 (1/100 pt)
        let json = r#"{"font-size": 15.5}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, Some(1550));

        // 정수 15 → 1500
        let json = r#"{"font-size": 15}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, Some(1500));

        // alias fontSize 도 동일 변환
        let json = r#"{"fontSize": 12.5}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, Some(1250));

        // alias base_size + 0 (최소 경계)
        let json = r#"{"base_size": 0}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, Some(0));

        // 반올림 — 14.567 pt → 1457 (1/100 pt round)
        let json = r#"{"font-size": 14.567}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, Some(1457));

        // 미지정 시 None
        let json = r#"{"bold": true}"#;
        let style: PartialRunStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.font_size, None);
    }

    /// font-size 가 문자열일 때는 reject (422).
    #[test]
    fn test_font_size_string_rejected() {
        let json = r#"{"font-size": "15pt"}"#;
        let result: Result<PartialRunStyle, _> = serde_json::from_str(json);
        assert!(result.is_err(), "문자열 font-size 는 reject 되어야 함");
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
                align: Some("right".to_string()),
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

    // ─── PressEnter 단위 테스트 ────────────────────────────────────────────
    // 본문 모드 7 + 셀 모드 7 + 에러 5 = 19 케이스 (spec docs/25 §6 단계 1.3)

    /// 본문 끝 (char_offset=-1) Enter — 원 본문 그대로, 새 빈 paragraph 가 +1 자리.
    /// `insert_paragraph` 의 *이름 ↔ 동작 어긋남* 사고 자체 해결 자세 검증.
    #[test]
    fn test_press_enter_body_end_default() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 2);
        assert_eq!(para_text(&core, 0, 0), "hello", "원 본문 그대로");
        assert_eq!(para_text(&core, 0, 1), "", "새 빈 paragraph 가 +1 자리");
    }

    /// 본문 시작 (char_offset=0) Enter — 빈 paragraph 가 앞, 원 본문이 +1 자리.
    /// 옛 `insert_paragraph` 의 동작 자세 (좌측 커서 + Enter).
    #[test]
    fn test_press_enter_body_start() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: 0, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 2);
        assert_eq!(para_text(&core, 0, 0), "", "앞에 빈 paragraph");
        assert_eq!(para_text(&core, 0, 1), "hello", "원 본문이 +1 자리");
    }

    /// char_offset = len 자세 → -1 (끝) 과 동등.
    #[test]
    fn test_press_enter_body_offset_equals_len() {
        let mut core = core_with_text("hello"); // len = 5
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: 5, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "hello");
        assert_eq!(para_text(&core, 0, 1), "");
    }

    /// char_offset > len 자세 → clamp to len. silent fail 0건.
    #[test]
    fn test_press_enter_body_offset_overflow_clamp() {
        let mut core = core_with_text("hello"); // len = 5
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: 100, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(para_text(&core, 0, 0), "hello");
        assert_eq!(para_text(&core, 0, 1), "");
    }

    /// 본문 중간 (char_offset=2) → "he" / "llo" 분할.
    #[test]
    fn test_press_enter_body_middle_split() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: 2, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 2);
        assert_eq!(para_text(&core, 0, 0), "he");
        assert_eq!(para_text(&core, 0, 1), "llo");
    }

    /// count = 3, char_offset = -1 → 원 본문 + 3 개 빈 paragraph 누적.
    #[test]
    fn test_press_enter_body_count_multi() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 3, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();
        assert_eq!(core.document.sections[0].paragraphs.len(), 4, "원 1 + 새 3");
        assert_eq!(para_text(&core, 0, 0), "hello");
        assert_eq!(para_text(&core, 0, 1), "");
        assert_eq!(para_text(&core, 0, 2), "");
        assert_eq!(para_text(&core, 0, 3), "");
    }

    /// JSON deserialize — char_offset 기본값 -1, count 기본값 1.
    #[test]
    fn test_press_enter_deserialize_defaults() {
        let json = r#"{"op":"press_enter","section":0,"para":3}"#;
        let op: EditOperation = serde_json::from_str(json).unwrap();
        if let EditOperation::PressEnter { char_offset, count, para, table_para, .. } = op {
            assert_eq!(char_offset, -1);
            assert_eq!(count, 1);
            assert_eq!(para, Some(3));
            assert_eq!(table_para, None);
        } else { panic!("Wrong variant"); }
    }

    // ─── 셀 모드 ─────────────────────────────────────────────────────────

    /// 셀 모드 끝 Enter — 셀 paragraph 가 추가 자세.
    #[test]
    fn test_press_enter_cell_end() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();
        // table paragraph 자리 = 1 (create_table_native 후 paragraphs[1] 이 표 자리).
        // 셀 (0,0) 의 첫 paragraph 에 본문 박음.
        let ctrl_idx = core.find_table_ctrl_idx(0, 1).unwrap();
        let cell_idx = core.find_cell_idx(0, 1, ctrl_idx, 0, 0).unwrap();
        core.insert_text_in_cell_native(0, 1, ctrl_idx, cell_idx, 0, 0, "AB").unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: Some(0), col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();

        // 셀 (0,0) 의 paragraphs 개수가 2 가 되어야.
        let table_p = &core.document.sections[0].paragraphs[1];
        if let crate::model::control::Control::Table(t) = &table_p.controls[ctrl_idx] {
            let cell = &t.cells[cell_idx];
            assert_eq!(cell.paragraphs.len(), 2);
            assert_eq!(cell.paragraphs[0].text, "AB", "원 본문 그대로");
            assert_eq!(cell.paragraphs[1].text, "", "새 빈 paragraph");
        } else { panic!("Not a Table"); }
    }

    /// 셀 모드 시작 Enter — 빈 paragraph 가 앞.
    #[test]
    fn test_press_enter_cell_start() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();
        let ctrl_idx = core.find_table_ctrl_idx(0, 1).unwrap();
        let cell_idx = core.find_cell_idx(0, 1, ctrl_idx, 0, 0).unwrap();
        core.insert_text_in_cell_native(0, 1, ctrl_idx, cell_idx, 0, 0, "AB").unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: Some(0), col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: 0, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();

        let table_p = &core.document.sections[0].paragraphs[1];
        if let crate::model::control::Control::Table(t) = &table_p.controls[ctrl_idx] {
            let cell = &t.cells[cell_idx];
            assert_eq!(cell.paragraphs.len(), 2);
            assert_eq!(cell.paragraphs[0].text, "", "빈 paragraph 앞");
            assert_eq!(cell.paragraphs[1].text, "AB", "원 본문 +1 자리");
        } else { panic!("Not a Table"); }
    }

    /// 셀 모드 중간 분할 — 셀 paragraph 본문 "ABCD" + char_offset=2 → "AB" / "CD".
    #[test]
    fn test_press_enter_cell_middle_split() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();
        let ctrl_idx = core.find_table_ctrl_idx(0, 1).unwrap();
        let cell_idx = core.find_cell_idx(0, 1, ctrl_idx, 0, 0).unwrap();
        core.insert_text_in_cell_native(0, 1, ctrl_idx, cell_idx, 0, 0, "ABCD").unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: Some(0), col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: 2, count: 1, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();

        let table_p = &core.document.sections[0].paragraphs[1];
        if let crate::model::control::Control::Table(t) = &table_p.controls[ctrl_idx] {
            let cell = &t.cells[cell_idx];
            assert_eq!(cell.paragraphs.len(), 2);
            assert_eq!(cell.paragraphs[0].text, "AB");
            assert_eq!(cell.paragraphs[1].text, "CD");
        } else { panic!("Not a Table"); }
    }

    /// 셀 모드 count > 1 — 새 paragraph N 개 누적.
    #[test]
    fn test_press_enter_cell_count_multi() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();
        let ctrl_idx = core.find_table_ctrl_idx(0, 1).unwrap();
        let cell_idx = core.find_cell_idx(0, 1, ctrl_idx, 0, 0).unwrap();
        core.insert_text_in_cell_native(0, 1, ctrl_idx, cell_idx, 0, 0, "X").unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: Some(0), col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 3, style: None, page_break: None,
        };
        core.apply_edit_op(&op).unwrap();

        let table_p = &core.document.sections[0].paragraphs[1];
        if let crate::model::control::Control::Table(t) = &table_p.controls[ctrl_idx] {
            let cell = &t.cells[cell_idx];
            assert_eq!(cell.paragraphs.len(), 4, "원 1 + 새 3");
            assert_eq!(cell.paragraphs[0].text, "X");
            assert_eq!(cell.paragraphs[1].text, "");
            assert_eq!(cell.paragraphs[2].text, "");
            assert_eq!(cell.paragraphs[3].text, "");
        } else { panic!("Not a Table"); }
    }

    // ─── 에러 케이스 ──────────────────────────────────────────────────────

    /// 본문 모드 + para 누락 → INVALID_PAYLOAD.
    #[test]
    fn test_press_enter_body_missing_para() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        let err = core.apply_edit_op(&op).unwrap_err();
        assert!(format!("{}", err).contains("INVALID_PAYLOAD"), "에러 메시지: {}", err);
        assert!(format!("{}", err).contains("para 누락"));
    }

    /// 셀 모드 + page_break:true → INVALID_PAYLOAD (셀 안 페이지 분리 미지원).
    #[test]
    fn test_press_enter_cell_page_break_rejected() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: Some(0), col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: Some(true),
        };
        let err = core.apply_edit_op(&op).unwrap_err();
        assert!(format!("{}", err).contains("INVALID_PAYLOAD"));
        assert!(format!("{}", err).contains("page_break"));
    }

    /// 셀 모드 + row 누락 → INVALID_PAYLOAD.
    #[test]
    fn test_press_enter_cell_missing_row() {
        let mut core = core_with_text("");
        core.create_table_native(0, 0, 0, 2, 2).unwrap();

        let op = EditOperation::PressEnter {
            section: 0, para: None,
            table_para: Some(1), row: None, col: Some(0), cell_para: Some(0),
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        let err = core.apply_edit_op(&op).unwrap_err();
        assert!(format!("{}", err).contains("INVALID_PAYLOAD"));
        assert!(format!("{}", err).contains("row 누락"));
    }

    /// 본문 모드 + section 범위 초과 → 에러.
    #[test]
    fn test_press_enter_body_section_out_of_range() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 99, para: Some(0),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        assert!(core.apply_edit_op(&op).is_err());
    }

    /// 본문 모드 + para 범위 초과 → 에러.
    #[test]
    fn test_press_enter_body_para_out_of_range() {
        let mut core = core_with_text("hello");
        let op = EditOperation::PressEnter {
            section: 0, para: Some(99),
            table_para: None, row: None, col: None, cell_para: None,
            ctrl_idx: None, cell_idx: None,
            char_offset: -1, count: 1, style: None, page_break: None,
        };
        assert!(core.apply_edit_op(&op).is_err());
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
            cell_idx: None,
            ctrl_idx: None,
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
            cell_idx: None,
            ctrl_idx: None,
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
            cell_idx: None,
            ctrl_idx: None,
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
            cell_idx: None,
            ctrl_idx: None,
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
            ctrl_idx: None,
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

    // ─── Sub-4: affected_range() variant 별 검증 ────────────────────────────

    #[test]
    fn affected_range_replace_runs_single_para() {
        let op = EditOperation::ReplaceRuns { section: 0, para: 5, runs: vec![] };
        let r = op.affected_range();
        assert_eq!(r.section, 0);
        assert_eq!(r.before, ParaRange::single(5));
        assert_eq!(r.after, ParaRange::single(5));
        assert!(r.cell.is_none());
    }

    #[test]
    fn affected_range_set_paragraph_style_single_para() {
        let op = EditOperation::SetParagraphStyle {
            section: 1, para: 3, style: PartialParagraphStyle::default(),
        };
        let r = op.affected_range();
        assert_eq!(r.section, 1);
        assert_eq!(r.before, ParaRange::single(3));
        assert_eq!(r.after, ParaRange::single(3));
    }

    #[test]
    fn affected_range_delete_range_multi_para_collapses_after() {
        let op = EditOperation::DeleteRange {
            section: 0, para_start: 2, char_start: 0, para_end: 5, char_end: 3,
        };
        let r = op.affected_range();
        // before = [2..6) (para_end inclusive → +1)
        assert_eq!(r.before, ParaRange { start: 2, end: 6 });
        // after collapses to single paragraph
        assert_eq!(r.after, ParaRange::single(2));
    }

    #[test]
    fn affected_range_insert_paragraph_after_expands() {
        let op = EditOperation::InsertParagraph {
            section: 0, after_para: 3, count: 2, style: None,
        };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(3));
        // after = [3..3+1+2) = [3..6)
        assert_eq!(r.after, ParaRange { start: 3, end: 6 });
    }

    #[test]
    fn affected_range_insert_table_after_expands() {
        let op = EditOperation::InsertTable {
            section: 0, insert_after_para: 4, rows: 2, cols: 3,
        };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(4));
        // after = [4..4+2) = [4..6) — 원래 paragraph + 새 표 paragraph
        assert_eq!(r.after, ParaRange { start: 4, end: 6 });
    }

    #[test]
    fn affected_range_delete_element_paragraph_empty_after() {
        let op = EditOperation::DeleteElement {
            section: 0, para: 7, element_type: ElementType::Paragraph,
        };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(7));
        assert_eq!(r.after, ParaRange::empty(7));
    }

    #[test]
    fn affected_range_delete_element_table_keeps_paragraph() {
        // 표 control 만 삭제 — paragraph 자체는 남는다.
        let op = EditOperation::DeleteElement {
            section: 0, para: 7, element_type: ElementType::Table,
        };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(7));
        assert_eq!(r.after, ParaRange::single(7));
    }

    #[test]
    fn affected_range_set_cell_style_carries_cell_focus() {
        let op = EditOperation::SetCellStyle {
            section: 0, table_para: 4, row: 2, col: 3,
            cell_idx: Some(11), ctrl_idx: None, style: PartialCellStyle::default(),
        };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(4));
        assert_eq!(r.after, ParaRange::single(4));
        let cell = r.cell.expect("cell focus 필요");
        assert_eq!(cell.table_para, 4);
        assert_eq!(cell.row, 2);
        assert_eq!(cell.col, 3);
        assert_eq!(cell.cell_idx, Some(11));
        assert_eq!(cell.cell_para, None);
    }

    #[test]
    fn affected_range_replace_cell_runs_carries_cell_para() {
        let op = EditOperation::ReplaceCellRuns {
            section: 0, table_para: 4, row: 1, col: 2,
            cell_idx: Some(6), ctrl_idx: None, cell_para: 0, runs: vec![],
        };
        let r = op.affected_range();
        let cell = r.cell.expect("cell focus 필요");
        assert_eq!(cell.cell_idx, Some(6));
        assert_eq!(cell.cell_para, Some(0));
    }

    #[test]
    fn affected_range_merge_cells_uses_start_coords() {
        let op = EditOperation::MergeCells {
            section: 0, table_para: 4,
            row_start: 1, col_start: 2, row_end: 3, col_end: 5,
            ctrl_idx: None,
        };
        let r = op.affected_range();
        let cell = r.cell.expect("cell focus 필요");
        assert_eq!(cell.row, 1);
        assert_eq!(cell.col, 2);
    }

    #[test]
    fn affected_range_split_paragraph_grows_after() {
        let op = EditOperation::SplitParagraph { section: 0, para: 5, offset: 3 };
        let r = op.affected_range();
        assert_eq!(r.before, ParaRange::single(5));
        assert_eq!(r.after, ParaRange { start: 5, end: 7 });
    }

    #[test]
    fn affected_range_insert_page_break_grows_after() {
        let op = EditOperation::InsertPageBreak { section: 0, para: 3, offset: 5 };
        let r = op.affected_range();
        assert_eq!(r.section, 0);
        assert_eq!(r.before, ParaRange::single(3));
        assert_eq!(r.after, ParaRange { start: 3, end: 5 });
        assert!(r.cell.is_none());
    }

    #[test]
    fn affected_range_merge_paragraph_consumes_prev() {
        let op = EditOperation::MergeParagraph { section: 0, para: 5, prev_len: 4 };
        let r = op.affected_range();
        // before = [4..6), after = [4..5)
        assert_eq!(r.before, ParaRange { start: 4, end: 6 });
        assert_eq!(r.after, ParaRange::single(4));
    }

    #[test]
    fn affected_range_merge_paragraph_at_zero_saturates() {
        // 안전: para=0 이면 prev=0 (실제로는 invalid 이지만 panic 방지 확인).
        let op = EditOperation::MergeParagraph { section: 0, para: 0, prev_len: 0 };
        let r = op.affected_range();
        assert_eq!(r.before.start, 0);
        assert_eq!(r.after.start, 0);
    }

    // ─── [Sub-7] Partial*Style schema 정합 / 변환 함수 단위 테스트 ──────────────

    #[test]
    fn partial_cell_style_deserializes_bgcolor() {
        // 광고 키 bgcolor 가 deserialize 되는지 확인 — Sub-7 이전엔 silent drop.
        let json = r##"{"bgcolor":"#FFC0CB"}"##;
        let s: PartialCellStyle = serde_json::from_str(json).unwrap();
        assert_eq!(s.bgcolor.as_deref(), Some("#FFC0CB"));
    }

    #[test]
    fn partial_cell_style_deny_unknown_fields_rejects_typo() {
        // deny_unknown_fields — 오타(bgClor) 는 400 에러로 반환되어야 함.
        let json = r##"{"bgClor":"#FFFFFF"}"##;
        let res: Result<PartialCellStyle, _> = serde_json::from_str(json);
        assert!(res.is_err(), "오타 키는 거부되어야 함");
    }

    #[test]
    fn partial_cell_style_to_native_json_includes_fill_type() {
        // bgcolor 지정 시 native 키 fillType=solid + fillColor 가 함께 출력.
        let s = PartialCellStyle {
            bgcolor: Some("#ABCDEF".to_string()),
            ..Default::default()
        };
        let native = partial_cell_style_to_native_json(&s);
        assert!(native.contains(r#""fillType":"solid""#), "native={native}");
        assert!(native.contains(r##""fillColor":"#ABCDEF""##), "native={native}");
    }

    #[test]
    fn partial_cell_style_border_all_expands_to_4dir() {
        // border.all 지정 시 4 방향 (borderLeft/Right/Top/Bottom) 모두 직렬화.
        let s = PartialCellStyle {
            border: Some(BorderSpec {
                all: Some(BorderLine {
                    color: Some("#000000".to_string()),
                    width: Some(10),
                    line_type: Some(1),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let native = partial_cell_style_to_native_json(&s);
        for k in ["borderLeft", "borderRight", "borderTop", "borderBottom"] {
            assert!(native.contains(&format!(r#""{k}":"#)), "{k} 부재: {native}");
        }
    }

    #[test]
    fn partial_cell_style_border_individual_overrides_all() {
        // all 적용 후 left 만 override 한 경우 left 의 색이 우선.
        let s = PartialCellStyle {
            border: Some(BorderSpec {
                all: Some(BorderLine {
                    color: Some("#000000".to_string()),
                    width: Some(10),
                    line_type: Some(1),
                }),
                left: Some(BorderLine {
                    color: Some("#FF0000".to_string()),
                    width: Some(20),
                    line_type: Some(2),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let native = partial_cell_style_to_native_json(&s);
        // left = 빨강, 나머지 = 검정
        assert!(native.contains(r##""borderLeft":{"color":"#FF0000""##), "{native}");
        assert!(native.contains(r##""borderTop":{"color":"#000000""##), "{native}");
    }

    #[test]
    fn partial_cell_style_vertical_align_string_to_u8() {
        for (s, expected) in [("top", 0u8), ("middle", 1), ("center", 1), ("bottom", 2)] {
            let style = PartialCellStyle {
                vertical_align: Some(s.to_string()),
                ..Default::default()
            };
            let native = partial_cell_style_to_native_json(&style);
            assert!(
                native.contains(&format!(r#""verticalAlign":{expected}"#)),
                "verticalAlign={s} expected={expected}, got {native}"
            );
        }
    }

    #[test]
    fn partial_run_style_color_alias_text_color() {
        // 광고 키 color, 기존 alias textColor / text_color 모두 deserialize.
        let a: PartialRunStyle = serde_json::from_str(r##"{"color":"#FF0000"}"##).unwrap();
        let b: PartialRunStyle = serde_json::from_str(r##"{"textColor":"#FF0000"}"##).unwrap();
        let c: PartialRunStyle = serde_json::from_str(r##"{"text_color":"#FF0000"}"##).unwrap();
        assert_eq!(a.color.as_deref(), Some("#FF0000"));
        assert_eq!(b.color.as_deref(), Some("#FF0000"));
        assert_eq!(c.color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn partial_run_style_font_size_alias_base_size() {
        // 광고 키 fontSize, 기존 alias baseSize / base_size 모두 deserialize.
        // 외부 인터페이스 = pt 실수, 내부 저장 = 1/100 pt 정수 (u16). 14 pt → 1400.
        let a: PartialRunStyle = serde_json::from_str(r#"{"fontSize":14}"#).unwrap();
        let b: PartialRunStyle = serde_json::from_str(r#"{"baseSize":14}"#).unwrap();
        let c: PartialRunStyle = serde_json::from_str(r#"{"base_size":14}"#).unwrap();
        assert_eq!(a.font_size, Some(1400));
        assert_eq!(b.font_size, Some(1400));
        assert_eq!(c.font_size, Some(1400));
    }

    #[test]
    fn partial_run_style_to_native_json_highlight_to_shade_color() {
        // 광고 키 highlight 가 native 키 shadeColor 로 변환.
        let s = PartialRunStyle {
            highlight: Some("#FFFF00".to_string()),
            ..Default::default()
        };
        let native = partial_run_style_to_native_json(&s);
        assert!(native.contains(r##""shadeColor":"#FFFF00""##), "{native}");
        assert!(!native.contains("highlight"), "highlight 키는 native 출력에 없어야 함: {native}");
    }

    #[test]
    fn partial_run_style_to_native_json_color_to_text_color() {
        let s = PartialRunStyle {
            color: Some("#00FF00".to_string()),
            font_size: Some(12),
            ..Default::default()
        };
        let native = partial_run_style_to_native_json(&s);
        assert!(native.contains(r##""textColor":"#00FF00""##), "{native}");
        assert!(native.contains(r#""fontSize":12"#), "{native}");
    }

    #[test]
    fn partial_run_style_deny_unknown_fields_rejects_typo() {
        let res: Result<PartialRunStyle, _> = serde_json::from_str(r##"{"colorr":"#FF0000"}"##);
        assert!(res.is_err());
    }

    #[test]
    fn partial_paragraph_style_align_alias() {
        // 광고 키 align, 기존 alias alignment 모두 deserialize.
        let a: PartialParagraphStyle = serde_json::from_str(r#"{"align":"right"}"#).unwrap();
        let b: PartialParagraphStyle = serde_json::from_str(r#"{"alignment":"right"}"#).unwrap();
        assert_eq!(a.align.as_deref(), Some("right"));
        assert_eq!(b.align.as_deref(), Some("right"));
    }

    #[test]
    fn partial_paragraph_style_line_height_alias_line_spacing() {
        let a: PartialParagraphStyle = serde_json::from_str(r#"{"lineHeight":200.0}"#).unwrap();
        let b: PartialParagraphStyle = serde_json::from_str(r#"{"lineSpacing":200.0}"#).unwrap();
        let c: PartialParagraphStyle = serde_json::from_str(r#"{"line_spacing":200.0}"#).unwrap();
        assert_eq!(a.line_height, Some(200.0));
        assert_eq!(b.line_height, Some(200.0));
        assert_eq!(c.line_height, Some(200.0));
    }

    #[test]
    fn partial_paragraph_style_to_native_json_align_to_alignment() {
        let s = PartialParagraphStyle {
            align: Some("center".to_string()),
            line_height: Some(150.0),
            ..Default::default()
        };
        let native = partial_paragraph_style_to_native_json(&s);
        assert!(native.contains(r#""alignment":"center""#), "{native}");
        assert!(native.contains(r#""lineSpacing":150"#), "{native}");
    }

    #[test]
    fn apply_set_cell_style_bgcolor_round_trip() {
        // SetCellStyle + bgcolor 적용 후 cell 의 border_fill_id 가 바뀌어야 한다 —
        // bgcolor 변환이 fillType=solid + fillColor 를 native 에 보내면
        // create_border_fill_from_json 가 새 BorderFill 을 만들어 cell.border_fill_id 갱신.
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.create_table_native(0, 0, 0, 2, 2).unwrap();

        let bfid_before = {
            let para = &core.document.sections[0].paragraphs[1];
            match &para.controls[0] {
                crate::model::control::Control::Table(t) => t.cells[0].border_fill_id,
                _ => panic!("Table 컨트롤 없음"),
            }
        };

        let op = EditOperation::SetCellStyle {
            section: 0,
            table_para: 1,
            row: 0,
            col: 0,
            cell_idx: None,
            ctrl_idx: None,
            style: PartialCellStyle {
                bgcolor: Some("#FFC0CB".to_string()),
                ..Default::default()
            },
        };
        core.apply_edit_op(&op).unwrap();

        let bfid_after = {
            let para = &core.document.sections[0].paragraphs[1];
            match &para.controls[0] {
                crate::model::control::Control::Table(t) => t.cells[0].border_fill_id,
                _ => panic!("Table 컨트롤 없음"),
            }
        };
        assert_ne!(
            bfid_before, bfid_after,
            "bgcolor 변경이 새 BorderFill 을 생성하고 cell.border_fill_id 를 바꿔야 한다"
        );
    }

    #[test]
    fn apply_insert_page_break_splits_and_sets_column_type() {
        use crate::model::paragraph::ColumnBreakType;
        // 본문 한 문단 들어 있는 빈 문서.
        let mut core = core_with_text("한 줄");
        let before_count = core.document.sections[0].paragraphs.len();

        let op = EditOperation::InsertPageBreak {
            section: 0,
            para: 0,
            offset: 1,
        };
        core.apply_edit_op(&op).unwrap();

        let secs = &core.document.sections;
        assert_eq!(
            secs[0].paragraphs.len(),
            before_count + 1,
            "문단이 둘로 분할되어야 한다"
        );
        assert_eq!(
            secs[0].paragraphs[1].column_type,
            ColumnBreakType::Page,
            "분할된 새 문단 (para+1) 에 page break 가 설정되어야 한다"
        );
    }

    #[test]
    fn apply_set_paragraph_style_align_via_advertised_key() {
        // 광고 키 align 으로 SetParagraphStyle 가 정상 적용되는지.
        let mut core = core_with_text("hello");
        let op = EditOperation::SetParagraphStyle {
            section: 0,
            para: 0,
            style: PartialParagraphStyle {
                align: Some("right".to_string()),
                ..Default::default()
            },
        };
        core.apply_edit_op(&op).unwrap();
        let result = core.get_para_properties_at_native(0, 0).unwrap();
        assert!(result.contains(r#""alignment":"right""#), "{result}");
    }

    #[test]
    fn legacy_alignment_alias_still_applies_via_json_path() {
        // 기존 e2e 호환: JSON 으로 {"alignment":"right"} 보내도 align 으로 받아 정상 적용.
        let mut core = core_with_text("hi");
        let op_json = r#"{"op":"set_paragraph_style","section":0,"para":0,"style":{"alignment":"left"}}"#;
        let op: EditOperation = serde_json::from_str(op_json).unwrap();
        core.apply_edit_op(&op).unwrap();
        let result = core.get_para_properties_at_native(0, 0).unwrap();
        assert!(result.contains(r#""alignment":"left""#), "{result}");
    }

    /// m400 sub-1 — 섹션 첫 문단의 controls 가 `[SectionDef, Table]` 모양일 때,
    /// 호출자가 control_idx=0 하드코딩으로 들어와도 fallback 으로 Table 자동 검색해야 한다.
    /// sim-1781219787 paragraphs[0] 의 1×1 표 셀 배경색 변경 사고 자리.
    #[test]
    fn find_cell_idx_falls_back_for_section_def_paragraph() {
        use crate::model::control::Control;
        use crate::model::document::{Section, SectionDef};
        use crate::model::paragraph::Paragraph;
        use crate::model::table::{Cell, Table};

        let mut core = DocumentCore::new_empty();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        // 섹션 첫 문단 모양 — SectionDef 가 controls 앞 자리에 박혀 있음
        para.controls.push(Control::SectionDef(Box::new(SectionDef::default())));
        let mut cell = Cell::default();
        cell.row = 0;
        cell.col = 0;
        let mut table = Table::default();
        table.row_count = 1;
        table.col_count = 1;
        table.cells.push(cell);
        para.controls.push(Control::Table(Box::new(table)));
        section.paragraphs.push(para);
        core.document.sections.push(section);

        // 옛 동작: control_idx=0 자리 = SectionDef → "control_idx=0 가 Table 아님" 사고
        // 새 동작: fallback 으로 controls 안 Table 자동 검색 → 셀 (0,0) 자리 = idx 0
        let cell_idx = core.find_cell_idx(0, 0, 0, 0, 0).unwrap();
        assert_eq!(cell_idx, 0);
    }

    /// m400 sub-1 — 섹션 내부 문단의 controls 가 `[Table]` 한 자리 자리, control_idx=0 자리가
    /// Table 자리. fallback 도입 후 *옛 동작 그대로 유지* 되어야 한다 (회귀 검증).
    #[test]
    fn find_cell_idx_direct_for_table_only_paragraph() {
        use crate::model::control::Control;
        use crate::model::document::Section;
        use crate::model::paragraph::Paragraph;
        use crate::model::table::{Cell, Table};

        let mut core = DocumentCore::new_empty();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        let mut table = Table::default();
        table.row_count = 2;
        table.col_count = 3;
        for r in 0..2u16 {
            for c in 0..3u16 {
                let mut cell = Cell::default();
                cell.row = r;
                cell.col = c;
                table.cells.push(cell);
            }
        }
        para.controls.push(Control::Table(Box::new(table)));
        section.paragraphs.push(para);
        core.document.sections.push(section);

        // control_idx=0 자리가 Table — 직접 매핑
        assert_eq!(core.find_cell_idx(0, 0, 0, 0, 0).unwrap(), 0);
        assert_eq!(core.find_cell_idx(0, 0, 0, 1, 2).unwrap(), 5);
    }
}
