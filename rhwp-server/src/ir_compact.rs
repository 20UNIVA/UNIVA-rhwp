//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

use rhwp::document_core::{AffectedRange, CellFocus, DocumentCore};
use rhwp::model::style::{
    Alignment, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape, UnderlineType,
};
use rhwp::model::table::Cell;
use rhwp::model::ColorRef;
use rhwp::renderer::style_resolver::{
    detect_lang_category, primary_font_name, ResolvedBorderStyle, ResolvedCharStyle,
};
use serde::Serialize;
use std::collections::HashMap;

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

/// `ResolvedCharStyle` + raw `CharShape` 의 두 짝 → `RunStyle` 평탄화.
///
/// 옛 TypeScript 원본 `rhwp-studio/src/llm-replay/style-map.ts::charPropsToRunStyle` 의
/// Rust 대응. *resolved* 측 (style_resolver 가 폰트 치환·언어별 폰트 풀이까지 마친 결과) 에
/// bold/italic/색·언어별 폰트 이름이 있고, *raw* 측 (`doc_info.char_shapes[id]`) 에는
/// 변환 전 원본 HWPUNIT 크기 (`base_size`) 가 남아있다. ts 의 `p.fontSize * 0.01` 정합 위해
/// `base_size / 100.0` 로 pt 단위 환산.
///
/// `lang_idx` 는 `detect_lang_category(ch)` 가 반환한 7개 언어 카테고리 인덱스 — 한국어=0/
/// 영어=1/한자=2/일본어=3/기타=4/기호=5/사용자=6. 언어별 폰트가 비어있으면 한국어로 폴백.
fn char_shape_to_run_style(
    cs: &ResolvedCharStyle,
    raw_cs: &CharShape,
    lang_idx: usize,
) -> RunStyle {
    // 언어별 폰트 이름. style_resolver 는 한컴 치환 체인을 평탄화한 *최종* 이름을 보유.
    let font_family_raw = cs.font_family_for_lang(lang_idx);
    let font_family = primary_font_name(font_family_raw).to_string();

    // 음영(형광펜) 색. resolved 의 `shade_color` 는 0x00FFFFFF (흰색=없음) 가 sentinel.
    // ts 원본은 `shadeColor` 가 없으면 키 자체를 생략 — `Option` 으로 표현.
    let highlight = if cs.shade_color == 0x00FFFFFF {
        None
    } else {
        Some(color_ref_to_css(cs.shade_color))
    };

    // 자간·장평 — *raw* CharShape 의 한국어(=인덱스 0) 값을 정수 그대로 전달.
    // resolved 측은 이미 px·비율로 환산되어 있어 모델 입력으로는 부적합 (init.md spec 은
    // HWP 원본 단위 정수).
    let char_spacing = raw_cs.spacings.first().copied().unwrap_or(0) as i32;
    let char_width = raw_cs.ratios.first().copied().unwrap_or(100) as i32;

    RunStyle {
        bold: Some(cs.bold),
        italic: Some(cs.italic),
        // resolved 의 `underline` 은 `UnderlineType` enum — None 외엔 모두 underline=true.
        underline: Some(!matches!(cs.underline, UnderlineType::None)),
        strikethrough: Some(cs.strikethrough),
        color: Some(color_ref_to_css(cs.text_color)),
        highlight,
        font_size: Some((raw_cs.base_size as f64) / 100.0),
        font_name: Some(font_family),
        char_spacing: Some(char_spacing),
        char_width: Some(char_width),
        vertical_align: Some(
            vertical_align_to_str(cs.subscript, cs.superscript).to_string(),
        ),
    }
}

/// 본체 `BorderLine` → `CellBorderSpec`. `line_type == None` 이면 `None` 반환 (해당 면 미설정).
///
/// 본체 `BorderLine` 은 4면 모두 *항상 존재* (배열) — `BorderFill::default()` 는 `Solid` 가
/// 기본값이지만 실제 폭이 0 이거나 line_type 이 `None` 인 경우 시각적으로 "테두리 없음".
/// 따라서 "선 없음" 판정은 `line_type == BorderLineType::None` 으로만 단정 (width=0 은
/// 본체 helper 에서 "굵기 0.1mm" 인덱스를 의미할 수 있어 sentinel 로 쓰면 안 됨).
fn border_line_to_spec(b: &BorderLine) -> Option<CellBorderSpec> {
    if matches!(b.line_type, BorderLineType::None) {
        return None;
    }
    Some(CellBorderSpec {
        // 선 종류는 표 27 의 인덱스 (0=None, 1=Solid, ...). enum 값을 u8 로 캐스팅.
        border_type: Some(b.line_type as u8),
        width: Some(b.width as i32),
        color: Some(color_ref_to_css(b.color)),
    })
}

/// `Cell` + 옵션 `ResolvedBorderStyle` → `CellStyle`.
///
/// 옛 TypeScript 원본 `style-map.ts::cellPropsToCellStyle` 의 Rust 대응. 셀의 *배경색*과
/// *4면 테두리* 는 `Cell` 자체가 아닌 `border_fill_id` 가 가리키는 `BorderFill` 테이블 항목
/// 에 들어있다 — style_resolver 가 그 BorderFill 을 `ResolvedBorderStyle` 로 풀어둔 상태로
/// 받는다.
///
/// `border_style` 이 `None` 이면 배경·테두리 모두 미설정 (셀에 border_fill_id=0 인 경우).
/// `all` 은 Phase 4 빌더에서 항상 `None` 으로 두고, Phase 5 압축이 4면이 동일한 경우에만 `all`
/// 한 칸으로 축약 — Phase 1 에서 정의한 invariant 그대로.
fn cell_to_cell_style(cell: &Cell, border_style: Option<&ResolvedBorderStyle>) -> CellStyle {
    // 배경색·테두리는 ResolvedBorderStyle 에서 분리.
    let bgcolor = border_style
        .and_then(|bs| bs.fill_color)
        .map(color_ref_to_css);

    let border = border_style.map(|bs| CellBorder {
        // 본체 borders 배열 인덱스: 0=좌, 1=우, 2=상, 3=하 (BorderFill 정의 정합).
        left: border_line_to_spec(&bs.borders[0]),
        right: border_line_to_spec(&bs.borders[1]),
        top: border_line_to_spec(&bs.borders[2]),
        bottom: border_line_to_spec(&bs.borders[3]),
        all: None, // 압축 단계에서 채움 — Phase 1 invariant.
    });

    // 4면 모두 미설정이면 border 키 자체 omit (init.md spec 의 sparse 표현).
    let border = border.filter(|b| {
        b.left.is_some() || b.right.is_some() || b.top.is_some() || b.bottom.is_some()
    });

    CellStyle {
        bgcolor,
        width: Some(cell.width as i32),
        height: Some(cell.height as i32),
        border,
        vertical_align: Some(cell_vertical_align_to_str(cell.vertical_align).to_string()),
    }
}

/// `ParaShape` → `ParagraphStyle`.
///
/// 옛 TypeScript 원본 `style-map.ts::paraPropsToParaStyle` 의 Rust 대응. 정렬/들여쓰기 두
/// 키는 항상 전달, *줄 간격은 `Percent` 타입일 때만* 전달 — 다른 타입 (Fixed/SpaceOnly/
/// Minimum) 은 HWP 내부 단위(HWPUNIT)·줄높이 배수가 섞여 모델 입력 단위가 통일되지 않으므로,
/// init.md spec 은 *Percent 만* 채택하기로 한 결정 (ts 원본 `style-map.ts:62` 도 동일 분기).
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
    /// 이 문단이 *새 페이지 시작*(ColumnBreakType::Page)이면 `Some(true)`. 아니면 None.
    /// 모델이 어느 문단이 page break 를 지녔는지 outline 에서 바로 보게 해, page_break 후
    /// "para N = 새 페이지" 를 추측하지 않게 한다(로그 0701 사고 대응).
    #[serde(skip_serializing_if = "Option::is_none")] pub page_break: Option<bool>,
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

/// 문단의 *문자 단위 스타일* 을 받아 *인접 동일 스타일 run 을 병합* 한 `IrRun` 배열을 반환.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::collectRuns` 의 Rust 대응. `style_at`
/// 람다는 *글자 오프셋* 마다 호출되어 그 위치의 `RunStyle` 을 돌려준다 — 호출자는 본문 텍스트
/// 의 char_shape_id_at 과 ResolvedCharStyle 을 합쳐 `char_shape_to_run_style` 호출 결과를
/// 넣는다.
///
/// 빈 문단 (len=0) 에 대해서는 `char_offset=0, length=0, text="", style=style_at(0)` 1건을
/// 반환 — IR slice 가 빈 문단도 "1개 run" 으로 표현하기로 한 init.md spec 정합.
///
/// Sub-4 v3 — 빈 문단의 placeholder run style 을 `RunStyle::default()` 대신 *paragraph
/// 의 첫 char_shape* 로 채운다. 빈 셀이라도 셀에 묶여 있는 char_shape (색·글자 크기 등)
/// 가 응답에 드러나, 모델이 *이 자리에 글자를 넣었을 때 어떤 스타일로 보일지* 미리
/// 확인 가능. 이전 동작은 `replace_cell_runs` 호출 후 텍스트가 갑자기 셀의 기존 색으로
/// 그려져 모델이 진단하기 어려웠다.
fn collect_runs<F>(text: &str, len: usize, mut style_at: F) -> Vec<IrRun>
where
    F: FnMut(usize) -> RunStyle,
{
    if len == 0 {
        return vec![IrRun {
            char_offset: 0,
            length: 0,
            text: String::new(),
            style: style_at(0),
        }];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut runs: Vec<IrRun> = Vec::new();
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

/// 본문 문단 (`sec`, `para` 인덱스) 의 IR 표현을 빌드.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildTextParagraph` 의 Rust 대응. 절차:
/// 1. `core.document().sections[sec].paragraphs[para]` 에서 본문 텍스트와 길이 추출
/// 2. `para_shape_id` 가 가리키는 `doc_info.para_shapes[id]` 를 `para_shape_to_para_style`
///    으로 변환 — 인덱스가 범위를 벗어나면 `ParagraphStyle::default()` 로 폴백
/// 3. 각 글자 오프셋마다 `char_shape_id_at(off)` 로 char_shape 인덱스를 얻고, *resolved*
///    (`core.styles().char_styles[id]`) 와 *raw* (`doc_info.char_shapes[id]`) 둘을 함께 가져와
///    `char_shape_to_run_style` 호출. 둘 중 하나라도 없으면 `RunStyle::default()` 로 폴백
/// 4. `collect_runs` 로 인접 동일 스타일 run 을 병합
///
/// 빈 문단 (텍스트 길이 0) 은 `collect_runs` 가 `length=0` 1건을 반환 — IR 응답 구조 유지.
fn build_text_paragraph(core: &DocumentCore, sec: usize, para: usize) -> IrTextParagraph {
    let p = &core.document().sections[sec].paragraphs[para];
    let len = p.text.chars().count();

    // 문단 모양 — para_shape_id 가 가리키는 doc_info.para_shapes[id] (범위 밖이면 default).
    let para_style = core
        .document()
        .doc_info
        .para_shapes
        .get(p.para_shape_id as usize)
        .map(para_shape_to_para_style)
        .unwrap_or_default();

    // 인접 run 병합 — 각 글자 위치의 char_shape_id 를 얻어 resolved + raw 둘로 run style 합성.
    let chars: Vec<char> = p.text.chars().collect();
    let runs = collect_runs(&p.text, len, |off| {
        let id = p.char_shape_id_at(off).unwrap_or(0) as usize;
        let resolved = core.styles().char_styles.get(id);
        let raw = core.document().doc_info.char_shapes.get(id);
        match (resolved, raw) {
            (Some(rs), Some(rw)) => {
                // detect_lang_category 는 char 1 개를 받아 7개 카테고리 중 하나의 인덱스 반환.
                let lang_idx = chars.get(off).copied().map(detect_lang_category).unwrap_or(0);
                char_shape_to_run_style(rs, rw, lang_idx)
            }
            _ => RunStyle::default(),
        }
    });

    // 새 페이지 시작 문단이면 page_break 마커 노출.
    let page_break = if matches!(
        p.column_type,
        rhwp::model::paragraph::ColumnBreakType::Page
    ) {
        Some(true)
    } else {
        None
    };

    IrTextParagraph {
        id: format!("p_{}_{}", sec, para),
        sec,
        para: para as i64,
        kind: "text",
        style: para_style,
        runs,
        cell_locator: None,
        page_break,
    }
}

/// 셀 안 문단 (`sec`, `parent_para`, `control_idx`, `cell_idx`, `cell_para`) 의 IR 표현 빌드.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildCellParagraph` 의 Rust 대응. 절차는
/// `build_text_paragraph` 와 거의 같지만 두 가지가 다르다.
///
/// 1. *id 형식* — 본문 문단은 `p_{sec}_{para}` 이지만 셀 안 문단은 셀 좌표·문단 인덱스를 포함한
///    `p_{sec}_{table_para}_c{ctrl_idx}_{cell_idx}_{cell_para}` 로 구별. *flatten 한 평탄 entry*
///    가 본문 문단과 동일 컬렉션에 섞일 때 충돌 방지.
/// 2. *cell_locator* — 모델이 셀의 *행/열* 을 알 수 있도록 `CellLocator` 를 채워둔다.
///    `para` 필드 는 `-1` 로 두어 본문 문단의 인덱스(0..)와 구별.
///
/// 인덱스가 범위 밖이거나 `Control` 이 표가 아니면 `cell_para_ref = None` → 텍스트·스타일은
/// 기본값, runs 는 빈 placeholder 1건. *함수는 panic 없이 항상 한 개의 IrTextParagraph 를 반환*.
fn build_cell_paragraph(
    core: &DocumentCore,
    sec: usize,
    parent_para: usize,
    control_idx: usize,
    cell_idx: usize,
    cell_para: usize,
    cell_row: u16,
    cell_col: u16,
) -> IrTextParagraph {
    // 1) 셀 안 문단 참조 — Control::Table(t) 에서 cells[cell_idx].paragraphs[cell_para].
    //    Control::Table 은 Box<Table> 이라 패턴 매칭에서 t 는 &Box<Table> → 자동 deref.
    let cell_para_ref = core
        .document()
        .sections
        .get(sec)
        .and_then(|s| s.paragraphs.get(parent_para))
        .and_then(|p| p.controls.get(control_idx))
        .and_then(|ctrl| match ctrl {
            rhwp::model::control::Control::Table(t) => t
                .cells
                .get(cell_idx)
                .and_then(|c| c.paragraphs.get(cell_para)),
            _ => None,
        });

    // 2) 텍스트·길이·para_shape_id 추출 — 셀 문단이 없으면 모두 기본값.
    let (text, para_shape_id) = cell_para_ref
        .map(|p| (p.text.clone(), p.para_shape_id))
        .unwrap_or_default();
    let len = text.chars().count();

    // 3) 문단 스타일.
    let para_style = core
        .document()
        .doc_info
        .para_shapes
        .get(para_shape_id as usize)
        .map(para_shape_to_para_style)
        .unwrap_or_default();

    // 4) runs — char_shape_id_at + char_shape_to_run_style.
    let chars: Vec<char> = text.chars().collect();
    let runs = collect_runs(&text, len, |off| {
        if let Some(p) = cell_para_ref {
            let id = p.char_shape_id_at(off).unwrap_or(0) as usize;
            let resolved = core.styles().char_styles.get(id);
            let raw = core.document().doc_info.char_shapes.get(id);
            if let (Some(rs), Some(rw)) = (resolved, raw) {
                let lang_idx = chars
                    .get(off)
                    .copied()
                    .map(detect_lang_category)
                    .unwrap_or(0);
                return char_shape_to_run_style(rs, rw, lang_idx);
            }
        }
        RunStyle::default()
    });

    IrTextParagraph {
        id: format!(
            "p_{}_{}_c{}_{}_{}",
            sec, parent_para, control_idx, cell_idx, cell_para
        ),
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
        // 셀 안 문단의 page break 는 다루지 않는다(셀 모드 page_break 미지원).
        page_break: None,
    }
}

/// 셀 한 칸의 IR 표현을 빌드. `Control::Table` 이 아니거나 셀이 범위 밖이면 `None`.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::tryBuildCell` 의 Rust 대응. *셀의 4면 테두리/
/// 배경* 은 `Cell::border_fill_id` 가 가리키는 `BorderFill` 항목에 있고, style_resolver 가 이미
/// 풀어둔 `ResolvedBorderStyle` 을 `core.styles().border_styles` 에서 *1-indexed* 로 조회한다
/// (`border_fill_id=1` → `border_styles[0]`). 본체 `layout/tests.rs:675` 의 invariant 그대로.
/// `border_fill_id=0` 인 셀은 *배경·테두리 모두 미설정* 으로 보고 `None` 을 전달.
fn try_build_cell(
    core: &DocumentCore,
    sec: usize,
    parent_para: usize,
    control_idx: usize,
    cell_idx: usize,
) -> Option<IrTableCell> {
    let parent = core
        .document()
        .sections
        .get(sec)?
        .paragraphs
        .get(parent_para)?;
    let table = match parent.controls.get(control_idx)? {
        rhwp::model::control::Control::Table(t) => t,
        _ => return None,
    };
    let cell = table.cells.get(cell_idx)?;

    let row = cell.row;
    let col = cell.col;
    let row_span = cell.row_span;
    let col_span = cell.col_span;

    // border_fill_id 가 0 이면 BorderFill 참조 없음 → border_style = None.
    // 1-indexed: id=1 이 border_styles[0]. saturating_sub(1) 로 안전 변환.
    let border_style = if cell.border_fill_id > 0 {
        core.styles()
            .border_styles
            .get((cell.border_fill_id - 1) as usize)
    } else {
        None
    };
    let style = cell_to_cell_style(cell, border_style);

    let mut paragraphs = Vec::with_capacity(cell.paragraphs.len());
    for cp in 0..cell.paragraphs.len() {
        paragraphs.push(IrParagraph::Text(build_cell_paragraph(
            core,
            sec,
            parent_para,
            control_idx,
            cell_idx,
            cp,
            row,
            col,
        )));
    }

    Some(IrTableCell {
        row,
        col,
        // span = 1 은 spec 의 *기본값* — 키 자체를 omit. span > 1 일 때만 채움.
        row_span: if row_span > 1 { Some(row_span) } else { None },
        col_span: if col_span > 1 { Some(col_span) } else { None },
        style,
        paragraphs,
    })
}

/// 본문 문단의 `controls[control_idx]` 가 표인 경우 표 한 개의 IR 빌드.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildTableParagraph` 의 Rust 대응. 셀 순회는
/// `Table::cells` 의 *행 우선 순서* 그대로 — `Table::rebuild_grid` 가 sort 후 보장.
fn build_table_paragraph(
    core: &DocumentCore,
    sec: usize,
    para: usize,
    control_idx: usize,
) -> Option<IrTableParagraph> {
    let parent = core
        .document()
        .sections
        .get(sec)?
        .paragraphs
        .get(para)?;
    let table = match parent.controls.get(control_idx)? {
        rhwp::model::control::Control::Table(t) => t,
        _ => return None,
    };

    let rows = table.row_count;
    let cols = table.col_count;
    let cell_count = table.cells.len();

    let mut cells = Vec::with_capacity(cell_count);
    for cell_idx in 0..cell_count {
        if let Some(c) = try_build_cell(core, sec, para, control_idx, cell_idx) {
            cells.push(c);
        }
    }

    Some(IrTableParagraph {
        id: format!("p_{}_{}", sec, para),
        sec,
        para,
        kind: "table",
        rows,
        cols,
        cells,
    })
}

/// 본문 문단 한 개 → IR slice 의 paragraphs[] 평탄 entry 묶음.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildParagraph` 의 Rust 대응. 절차:
/// 1. 본문 문단이 *표 control* 을 가지면 → 표 본체 (`IrParagraph::Table`) 한 건 +
///    각 셀의 모든 문단을 평탄화한 `IrParagraph::Text` 들을 *같은 paragraphs[] 안* 에
///    나란히 push. 모델이 표와 셀 내용을 한 배열에서 동시에 볼 수 있도록 한다 — 옛 ts
///    원본의 *flatten 평탄 표현* 규약 (init.md spec 의 paragraphs[] 정의 정합).
/// 2. 표 control 이 없으면 → 일반 텍스트 문단 1건만 반환.
///
/// 한 문단에 표 control 이 둘 이상이면 *첫 표만* 처리 (옛 ts 원본도 동일 — 한 문단에
/// 표는 하나라는 HWP 관습).
fn build_paragraph(core: &DocumentCore, sec: usize, para: usize) -> Vec<IrParagraph> {
    let p = match core
        .document()
        .sections
        .get(sec)
        .and_then(|s| s.paragraphs.get(para))
    {
        Some(p) => p,
        None => return vec![],
    };

    // 첫 번째 Table control 검색.
    for (ci, ctrl) in p.controls.iter().enumerate() {
        if matches!(ctrl, rhwp::model::control::Control::Table(_)) {
            if let Some(table) = build_table_paragraph(core, sec, para, ci) {
                // Sub-3 v2: 셀 평탄 entry 제거 — nested cell_locator 가 이미 4 좌표 보유.
                // 모델은 table.cells[i].paragraphs[j] 안에서 직접 cell_locator 추출.
                return vec![IrParagraph::Table(table)];
            }
        }
    }

    // 표 없음 → 일반 텍스트 문단 1건.
    vec![IrParagraph::Text(build_text_paragraph(core, sec, para))]
}

/// `build_ir_slice` 의 입력 파라미터.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildIRSlice` 의 옵션 객체와 정합. `sec`
/// 은 섹션 인덱스, `para_start..para_end` 는 *반열림 구간*. `para_end == None` 이면 섹션의
/// 마지막 문단까지, `edit_session_id == None` 이면 현재 시각 (ms) 기반 자동 생성.
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub sec: usize,
    pub para_start: usize,
    pub para_end: Option<usize>,
    pub edit_session_id: Option<String>,
    /// Sub-3 v2: 페이지 단위 슬라이스. `Some(n)` 이면 paginator 결과에서 *문서 전체 0-based*
    /// 페이지 `n` 의 paragraph 범위로 sec/para_start/para_end 를 *덮어쓴다*. 범위 외이거나
    /// 페이지에 paragraph 가 없으면 sec/para_start/para_end 폴백.
    pub page: Option<u32>,
    /// 브라우저 (rhwp-studio WASM) 가 직접 계산한 page → (sec, para_start, para_end) 매핑.
    /// `Some` 이고 `page` 도 `Some` 이면 native `page_to_para_range` 를 *건너뛰고* 이 값을 사용한다.
    /// 측정기 격차 (native EmbeddedTextMeasurer ↔ WASM Canvas) 로 페이지 경계가 어긋날 때
    /// *사용자가 본 화면* 을 진실로 삼기 위한 우회 경로.
    pub page_override_range: Option<(usize, usize, usize)>,
    /// 클라이언트 page map 의 총 페이지 수. `Some` 이면 응답 `doc_meta.total_pages` 에 사용.
    pub total_pages_override: Option<u32>,

    // ─── 신규 응답 옵션 4 키 ─────────────────────────────────────────
    /// 응답 세부 단계 — raw / compact / outline / structure.
    pub detail: Detail,
    /// run·paragraph style 강도 — full / essential / none.
    pub include_style: StyleLevel,
    /// 표 정보 강도 — full / structure / count.
    pub include_tables: TableLevel,
    /// outline / structure 단계에서 paragraph 별 텍스트 자르기 (글자 단위).
    /// compact / raw 단계에서는 무시.
    pub max_text_chars: Option<u32>,
}

/// 응답 세부 단계. detail query 키에 매핑.
///
/// - `Raw` — 현 raw 분기 그대로 (Paragraph Serialize derive 결과)
/// - `Compact` — 현 compact 분기 그대로 (기본값)
/// - `Outline` — paragraph 별 첫 N 글자 + style.align 만, runs 없음
/// - `Structure` — paragraph idx + type + 글자 수만, 본문 텍스트 없음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Detail {
    Raw,
    #[default]
    Compact,
    Outline,
    Structure,
}

/// style 강도. include_style query 키에 매핑.
///
/// - `Full` — 모든 RunStyle / ParagraphStyle 키 유지 (현 compact 동작)
/// - `Essential` — 핵심 키만 (기본값). RunStyle: bold/italic/color/highlight/font-size/font-name.
///   ParagraphStyle: align/indent/line-height. 나머지는 omit.
/// - `None` — style 자체 omit (paragraph·run·cell 모두)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleLevel {
    Full,
    #[default]
    Essential,
    None,
}

/// 표 정보 강도. include_tables query 키에 매핑.
///
/// - `Full` — 현 compact 동일, 셀 안 paragraph 까지 (기본값)
/// - `Structure` — rows/cols + 셀 row/col/row_span/col_span + style.border/bgcolor 만. 셀 안 paragraph 빼기
/// - `Count` — "표가 있다" 표시만. type/rows/cols 만, cells 자체 omit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableLevel {
    #[default]
    Full,
    Structure,
    Count,
}

/// essential 단계에서 유지할 RunStyle 키. 나머지 키는 omit.
const ESSENTIAL_RUN_STYLE_KEYS: &[&str] = &[
    "bold",
    "italic",
    "color",
    "highlight",
    "font-size",
    "font-name",
];

/// essential 단계에서 유지할 ParagraphStyle 키.
const ESSENTIAL_PARA_STYLE_KEYS: &[&str] = &["align", "indent", "line-height"];

/// 페이지 번호 → `(sec, para_start, para_end)` 매핑.
///
/// `page_num` 은 *문서 전체 페이지 0-based* — paginator 가 섹션별로 분할한 `PaginationResult`
/// 들의 `pages` 를 *섹션 순서* 로 평탄화한 인덱스. 페이지가 범위 밖이거나 그 페이지에 paragraph
/// 가 하나도 없으면 `None`. `para_end` 는 *exclusive*.
///
/// `PageItem` 의 모든 variant 가 `para_index` (또는 `para_index` + 다른 필드) 를 가지므로
/// 각 variant 에서 `para_index` 를 추출 → min/max+1 으로 [start, end) 구간 산정.
pub fn page_to_para_range(
    core: &DocumentCore,
    page_num: u32,
) -> Option<(usize, usize, usize)> {
    use rhwp::renderer::pagination::PageItem;

    let mut global_idx: u32 = 0;
    for pr in core.pagination().iter() {
        for page in &pr.pages {
            if global_idx == page_num {
                // 페이지의 모든 단을 가로질러 paragraph 인덱스 수집.
                let mut pis: Vec<usize> = Vec::new();
                for col in &page.column_contents {
                    for item in &col.items {
                        let pi = match item {
                            PageItem::FullParagraph { para_index } => *para_index,
                            PageItem::PartialParagraph { para_index, .. } => *para_index,
                            PageItem::Table { para_index, .. } => *para_index,
                            PageItem::PartialTable { para_index, .. } => *para_index,
                            PageItem::Shape { para_index, .. } => *para_index,
                        };
                        pis.push(pi);
                    }
                }
                if pis.is_empty() {
                    return None;
                }
                // 페이지가 속한 섹션 — PageContent.section_index.
                let sec = page.section_index;
                let start = *pis.iter().min().unwrap();
                let end = *pis.iter().max().unwrap() + 1;
                return Some((sec, start, end));
            }
            global_idx += 1;
        }
    }
    None
}

/// IR slice 진입점 — *텍스트 path 만* 처리 (표 처리는 Phase 4).
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildIRSlice` 의 Rust 대응 중 텍스트 부분.
/// `para_start..para_end` 가 섹션의 문단 수를 초과하면 끝쪽 경계를 잘라 panic 없이 빈 slice 를
/// 반환. `edit_session_id` 미지정 시 `std::time::SystemTime::now()` 기반 ms 타임스탬프로 채움.
pub fn build_ir_slice(core: &DocumentCore, opts: &BuildOptions) -> IrSlice {
    // Sub-3 v2: page 지정 시 paginator 결과로 sec/start/end 덮어씀. 범위 외 / 빈 페이지면 fallback.
    // 클라이언트가 사전 계산한 page_override_range 가 있으면 native paginator 를 건너뛴다.
    let (sec, start, end) = if let Some(p) = opts.page {
        if let Some(triple) = opts.page_override_range {
            triple
        } else if let Some(triple) = page_to_para_range(core, p) {
            triple
        } else {
            let sec = opts.sec;
            let total = core
                .document()
                .sections
                .get(sec)
                .map(|s| s.paragraphs.len())
                .unwrap_or(0);
            let start = opts.para_start.min(total);
            let end = opts.para_end.unwrap_or(total).min(total);
            (sec, start, end)
        }
    } else {
        let sec = opts.sec;
        let total = core
            .document()
            .sections
            .get(sec)
            .map(|s| s.paragraphs.len())
            .unwrap_or(0);
        let start = opts.para_start.min(total);
        let end = opts.para_end.unwrap_or(total).min(total);
        (sec, start, end)
    };

    // edit_session_id — 미지정 시 ms 단위 timestamp 로 자동 생성. chrono 의존을 피하기 위해
    // std::time::SystemTime 직접 사용.
    let edit_session_id = opts.edit_session_id.clone().unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("ed_{}", ms)
    });

    let mut paragraphs = Vec::with_capacity(end.saturating_sub(start));
    for p in start..end {
        // 본문 문단마다 *표 분기* 거쳐 평탄 entry 묶음을 반환받는다 — 텍스트만 있으면 1건,
        // 표가 있으면 표 본체 + 셀 평탄 entry 들로 확장.
        paragraphs.extend(build_paragraph(core, sec, p));
    }

    // m500 — paginator 결과로 실제 페이지 수 계산. rendering.rs:2715 패턴 정합.
    // 빈 paginator (paginator 미실행 / 빈 문서) 자리는 1 fallback.
    // 클라이언트 page map 이 함께 들어왔으면 그 쪽 총 페이지 수를 우선 사용.
    let total_pages: u32 = opts.total_pages_override.unwrap_or_else(|| {
        core.pagination()
            .iter()
            .map(|p| p.pages.len() as u32)
            .sum::<u32>()
            .max(1)
    });
    // opts.page 는 *0-based 내부 인덱스* (BuildOptions 문서 정합).
    // 응답 doc_meta.page 는 *1-based 표시* — page=1 이 첫 페이지.
    // m400 sub-2 의 main.rs 변환 (외부 1-based → 내부 0-based) 과 정합.
    let page_display: u32 = opts.page.map(|p| p + 1).unwrap_or(1);

    IrSlice {
        doc_meta: IrDocMeta {
            edit_session_id,
            page: page_display,
            total_pages,
            anchor: IrAnchor {
                sec,
                para_start: start,
                para_end: end,
            },
        },
        paragraphs,
    }
}

/// 가장 흔한 값(mode) 을 돌려준다. 동률 시 *먼저 등장한 값* 우선 — 옛 ts 원본 동작 정합.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::mode` 의 Rust 대응. 직렬화 가능한 임의 타입
/// 을 받아 *JSON 문자열* 키로 빈도 카운트 — `f64` 처럼 `Hash` 가 없는 타입도 지원하기 위함.
/// 직렬화 실패한 항목은 *조용히 무시* (carbon copy 가 NaN 등 비정상 값일 가능성을 차단).
fn mode<T: Clone + serde::Serialize>(arr: &[T]) -> Option<T> {
    if arr.is_empty() {
        return None;
    }
    let mut counts: HashMap<String, (T, usize)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for v in arr {
        let k = match serde_json::to_string(v) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let entry = counts.entry(k.clone()).or_insert_with(|| {
            order.push(k.clone());
            (v.clone(), 0)
        });
        entry.1 += 1;
    }
    // count 가 큰 것 우선, 동률이면 *먼저 등장한 순서* (order 인덱스가 작은 것) 우선.
    order
        .iter()
        .enumerate()
        .map(|(idx, k)| {
            let (val, cnt) = counts.get(k).unwrap();
            (idx, val.clone(), *cnt)
        })
        .max_by(|(ia, _, ca), (ib, _, cb)| ca.cmp(cb).then(ib.cmp(ia)))
        .map(|(_, v, _)| v)
}

/// 문서 전체 paragraph 를 순회해 가장 흔한 font-size/font-name 으로 defaults 산정.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::computeDocDefaults` 의 Rust 대응. font-size 와
/// font-name 만 *통계 산정* — 나머지 키 (bold/italic/color 등) 는 *상수 기본값* 으로 고정한다.
/// ts 원본도 동일 패턴 (`charPropsToRunStyle` 기본값을 그대로 박아넣음).
fn compute_doc_defaults(ir: &IrSlice) -> DocDefaults {
    let mut sizes: Vec<f64> = Vec::new();
    let mut fonts: Vec<String> = Vec::new();

    fn visit(p: &IrParagraph, sizes: &mut Vec<f64>, fonts: &mut Vec<String>) {
        match p {
            IrParagraph::Text(t) => {
                for r in &t.runs {
                    if let Some(s) = r.style.font_size {
                        sizes.push(s);
                    }
                    if let Some(f) = &r.style.font_name {
                        fonts.push(f.clone());
                    }
                }
            }
            IrParagraph::Table(tt) => {
                for cell in &tt.cells {
                    for inner in &cell.paragraphs {
                        visit(inner, sizes, fonts);
                    }
                }
            }
        }
    }
    for p in &ir.paragraphs {
        visit(p, &mut sizes, &mut fonts);
    }

    DocDefaults {
        run: RunStyle {
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
            strikethrough: Some(false),
            color: Some("#000000".into()),
            highlight: None,
            char_spacing: Some(0),
            char_width: Some(100),
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

/// `style` 의 키 중 *defaults 와 같은 값* 은 제외한 JSON. 결과가 빈 객체면 None.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::omitDefaults` 의 Rust 대응 (run 측). JSON 직렬화
/// 후 키-값 비교 — `RunStyle` 의 `skip_serializing_if = "Option::is_none"` 덕에 None 키는 양쪽 모두
/// 생략되어 자연스럽게 일치.
fn omit_run_style_defaults(
    style: &RunStyle,
    defaults: &RunStyle,
) -> Option<serde_json::Value> {
    let s_json = serde_json::to_value(style).ok()?;
    let d_json = serde_json::to_value(defaults).ok()?;
    let mut out = serde_json::Map::new();
    if let (serde_json::Value::Object(s_obj), serde_json::Value::Object(d_obj)) = (s_json, d_json)
    {
        for (k, v) in s_obj {
            if d_obj.get(&k) == Some(&v) {
                continue;
            }
            out.insert(k, v);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(out))
    }
}

/// `omit_run_style_defaults` 의 paragraph 판 — 동일 알고리즘.
fn omit_para_style_defaults(
    style: &ParagraphStyle,
    defaults: &ParagraphStyle,
) -> Option<serde_json::Value> {
    let s_json = serde_json::to_value(style).ok()?;
    let d_json = serde_json::to_value(defaults).ok()?;
    let mut out = serde_json::Map::new();
    if let (serde_json::Value::Object(s_obj), serde_json::Value::Object(d_obj)) = (s_json, d_json)
    {
        for (k, v) in s_obj {
            if d_obj.get(&k) == Some(&v) {
                continue;
            }
            out.insert(k, v);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(out))
    }
}

/// 단일 run 을 compact JSON 으로 변환. style 이 defaults 와 모두 같으면 `style` 키 omit.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactRun` 의 Rust 대응.
///
/// Sub-3 v2 Phase 3 — `is_first=true` 이면서 `char_offset==0` 인 경우 *해당 키 자체를 omit*.
/// 모델은 첫 run 의 char_offset 부재 시 0 으로 해석 (init.md §3).
fn compact_run(run: &IrRun, defaults: &DocDefaults, is_first: bool) -> serde_json::Value {
    let style = omit_run_style_defaults(&run.style, &defaults.run);
    let mut out = serde_json::Map::new();
    if !(is_first && run.char_offset == 0) {
        out.insert("char_offset".into(), serde_json::json!(run.char_offset));
    }
    out.insert("text".into(), serde_json::json!(run.text));
    if let Some(s) = style {
        out.insert("style".into(), s);
    }
    serde_json::Value::Object(out)
}

/// 본문 문단을 compact JSON 으로 변환. *단일 run + run 스타일 없음* 이면 `text` 직속으로 평탄화.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactText` 의 Rust 대응. 평탄화 조건은
/// "runs 가 1건이고 그 run 에 `style` 키가 없는 경우" — 모델 입력 길이를 줄이기 위한 sugar.
///
/// Sub-3 v2 Phase 3 — *무용 구조 키 omit*:
/// - `id` 는 *항상* 생략 (모델 미사용 디버그 라벨)
/// - `sec` 는 `omit_sec=true` 이면 생략. 응답 전체에서 sec 가 단일이면 `doc_meta.anchor.sec` 가
///   진실이므로 paragraph 마다의 sec 키는 중복.
/// - `type:"text"` 는 기본값 — 생략. table 만 `"table"` 로 명시 유지.
fn compact_text(
    p: &IrTextParagraph,
    defaults: &DocDefaults,
    omit_sec: bool,
) -> serde_json::Value {
    let runs: Vec<serde_json::Value> = p
        .runs
        .iter()
        .enumerate()
        .map(|(i, r)| compact_run(r, defaults, i == 0))
        .collect();
    let para_style = omit_para_style_defaults(&p.style, &defaults.paragraph);

    let mut out = serde_json::Map::new();
    // id 항상 omit (모델 미사용 디버그 라벨).
    if !omit_sec {
        out.insert("sec".into(), serde_json::json!(p.sec));
    }
    out.insert("para".into(), serde_json::json!(p.para));
    // type:"text" 는 기본값 — omit.
    // 새 페이지 시작 문단이면 page_break 마커 노출 (compact 도 outline 과 동일).
    // client get_document_outline 이 compact detail 을 쓰므로 여기서도 방출해야 한다.
    if p.page_break == Some(true) {
        out.insert("page_break".into(), serde_json::json!(true));
    }
    if let Some(cl) = &p.cell_locator {
        out.insert(
            "cell_locator".into(),
            serde_json::to_value(cl).unwrap_or_default(),
        );
    }
    if let Some(s) = para_style {
        out.insert("style".into(), s);
    }
    // 단일 run + 스타일 없음 → text 직속.
    if runs.len() == 1 && runs[0].get("style").is_none() {
        out.insert(
            "text".into(),
            runs[0]
                .get("text")
                .cloned()
                .unwrap_or(serde_json::Value::String(String::new())),
        );
    } else {
        out.insert("runs".into(), serde_json::Value::Array(runs));
    }
    serde_json::Value::Object(out)
}

/// 4면 모두 동일한 spec 이면 `all` 1키로 축약. 4면 모두 None 이면 None.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactBorder` 의 Rust 대응. 동일 판정은
/// `Option<CellBorderSpec>` 의 `PartialEq` — 4면 모두 `Some` 이고 값이 같아야 `all` 적용.
fn compact_border(border: &CellBorder) -> Option<CellBorder> {
    let sides = [&border.left, &border.right, &border.top, &border.bottom];
    let first_some = sides[0].as_ref();
    let all_same = first_some.is_some() && sides.iter().all(|s| s.as_ref() == first_some);
    if all_same {
        return Some(CellBorder {
            left: None,
            right: None,
            top: None,
            bottom: None,
            all: first_some.cloned(),
        });
    }
    if sides.iter().any(|s| s.is_some()) {
        Some(CellBorder {
            left: border.left.clone(),
            right: border.right.clone(),
            top: border.top.clone(),
            bottom: border.bottom.clone(),
            all: None,
        })
    } else {
        None
    }
}

/// 셀 한 칸을 compact JSON 으로 변환. border 만 4면 축약·내부 문단은 compact_text/compact_table 재귀.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactCell` 의 Rust 대응. 셀의 style 중 *border*
/// 만 압축 대상 — 다른 키 (bgcolor/width/height/vertical-align) 는 그대로 직렬화.
///
/// Sub-3 v2 Phase 3 — 셀 안 paragraph 도 동일하게 sec omit. 호출자가 `omit_sec` 전달.
fn compact_cell(
    cell: &IrTableCell,
    defaults: &DocDefaults,
    omit_sec: bool,
) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    out.insert("row".into(), serde_json::json!(cell.row));
    out.insert("col".into(), serde_json::json!(cell.col));
    if let Some(rs) = cell.row_span {
        out.insert("row_span".into(), serde_json::json!(rs));
    }
    if let Some(cs) = cell.col_span {
        out.insert("col_span".into(), serde_json::json!(cs));
    }
    // border 만 compact 처리한 사본 — 나머지 키는 원본 그대로.
    let mut style_clone = cell.style.clone();
    if let Some(b) = &cell.style.border {
        style_clone.border = compact_border(b);
    }
    if let Ok(s_val) = serde_json::to_value(&style_clone) {
        if let Some(obj) = s_val.as_object() {
            if !obj.is_empty() {
                out.insert("style".into(), s_val);
            }
        }
    }
    let paras: Vec<serde_json::Value> = cell
        .paragraphs
        .iter()
        .map(|p| match p {
            IrParagraph::Text(t) => compact_text(t, defaults, omit_sec),
            IrParagraph::Table(tt) => compact_table(tt, defaults, omit_sec),
        })
        .collect();
    out.insert("paragraphs".into(), serde_json::Value::Array(paras));
    serde_json::Value::Object(out)
}

/// 표 한 개를 compact JSON 으로 변환. 셀은 `compact_cell` 로 재귀.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactTable` 의 Rust 대응.
///
/// Sub-3 v2 Phase 3 — `id` 항상 omit, `sec` 는 `omit_sec=true` 일 때 omit. `type:"table"` 은
/// 기본값 ("text") 과 달라 *명시 유지* (모델이 표를 식별하는 키).
fn compact_table(
    p: &IrTableParagraph,
    defaults: &DocDefaults,
    omit_sec: bool,
) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if !omit_sec {
        out.insert("sec".into(), serde_json::json!(p.sec));
    }
    out.insert("para".into(), serde_json::json!(p.para));
    out.insert("type".into(), serde_json::json!("table"));
    out.insert("rows".into(), serde_json::json!(p.rows));
    out.insert("cols".into(), serde_json::json!(p.cols));
    out.insert(
        "cells".into(),
        serde_json::Value::Array(
            p.cells
                .iter()
                .map(|c| compact_cell(c, defaults, omit_sec))
                .collect(),
        ),
    );
    serde_json::Value::Object(out)
}

/// `IrSlice` → `CompactIrSlice` (defaults 박스 + 압축된 paragraphs).
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::compactIRSlice` 의 Rust 대응. compute_doc_defaults
/// 로 defaults 를 먼저 산정한 뒤, 각 paragraph 를 `compact_text` / `compact_table` 로 변환한다.
pub fn compact_ir_slice(ir: IrSlice) -> CompactIrSlice {
    let defaults = compute_doc_defaults(&ir);
    // Sub-3 v2 Phase 3 — sec 단일성 판단. 응답 안의 모든 paragraph 가 같은 sec 이면
    // paragraph 마다의 sec 키는 doc_meta.anchor.sec 와 중복 — omit.
    let secs: std::collections::HashSet<usize> = ir
        .paragraphs
        .iter()
        .map(|p| match p {
            IrParagraph::Text(t) => t.sec,
            IrParagraph::Table(t) => t.sec,
        })
        .collect();
    let omit_sec = secs.len() <= 1;

    let paragraphs: Vec<serde_json::Value> = ir
        .paragraphs
        .iter()
        .map(|p| match p {
            IrParagraph::Text(t) => compact_text(t, &defaults, omit_sec),
            IrParagraph::Table(tt) => compact_table(tt, &defaults, omit_sec),
        })
        .collect();
    CompactIrSlice {
        doc_meta: ir.doc_meta,
        paragraphs,
        defaults,
    }
}

/// `build_ir_slice` + `compact_ir_slice` 결합 — endpoint 가 호출할 진입 함수.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildCompactIRSlice` 의 Rust 대응.
pub fn build_compact_ir_slice(core: &DocumentCore, opts: &BuildOptions) -> CompactIrSlice {
    compact_ir_slice(build_ir_slice(core, opts))
}

// ─── outline / structure 단계 빌더 ──────────────────────────────────

/// paragraph 별 본문 텍스트의 *첫 max_chars 글자* 추출 + 잘림 여부 동반 반환.
/// `max_chars == None` 이면 잘리지 않음. 빈 텍스트는 `("", false)`.
fn truncate_text(text: &str, max_chars: Option<u32>) -> (String, bool) {
    match max_chars {
        Some(n) => {
            let n = n as usize;
            let chars: Vec<char> = text.chars().collect();
            if chars.len() <= n {
                (text.to_string(), false)
            } else {
                (chars[..n].iter().collect(), true)
            }
        }
        None => (text.to_string(), false),
    }
}

/// outline 단계 진입 — paragraph 별 *첫 N 글자 + style.align* 만 박는다.
///
/// 표 paragraph 는 `type:"table"` + `rows`/`cols` 만. 셀 안 paragraph 빼기.
/// `doc_meta` 는 compact 와 동일 (edit_session_id / page / total_pages / anchor) — 모델 입장에서
/// 위치 좌표는 유지하되 본문은 *훑어보는 정도* 만 받음.
pub fn build_outline_slice(core: &DocumentCore, opts: &BuildOptions) -> serde_json::Value {
    let ir = build_ir_slice(core, opts);
    let paragraphs: Vec<serde_json::Value> = ir
        .paragraphs
        .iter()
        .map(|p| build_outline_paragraph(p, opts.max_text_chars))
        .collect();
    serde_json::json!({
        "doc_meta": ir.doc_meta,
        "paragraphs": paragraphs,
    })
}

fn build_outline_paragraph(
    p: &IrParagraph,
    max_chars: Option<u32>,
) -> serde_json::Value {
    match p {
        IrParagraph::Text(t) => {
            let full_text: String = t.runs.iter().map(|r| r.text.clone()).collect();
            let (text, truncated) = truncate_text(&full_text, max_chars);
            let mut out = serde_json::Map::new();
            out.insert("para".into(), serde_json::json!(t.para));
            out.insert("text".into(), serde_json::json!(text));
            if truncated {
                out.insert("text_truncated".into(), serde_json::json!(true));
            }
            // 새 페이지 시작 문단이면 마커 노출 — 모델이 outline 만으로 page break 위치 파악.
            if t.page_break == Some(true) {
                out.insert("page_break".into(), serde_json::json!(true));
            }
            if let Some(align) = &t.style.align {
                out.insert("style".into(), serde_json::json!({ "align": align }));
            }
            if let Some(cl) = &t.cell_locator {
                out.insert(
                    "cell_locator".into(),
                    serde_json::to_value(cl).unwrap_or_default(),
                );
            }
            serde_json::Value::Object(out)
        }
        IrParagraph::Table(tt) => {
            serde_json::json!({
                "para": tt.para,
                "type": "table",
                "rows": tt.rows,
                "cols": tt.cols,
            })
        }
    }
}

/// structure 단계 진입 — paragraph idx + type + 글자 수만. 본문 텍스트 없음.
///
/// 표 paragraph 는 rows/cols + cells 의 row/col/row_span/col_span (셀 안 paragraph 없음).
/// `include_tables == Count` 면 cells 자리 자체 omit.
pub fn build_structure_slice(core: &DocumentCore, opts: &BuildOptions) -> serde_json::Value {
    let ir = build_ir_slice(core, opts);
    let paragraphs: Vec<serde_json::Value> = ir
        .paragraphs
        .iter()
        .map(|p| build_structure_paragraph(p, opts.include_tables))
        .collect();
    serde_json::json!({
        "doc_meta": ir.doc_meta,
        "paragraphs": paragraphs,
    })
}

fn build_structure_paragraph(
    p: &IrParagraph,
    tables: TableLevel,
) -> serde_json::Value {
    match p {
        IrParagraph::Text(t) => {
            let char_count: usize = t.runs.iter().map(|r| r.text.chars().count()).sum();
            let mut out = serde_json::Map::new();
            out.insert("para".into(), serde_json::json!(t.para));
            out.insert("char_count".into(), serde_json::json!(char_count));
            if let Some(cl) = &t.cell_locator {
                out.insert(
                    "cell_locator".into(),
                    serde_json::to_value(cl).unwrap_or_default(),
                );
            }
            serde_json::Value::Object(out)
        }
        IrParagraph::Table(tt) => {
            let mut out = serde_json::Map::new();
            out.insert("para".into(), serde_json::json!(tt.para));
            out.insert("type".into(), serde_json::json!("table"));
            out.insert("rows".into(), serde_json::json!(tt.rows));
            out.insert("cols".into(), serde_json::json!(tt.cols));
            if !matches!(tables, TableLevel::Count) {
                let cells: Vec<serde_json::Value> = tt
                    .cells
                    .iter()
                    .map(|c| {
                        let mut m = serde_json::Map::new();
                        m.insert("row".into(), serde_json::json!(c.row));
                        m.insert("col".into(), serde_json::json!(c.col));
                        if let Some(rs) = c.row_span {
                            m.insert("row_span".into(), serde_json::json!(rs));
                        }
                        if let Some(cs) = c.col_span {
                            m.insert("col_span".into(), serde_json::json!(cs));
                        }
                        serde_json::Value::Object(m)
                    })
                    .collect();
                out.insert("cells".into(), serde_json::Value::Array(cells));
            }
            serde_json::Value::Object(out)
        }
    }
}

// ─── include_style / include_tables 후처리 필터 ────────────────────

/// compact 응답의 paragraph 배열에 style 강도 필터 적용.
///
/// `Full` 이면 무변경. `Essential` 이면 RunStyle / ParagraphStyle 화이트리스트 외 키 제거.
/// `None` 이면 paragraph·run·cell 의 `style` 키 자체 제거.
pub fn apply_style_filter(paragraphs: &mut [serde_json::Value], level: StyleLevel) {
    if matches!(level, StyleLevel::Full) {
        return;
    }
    for p in paragraphs.iter_mut() {
        filter_style_in_paragraph(p, level);
    }
}

fn filter_style_in_paragraph(p: &mut serde_json::Value, level: StyleLevel) {
    let Some(obj) = p.as_object_mut() else {
        return;
    };

    // paragraph 자체의 style
    match level {
        StyleLevel::None => {
            obj.remove("style");
        }
        StyleLevel::Essential => {
            if let Some(serde_json::Value::Object(map)) = obj.get_mut("style") {
                map.retain(|k, _| ESSENTIAL_PARA_STYLE_KEYS.contains(&k.as_str()));
            }
            if let Some(serde_json::Value::Object(map)) = obj.get("style") {
                if map.is_empty() {
                    obj.remove("style");
                }
            }
        }
        StyleLevel::Full => {}
    }

    // runs 안 style
    if let Some(serde_json::Value::Array(runs)) = obj.get_mut("runs") {
        for run in runs.iter_mut() {
            let Some(rmap) = run.as_object_mut() else {
                continue;
            };
            match level {
                StyleLevel::None => {
                    rmap.remove("style");
                }
                StyleLevel::Essential => {
                    if let Some(serde_json::Value::Object(map)) = rmap.get_mut("style") {
                        map.retain(|k, _| ESSENTIAL_RUN_STYLE_KEYS.contains(&k.as_str()));
                    }
                    if let Some(serde_json::Value::Object(map)) = rmap.get("style") {
                        if map.is_empty() {
                            rmap.remove("style");
                        }
                    }
                }
                StyleLevel::Full => {}
            }
        }
    }

    // 표 셀 — cells[].style 및 cells[].paragraphs[] 재귀
    if let Some(serde_json::Value::Array(cells)) = obj.get_mut("cells") {
        for cell in cells.iter_mut() {
            let Some(cmap) = cell.as_object_mut() else {
                continue;
            };
            if matches!(level, StyleLevel::None) {
                cmap.remove("style");
            }
            // 셀의 style 은 CellStyle (bgcolor/width/height/border/vertical-align) — essential 단계는
            // 자체적으로 의미 있는 값이라 화이트리스트 가지치기 대신 *유지* (border 색 등 시각 식별 키).
            // Full / Essential 둘 다 동일 처리. None 만 제거.
            if let Some(serde_json::Value::Array(inner)) = cmap.get_mut("paragraphs") {
                for inner_p in inner.iter_mut() {
                    filter_style_in_paragraph(inner_p, level);
                }
            }
        }
    }
}

/// compact 응답의 paragraph 배열에 표 정보 강도 필터 적용.
///
/// `Full` 이면 무변경. `Structure` 이면 셀의 `paragraphs` 키 제거 (rows/cols/style/span 만 유지).
/// `Count` 이면 표 paragraph 의 `cells` 키 자체 제거 (type/rows/cols 만 남김).
pub fn apply_table_filter(paragraphs: &mut [serde_json::Value], level: TableLevel) {
    if matches!(level, TableLevel::Full) {
        return;
    }
    for p in paragraphs.iter_mut() {
        let Some(obj) = p.as_object_mut() else {
            continue;
        };
        // 표 paragraph 인지 — `cells` 키 보유 또는 `type:"table"`.
        let is_table = obj.contains_key("cells")
            || matches!(obj.get("type"), Some(serde_json::Value::String(s)) if s == "table");
        if !is_table {
            continue;
        }
        match level {
            TableLevel::Count => {
                obj.remove("cells");
            }
            TableLevel::Structure => {
                if let Some(serde_json::Value::Array(cells)) = obj.get_mut("cells") {
                    for cell in cells.iter_mut() {
                        if let Some(cmap) = cell.as_object_mut() {
                            cmap.remove("paragraphs");
                        }
                    }
                }
            }
            TableLevel::Full => {}
        }
    }
}

// ─── Sub-4: patch diff 캡처 ─────────────────────────────────────────

/// 편집 연산 적용 전후의 IR 스냅샷.
///
/// `op` 는 EditOperation 의 `op` 태그(예: "replace_cell_runs") — 모델이 어떤 액션
/// 결과인지 응답에서 바로 인식하도록 함께 싣는다.
/// `before` 와 `after` 는 *영향받은 데이터만* 잘라낸 패치 타겟이다 — 셀 편집이면
/// 그 셀 한 칸의 JSON, 그 외에는 영향 paragraph 슬라이스. doc_meta·defaults 는
/// `location` 에 이미 좌표가 들어 있어 제거 — 응답 크기 부담 최소화.
/// `summary` 는 변경 여부와 길이 통계를 미리 계산해 모델이 한눈에 확인할 수 있게 한다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchDiff {
    pub op: String,
    pub location: PatchLocation,
    pub before: PatchTarget,
    pub after: PatchTarget,
    pub summary: PatchSummary,
}

/// 패치 타겟 — 응답 크기 절감을 위해 영향받은 *최소 단위* 만 직렬화.
///
/// - `Cell { cell }` — 표 셀 편집 (SetCellStyle / ReplaceCellRuns /
///   InsertTextInCell / DeleteRangeInCell). 표 paragraph 전체가 아니라 *해당
///   셀 한 칸* 의 compact JSON 만 싣는다 (row, col, style, paragraphs).
/// - `Paragraphs { paragraphs }` — 본문 paragraph 편집 또는 표 추가/제거
///   (InsertTable, DeleteElement::Table, MergeCells 등 cell_idx 가 단일이지
///   않은 경우 포함).
///
/// 직렬화 시 untagged — `cell` 키 vs `paragraphs` 키로 모델이 구분.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PatchTarget {
    Cell { cell: serde_json::Value },
    Paragraphs { paragraphs: Vec<serde_json::Value> },
}

/// 변경 위치 좌표.
///
/// 본문 paragraph 범위는 0-based 반열린 `[start, end)`. 표 셀 단위 편집이면 `cell`
/// 에 (table_para, row, col, cell_idx?, cell_para?) 가 채워진다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchLocation {
    pub section: usize,
    pub para_start_before: usize,
    pub para_end_before: usize,
    pub para_start_after: usize,
    pub para_end_after: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellFocus>,
}

/// before/after 변화 요약. 모델이 응답 JSON 만 보고 *적용 여부* 와 *변화 크기* 를
/// 즉시 알 수 있도록 미리 계산해 둔다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchSummary {
    /// before != after (compact IR JSON 직렬화 비교). false 면 *no-op* — 좌표/payload 가
    /// 실제 데이터를 바꾸지 못한 경우 (예: 같은 runs 로 교체, 빈 범위 삭제).
    pub changed: bool,
    pub before_para_count: usize,
    pub after_para_count: usize,
    pub before_text_len: usize,
    pub after_text_len: usize,
    /// [Sub-7] changed=false 일 때 모델이 *놓치지 않게* 채워 보내는 경고 문자열.
    /// payload 의 style 키가 schema (Partial*Style) 와 일치하는지, native 키 매핑이
    /// 깨지지 않았는지 확인하라는 hint. changed=true 면 None — silent drop 사고
    /// 예방 패턴.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_change_warning: Option<String>,
}

/// 단일 paragraph JSON 의 텍스트 길이 (셀 paragraph 도 단순 문단으로 취급).
///
/// Sub-3 v2 의 compact 형식 기준 — 표 paragraph 는 `cells` 키를 직접 갖고
/// 각 셀이 `paragraphs` 배열을 갖는다. 표 키가 따로 wrapper 없이 펼쳐진 형태.
fn paragraph_text_len_recursive(p: &serde_json::Value) -> usize {
    // 표 paragraph — `cells` 키 있음.
    if let Some(cells) = p.get("cells").and_then(|c| c.as_array()) {
        return cells
            .iter()
            .map(|c| {
                c.get("paragraphs")
                    .and_then(|ps| ps.as_array())
                    .map(|ps| ps.iter().map(paragraph_text_len_recursive).sum::<usize>())
                    .unwrap_or(0)
            })
            .sum();
    }
    // 본문 paragraph — `runs` 배열 또는 압축 시 `text` 단일 키.
    if let Some(runs) = p.get("runs").and_then(|r| r.as_array()) {
        return runs
            .iter()
            .map(|r| {
                r.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.chars().count())
                    .unwrap_or(0)
            })
            .sum();
    }
    // Sub-3 v2 의 단일 run 축약 형식: paragraph 가 직접 `text` 키를 가짐.
    if let Some(s) = p.get("text").and_then(|t| t.as_str()) {
        return s.chars().count();
    }
    0
}

fn target_para_count(t: &PatchTarget) -> usize {
    match t {
        // 단일 셀은 항상 1 — 셀 안의 paragraph 수 변화는 cell.paragraphs 비교로 확인.
        PatchTarget::Cell { .. } => 1,
        PatchTarget::Paragraphs { paragraphs } => paragraphs.len(),
    }
}

fn target_text_len(t: &PatchTarget) -> usize {
    match t {
        PatchTarget::Cell { cell } => cell
            .get("paragraphs")
            .and_then(|ps| ps.as_array())
            .map(|ps| ps.iter().map(paragraph_text_len_recursive).sum::<usize>())
            .unwrap_or(0),
        PatchTarget::Paragraphs { paragraphs } => {
            paragraphs.iter().map(paragraph_text_len_recursive).sum()
        }
    }
}

/// 표 paragraph 의 `cells` 배열에서 선형 인덱스 `cell_idx` 위치의 셀 JSON 만 추출.
///
/// `slice.paragraphs` 안에서 첫 표 paragraph (cells 키를 가진 것) 를 찾아 거기서
/// `cells[cell_idx]` 를 복사해 반환. 셀이 없거나 인덱스 범위 밖이면 None.
///
/// 셀 한 칸만 응답에 싣기 위한 헬퍼 — 표 전체 IR 두 번 (before+after) 을 보내는
/// 대신 셀 단위 JSON 으로 응답 크기를 표 셀 수에 비례하게 압축.
fn extract_compact_cell(
    slice: &CompactIrSlice,
    cell_idx: Option<usize>,
) -> Option<serde_json::Value> {
    let idx = cell_idx?;
    for para in &slice.paragraphs {
        if let Some(cells) = para.get("cells").and_then(|c| c.as_array()) {
            return cells.get(idx).cloned();
        }
    }
    None
}

/// CompactIrSlice + AffectedRange → PatchTarget 변환.
///
/// cell focus + 채워진 cell_idx 가 있으면 셀 한 칸만 추출. 추출 실패하거나 cell focus
/// 없으면 paragraphs 슬라이스 그대로.
fn slice_to_target(slice: CompactIrSlice, range: &AffectedRange) -> PatchTarget {
    if let Some(focus) = &range.cell {
        if let Some(cell_json) = extract_compact_cell(&slice, focus.cell_idx) {
            return PatchTarget::Cell { cell: cell_json };
        }
    }
    PatchTarget::Paragraphs {
        paragraphs: slice.paragraphs,
    }
}

/// 편집 적용 전 패치 타겟 캡처. 셀 편집이면 셀 한 칸, 그 외엔 paragraphs 슬라이스.
pub fn capture_before_target(core: &DocumentCore, range: &AffectedRange) -> PatchTarget {
    let slice = build_compact_ir_slice(
        core,
        &BuildOptions {
            sec: range.section,
            para_start: range.before.start,
            para_end: Some(range.before.end),
            edit_session_id: None,
            page: None,
            page_override_range: None,
            total_pages_override: None,
            ..Default::default()
        },
    );
    slice_to_target(slice, range)
}

/// 편집 적용 후 패치 타겟 캡처. 셀 편집이면 셀 한 칸, 그 외엔 paragraphs 슬라이스.
pub fn capture_after_target(core: &DocumentCore, range: &AffectedRange) -> PatchTarget {
    let slice = build_compact_ir_slice(
        core,
        &BuildOptions {
            sec: range.section,
            para_start: range.after.start,
            para_end: Some(range.after.end),
            edit_session_id: None,
            page: None,
            page_override_range: None,
            total_pages_override: None,
            ..Default::default()
        },
    );
    slice_to_target(slice, range)
}

/// before/after PatchTarget 과 affected range 를 결합해 PatchDiff 를 구성한다.
///
/// `op_tag` 는 EditOperation 의 `op` 태그 문자열 — 모델 응답에서 어떤 액션인지 식별
/// 용도로 함께 싣는다.
pub fn build_patch_diff(
    op_tag: &str,
    range: &AffectedRange,
    before: PatchTarget,
    after: PatchTarget,
) -> PatchDiff {
    let before_para_count = target_para_count(&before);
    let after_para_count = target_para_count(&after);
    let before_text_len = target_text_len(&before);
    let after_text_len = target_text_len(&after);

    // changed 판정 — PatchTarget JSON 직렬 비교. 셀 단위든 paragraphs 단위든
    // 같은 타겟 형식끼리 비교 (둘 다 Cell or 둘 다 Paragraphs).
    let changed = serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)
        != serde_json::to_value(&after).unwrap_or(serde_json::Value::Null);

    // [Sub-7] changed=false 면 silent drop 사고일 가능성 — 모델/클라가 응답을 보고
    // 즉시 인지하도록 경고 문자열 채움.
    let no_change_warning = if !changed {
        Some(
            "before/after 동일 — payload 의 style 키가 schema (Partial*Style) 와 일치하는지 확인. \
             오타·미지원 키는 deny_unknown_fields 로 400 반환되어야 하지만, 값이 같거나 \
             좌표가 유효 범위를 벗어나도 no-op 가 될 수 있음."
                .to_string(),
        )
    } else {
        None
    };

    PatchDiff {
        op: op_tag.to_string(),
        location: PatchLocation {
            section: range.section,
            para_start_before: range.before.start,
            para_end_before: range.before.end,
            para_start_after: range.after.start,
            para_end_after: range.after.end,
            cell: range.cell.clone(),
        },
        before,
        after,
        summary: PatchSummary {
            changed,
            before_para_count,
            after_para_count,
            before_text_len,
            after_text_len,
            no_change_warning,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::document_core::ParaRange;
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
    fn run_style_from_char_shape_bold_size() {
        use rhwp::model::style::{CharShape, UnderlineType};
        use rhwp::renderer::style_resolver::ResolvedCharStyle;

        // resolved: bold + 한국어 함초롬돋움.
        let mut resolved = ResolvedCharStyle::default();
        resolved.bold = true;
        resolved.italic = false;
        resolved.font_family = "함초롬돋움".into();
        resolved.font_families = vec![
            "함초롬돋움".into(),
            "Calibri".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        resolved.text_color = 0x000000FF; // 빨강 #FF0000 (BGR 순서로 R=0xFF 가 low byte)
        resolved.shade_color = 0x00FFFFFF; // sentinel — highlight 없음
        resolved.underline = UnderlineType::None;
        resolved.strikethrough = false;
        resolved.subscript = false;
        resolved.superscript = false;

        // raw: base_size=2400 (= 24pt), 한국어 자간 0, 장평 100.
        let mut raw = CharShape::default();
        raw.base_size = 2400;
        raw.ratios = [100, 100, 100, 100, 100, 100, 100];
        raw.spacings = [0, 0, 0, 0, 0, 0, 0];

        let run_style = char_shape_to_run_style(&resolved, &raw, 0);

        assert_eq!(run_style.bold, Some(true));
        assert_eq!(run_style.italic, Some(false));
        assert_eq!(run_style.underline, Some(false));
        assert_eq!(run_style.strikethrough, Some(false));
        // 24pt = 2400 / 100.
        assert_eq!(run_style.font_size, Some(24.0));
        assert_eq!(run_style.font_name.as_deref(), Some("함초롬돋움"));
        assert_eq!(run_style.color.as_deref(), Some("#FF0000"));
        // shade_color 가 sentinel 이면 키 자체 미설정.
        assert!(run_style.highlight.is_none());
        assert_eq!(run_style.char_spacing, Some(0));
        assert_eq!(run_style.char_width, Some(100));
        assert_eq!(run_style.vertical_align.as_deref(), Some("baseline"));
    }

    #[test]
    fn run_style_underline_subscript_highlight() {
        use rhwp::model::style::{CharShape, UnderlineType};
        use rhwp::renderer::style_resolver::ResolvedCharStyle;

        // 밑줄 + 아래첨자 + 형광펜(노랑).
        let mut resolved = ResolvedCharStyle::default();
        resolved.font_family = "맑은 고딕".into();
        resolved.font_families = vec!["맑은 고딕".into(); 7];
        resolved.underline = UnderlineType::Bottom;
        resolved.subscript = true;
        resolved.shade_color = 0x0007C1FF; // #FFC107 (BGR 표기로 B=07 G=C1 R=FF)
        resolved.text_color = 0;

        let mut raw = CharShape::default();
        raw.base_size = 1100; // 11pt
        raw.ratios = [100; 7];
        raw.spacings = [0; 7];

        let run_style = char_shape_to_run_style(&resolved, &raw, 0);

        assert_eq!(run_style.underline, Some(true));
        assert_eq!(run_style.vertical_align.as_deref(), Some("sub"));
        assert_eq!(run_style.highlight.as_deref(), Some("#FFC107"));
        assert_eq!(run_style.font_size, Some(11.0));
    }

    #[test]
    fn run_style_lang_specific_font() {
        use rhwp::model::style::CharShape;
        use rhwp::renderer::style_resolver::ResolvedCharStyle;

        // 영어(1) 카테고리 폰트가 한국어와 다른 경우, lang_idx=1 로 호출하면 영어 폰트가 들어와야.
        let mut resolved = ResolvedCharStyle::default();
        resolved.font_family = "함초롬돋움".into();
        resolved.font_families = vec![
            "함초롬돋움".into(),
            "Calibri".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];

        let mut raw = CharShape::default();
        raw.base_size = 1000;
        raw.ratios = [100; 7];
        raw.spacings = [0; 7];

        let kor = char_shape_to_run_style(&resolved, &raw, 0);
        let eng = char_shape_to_run_style(&resolved, &raw, 1);

        assert_eq!(kor.font_name.as_deref(), Some("함초롬돋움"));
        assert_eq!(eng.font_name.as_deref(), Some("Calibri"));
    }

    #[test]
    fn para_style_align_center_percent_160() {
        use rhwp::model::style::{Alignment, LineSpacingType, ParaShape};
        let ps = ParaShape {
            alignment: Alignment::Center,
            indent: 0,
            line_spacing: 160,
            line_spacing_type: LineSpacingType::Percent,
            ..Default::default()
        };
        let s = para_shape_to_para_style(&ps);
        assert_eq!(s.align.as_deref(), Some("center"));
        assert_eq!(s.indent, Some(0));
        assert_eq!(s.line_height, Some(160));
    }

    #[test]
    fn para_style_line_height_omitted_when_not_percent() {
        use rhwp::model::style::{LineSpacingType, ParaShape};
        let ps = ParaShape {
            line_spacing: 1000,
            line_spacing_type: LineSpacingType::Fixed,
            ..Default::default()
        };
        let s = para_shape_to_para_style(&ps);
        // Fixed/SpaceOnly/Minimum 은 모델 입력 단위가 통일되지 않아 omit.
        assert!(s.line_height.is_none());

        let ps2 = ParaShape {
            line_spacing: 100,
            line_spacing_type: LineSpacingType::Minimum,
            ..Default::default()
        };
        let s2 = para_shape_to_para_style(&ps2);
        assert!(s2.line_height.is_none());
    }

    #[test]
    fn para_style_indent_negative_outdent() {
        use rhwp::model::style::{Alignment, LineSpacingType, ParaShape};
        // 내어쓰기(음수 indent) — i32 로 그대로 전달.
        let ps = ParaShape {
            alignment: Alignment::Left,
            indent: -1500,
            line_spacing: 200,
            line_spacing_type: LineSpacingType::Percent,
            ..Default::default()
        };
        let s = para_shape_to_para_style(&ps);
        assert_eq!(s.align.as_deref(), Some("left"));
        assert_eq!(s.indent, Some(-1500));
        assert_eq!(s.line_height, Some(200));
    }

    #[test]
    fn cell_style_with_bgcolor() {
        use rhwp::model::table::{Cell, VerticalAlign};
        use rhwp::renderer::style_resolver::ResolvedBorderStyle;

        let cell = Cell {
            width: 1000,
            height: 500,
            vertical_align: VerticalAlign::Center,
            ..Default::default()
        };

        // 배경색 #FFC107 (BGR 표기로 R=0xFF, G=0xC1, B=0x07 → 0x0007C1FF).
        // 4면 테두리는 BorderLine::default() = Solid + width 0 — 기본 "보임"
        // 으로 풀이되지만 본 테스트는 fill_color 만 검증.
        let mut bs = ResolvedBorderStyle::default();
        bs.fill_color = Some(0x0007C1FF);

        let s = cell_to_cell_style(&cell, Some(&bs));
        assert_eq!(s.bgcolor.as_deref(), Some("#FFC107"));
        assert_eq!(s.width, Some(1000));
        assert_eq!(s.height, Some(500));
        assert_eq!(s.vertical_align.as_deref(), Some("middle"));
    }

    #[test]
    fn cell_style_no_border_style() {
        // border_style=None (border_fill_id=0 인 셀) — border 키, bgcolor 키 모두 omit.
        use rhwp::model::table::{Cell, VerticalAlign};
        let cell = Cell {
            width: 800,
            height: 300,
            vertical_align: VerticalAlign::Top,
            ..Default::default()
        };
        let s = cell_to_cell_style(&cell, None);
        assert!(s.bgcolor.is_none());
        assert!(s.border.is_none());
        assert_eq!(s.width, Some(800));
        assert_eq!(s.height, Some(300));
        assert_eq!(s.vertical_align.as_deref(), Some("top"));
    }

    #[test]
    fn cell_style_border_only_some_sides() {
        // 좌·하 두 면만 테두리, 우·상 두 면은 None.
        use rhwp::model::style::{BorderLine, BorderLineType};
        use rhwp::model::table::{Cell, VerticalAlign};
        use rhwp::renderer::style_resolver::ResolvedBorderStyle;

        let cell = Cell {
            width: 500,
            height: 500,
            vertical_align: VerticalAlign::Bottom,
            ..Default::default()
        };

        let mut bs = ResolvedBorderStyle::default();
        bs.fill_color = None;
        // borders 배열 순서: 0=좌, 1=우, 2=상, 3=하.
        bs.borders[0] = BorderLine {
            line_type: BorderLineType::Solid,
            width: 1,
            color: 0x000000FF, // 빨강
        };
        bs.borders[1] = BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        };
        bs.borders[2] = BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        };
        bs.borders[3] = BorderLine {
            line_type: BorderLineType::Dash,
            width: 2,
            color: 0,
        };

        let s = cell_to_cell_style(&cell, Some(&bs));
        let border = s.border.expect("일부 면만 설정되어도 border 키는 존재");
        assert!(border.left.is_some());
        assert!(border.right.is_none());
        assert!(border.top.is_none());
        assert!(border.bottom.is_some());
        // 좌측 면 — type/width/color 확인.
        let left = border.left.unwrap();
        assert_eq!(left.border_type, Some(BorderLineType::Solid as u8));
        assert_eq!(left.width, Some(1));
        assert_eq!(left.color.as_deref(), Some("#FF0000"));
        // 하단 면 — Dash 종류 확인.
        let bottom = border.bottom.unwrap();
        assert_eq!(bottom.border_type, Some(BorderLineType::Dash as u8));
        assert_eq!(bottom.width, Some(2));
        // 4면 모두 BorderLineType::None 으로 만들면 border 키 자체 omit 되는지 확인 (다른 셀).
        let mut bs_all_none = ResolvedBorderStyle::default();
        for i in 0..4 {
            bs_all_none.borders[i].line_type = BorderLineType::None;
        }
        let s2 = cell_to_cell_style(&cell, Some(&bs_all_none));
        assert!(s2.border.is_none(), "4면 모두 None 이면 border 키 omit");
    }

    #[test]
    fn collect_runs_single_style() {
        let s = RunStyle::default();
        let runs = collect_runs("ABC", 3, |_| s.clone());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "ABC");
        assert_eq!(runs[0].length, 3);
        assert_eq!(runs[0].char_offset, 0);
    }

    #[test]
    fn collect_runs_two_styles() {
        let bold = RunStyle {
            bold: Some(true),
            ..Default::default()
        };
        let plain = RunStyle::default();
        let runs = collect_runs(
            "ABCDE",
            5,
            |off| if off < 2 { bold.clone() } else { plain.clone() },
        );
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "AB");
        assert_eq!(runs[0].char_offset, 0);
        assert_eq!(runs[0].length, 2);
        assert_eq!(runs[0].style.bold, Some(true));
        assert_eq!(runs[1].text, "CDE");
        assert_eq!(runs[1].char_offset, 2);
        assert_eq!(runs[1].length, 3);
        assert!(runs[1].style.bold.is_none() || runs[1].style.bold == Some(false));
    }

    #[test]
    fn collect_runs_empty_paragraph() {
        let runs = collect_runs("", 0, |_| RunStyle::default());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].length, 0);
        assert_eq!(runs[0].char_offset, 0);
        assert!(runs[0].text.is_empty());
    }

    #[test]
    fn collect_runs_empty_paragraph_takes_style_from_callback() {
        // Sub-4 v3 — 빈 paragraph 라도 style_at(0) 호출 결과를 placeholder run 의
        // style 로 사용해야 한다. 빈 셀에 묶인 char_shape (빨간색 등) 가 응답에 노출되도록.
        let red_style = RunStyle {
            color: Some("#FF0000".into()),
            bold: Some(true),
            ..Default::default()
        };
        let captured_red = red_style.clone();
        let runs = collect_runs("", 0, move |off| {
            assert_eq!(off, 0, "빈 paragraph 에 대해 style_at 은 offset 0 으로 호출");
            captured_red.clone()
        });
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].length, 0);
        assert_eq!(runs[0].style, red_style, "callback 이 반환한 style 이 placeholder 에 박혀야");
    }

    #[test]
    fn build_text_paragraph_blank_returns_default_style() {
        // `samples/hwpx/blank_hwpx.hwpx` 는 워크스페이스 루트의 빈 문서.
        // 첫 섹션의 첫 문단을 빌드해 보면 텍스트가 비어있어 collect_runs 가 len=0 1건 반환.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load blank");

        let para = build_text_paragraph(&core, 0, 0);
        assert_eq!(para.kind, "text");
        assert_eq!(para.id, "p_0_0");
        assert_eq!(para.sec, 0);
        assert_eq!(para.para, 0);
        // 빈 문단이라도 collect_runs 는 1건의 placeholder run 을 반환.
        assert!(!para.runs.is_empty());
        // 빈 문단이면 첫 run 은 length=0.
        if para.runs.len() == 1 {
            assert_eq!(para.runs[0].length, 0);
        }
    }

    #[test]
    fn build_ir_slice_blank_doc() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("test".into()),
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        assert_eq!(slice.doc_meta.anchor.sec, 0);
        assert_eq!(slice.doc_meta.edit_session_id, "test");
        // 빈 문서라도 섹션 0 에는 최소 1개 문단이 존재 — paragraphs 가 비어있지 않아야.
        assert!(!slice.paragraphs.is_empty());
        // anchor.para_end 는 섹션의 실제 문단 수를 넘지 않음.
        let total = core.document().sections[0].paragraphs.len();
        assert_eq!(slice.doc_meta.anchor.para_end, total);
    }

    #[test]
    fn build_ir_slice_auto_edit_session_id() {
        // edit_session_id 가 None 이면 "ed_<ms>" 형식의 자동 ID 생성.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: None,
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        assert!(slice.doc_meta.edit_session_id.starts_with("ed_"));
    }

    #[test]
    fn build_cell_paragraph_with_cell_locator() {
        // blank hwpx 에 표가 없을 경우 → cell_para_ref = None → 함수는 *기본값* 으로 채운 entry 반환.
        // 그래도 cell_locator 와 id 형식 invariant 는 검증 가능. 본격 e2e 는 Phase 6.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        let cell = build_cell_paragraph(&core, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(cell.kind, "text");
        // 셀 안 문단은 본문 문단과 구별 위해 para = -1.
        assert_eq!(cell.para, -1);
        // id 형식: p_{sec}_{table_para}_c{control_idx}_{cell_idx}_{cell_para}.
        assert_eq!(cell.id, "p_0_0_c0_0_0");
        // cell_locator 가 채워져 있어야 함.
        let locator = cell.cell_locator.as_ref().expect("cell_locator");
        assert_eq!(locator.table_para, 0);
        assert_eq!(locator.row, 0);
        assert_eq!(locator.col, 0);
        assert_eq!(locator.cell_para, 0);
        // 표가 없으므로 runs 는 placeholder 1건 (length=0).
        assert_eq!(cell.runs.len(), 1);
        assert_eq!(cell.runs[0].length, 0);
    }

    #[test]
    fn build_cell_paragraph_id_format_with_row_col() {
        // id 와 cell_locator 의 row/col 이 호출자가 전달한 값 그대로 들어가는지 검증.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        let cell = build_cell_paragraph(&core, 0, 2, 1, 3, 0, 5, 7);
        // id 는 cell_idx 까지 포함 — row/col 자체는 id 에 들어가지 않음.
        assert_eq!(cell.id, "p_0_2_c1_3_0");
        let locator = cell.cell_locator.as_ref().expect("cell_locator");
        assert_eq!(locator.table_para, 2);
        assert_eq!(locator.row, 5);
        assert_eq!(locator.col, 7);
        assert_eq!(locator.cell_para, 0);
    }

    #[test]
    fn build_table_paragraph_none_when_no_table() {
        // 표 control 이 없는 본문 문단 — build_table_paragraph 는 None 반환.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let result = build_table_paragraph(&core, 0, 0, 0);
        assert!(result.is_none(), "표가 없으면 None");
    }

    #[test]
    fn try_build_cell_none_when_no_table() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let result = try_build_cell(&core, 0, 0, 0, 0);
        assert!(result.is_none(), "표가 없으면 try_build_cell 도 None");
    }

    #[test]
    fn build_table_paragraph_with_mock_table() {
        // blank hwpx 로드 후 본문 첫 문단에 *직접* Control::Table 를 끼워넣어 positive path 검증.
        // 본체 mutator (insert_table_native 등) 의존을 피하고 *IR 빌드 알고리즘만* 검증.
        use rhwp::model::control::Control;
        use rhwp::model::table::{Cell, Table};

        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let mut core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        // 2x2 표 mock — 4 cells, row/col 0..2 직접 채움.
        let mut table = Table {
            row_count: 2,
            col_count: 2,
            ..Default::default()
        };
        for r in 0..2u16 {
            for c in 0..2u16 {
                table.cells.push(Cell {
                    col: c,
                    row: r,
                    col_span: 1,
                    row_span: 1,
                    width: 1000,
                    height: 500,
                    border_fill_id: 0, // 테두리·배경 미설정 — border_style = None 경로 검증
                    paragraphs: vec![rhwp::model::paragraph::Paragraph::default()],
                    ..Default::default()
                });
            }
        }
        // 병합 셀이 없으니 row_span = 1 — IR 측에서 키 자체 omit.
        // 첫 문단의 controls 에 표 끼워넣기.
        // blank hwpx 의 첫 문단에는 이미 섹션·단 정의 등 *비-표 control* 이 있다.
        // 표는 controls 끝에 push 되므로 *제일 마지막 인덱스* 가 표.
        core.document_mut().sections[0].paragraphs[0]
            .controls
            .push(Control::Table(Box::new(table)));
        let ctrl_idx = core.document().sections[0].paragraphs[0].controls.len() - 1;

        let result =
            build_table_paragraph(&core, 0, 0, ctrl_idx).expect("표 있음 → Some");
        assert_eq!(result.kind, "table");
        assert_eq!(result.id, "p_0_0");
        assert_eq!(result.sec, 0);
        assert_eq!(result.para, 0);
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 2);
        assert_eq!(result.cells.len(), 4);

        // 첫 셀: row=0, col=0, span 키 omit, 문단 1건.
        let c00 = &result.cells[0];
        assert_eq!(c00.row, 0);
        assert_eq!(c00.col, 0);
        assert!(c00.row_span.is_none(), "span=1 → 키 omit");
        assert!(c00.col_span.is_none());
        assert_eq!(c00.paragraphs.len(), 1);
        // 셀 안 첫 문단이 Text 타입이며 cell_locator 가 채워졌는지.
        match &c00.paragraphs[0] {
            IrParagraph::Text(t) => {
                assert_eq!(t.para, -1);
                // id 형식: p_{sec}_{table_para}_c{ctrl_idx}_{cell_idx}_{cell_para}
                assert_eq!(t.id, format!("p_0_0_c{}_0_0", ctrl_idx));
                let loc = t.cell_locator.as_ref().expect("locator");
                assert_eq!(loc.row, 0);
                assert_eq!(loc.col, 0);
            }
            _ => panic!("셀 안 문단은 Text 여야 함"),
        }
        // 마지막 셀 (row=1, col=1) — 셀 순서는 행 우선.
        let c11 = &result.cells[3];
        assert_eq!(c11.row, 1);
        assert_eq!(c11.col, 1);
    }

    #[test]
    fn try_build_cell_with_row_span() {
        // row_span > 1 인 셀 — IR 측 row_span 키가 채워지는지.
        use rhwp::model::control::Control;
        use rhwp::model::table::{Cell, Table};

        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let mut core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        let mut table = Table {
            row_count: 2,
            col_count: 1,
            ..Default::default()
        };
        // 단일 병합 셀 (row_span=2, col_span=1).
        table.cells.push(Cell {
            col: 0,
            row: 0,
            col_span: 1,
            row_span: 2,
            width: 1000,
            height: 1000,
            border_fill_id: 0,
            paragraphs: vec![rhwp::model::paragraph::Paragraph::default()],
            ..Default::default()
        });
        core.document_mut().sections[0].paragraphs[0]
            .controls
            .push(Control::Table(Box::new(table)));
        // blank hwpx 의 첫 문단에는 비-표 control 이 이미 있으므로 *끝에 push 된* 표는
        // 마지막 인덱스. build_table_paragraph_with_mock_table 테스트와 동일 fix.
        let ctrl_idx = core.document().sections[0].paragraphs[0].controls.len() - 1;

        let cell = try_build_cell(&core, 0, 0, ctrl_idx, 0).expect("cell 0");
        assert_eq!(cell.row_span, Some(2));
        assert!(cell.col_span.is_none(), "col_span=1 → 키 omit");
    }

    #[test]
    fn build_ir_slice_text_and_table() {
        // Sub-3 v2: 본문 첫 문단에 mock 2x2 표를 끼워넣고 build_ir_slice 가 *표 본체만*
        // paragraphs[] 에 노출하는지 검증. nested 안 cell_locator 가 4 좌표를 보유해야
        // 모델이 셀 위치를 식별할 수 있다. 셀 평탄 entry 는 제거.
        use rhwp::model::control::Control;
        use rhwp::model::table::{Cell, Table};

        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let mut core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        let mut table = Table {
            row_count: 2,
            col_count: 2,
            ..Default::default()
        };
        for r in 0..2u16 {
            for c in 0..2u16 {
                table.cells.push(Cell {
                    col: c,
                    row: r,
                    col_span: 1,
                    row_span: 1,
                    width: 1000,
                    height: 500,
                    border_fill_id: 0,
                    paragraphs: vec![rhwp::model::paragraph::Paragraph::default()],
                    ..Default::default()
                });
            }
        }
        core.document_mut().sections[0].paragraphs[0]
            .controls
            .push(Control::Table(Box::new(table)));

        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("t".into()),
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        // paragraphs[] 에 table kind 가 적어도 1건.
        let kinds: Vec<&str> = slice
            .paragraphs
            .iter()
            .map(|p| match p {
                IrParagraph::Text(t) => t.kind,
                IrParagraph::Table(t) => t.kind,
            })
            .collect();
        assert!(
            kinds.iter().any(|k| *k == "table"),
            "표 entry 없음: {:?}",
            kinds
        );
        // Sub-3 v2: 평탄 entry 가 *0건* 이어야 한다 (para=-1, cell_locator.is_some()).
        let flat_cell_entries: Vec<_> = slice
            .paragraphs
            .iter()
            .filter_map(|p| match p {
                IrParagraph::Text(t) if t.para == -1 && t.cell_locator.is_some() => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(
            flat_cell_entries.len(),
            0,
            "평탄 cell_locator entry 가 남음: {} 건",
            flat_cell_entries.len()
        );
        // nested cell_locator 검증 — table.cells[i].paragraphs[j].cell_locator 가 4 좌표 보유.
        let table_para = slice
            .paragraphs
            .iter()
            .find_map(|p| match p {
                IrParagraph::Table(t) => Some(t),
                _ => None,
            })
            .expect("table");
        let cell00 = table_para
            .cells
            .iter()
            .find(|c| c.row == 0 && c.col == 0)
            .expect("(0,0)");
        let cell_para0 = cell00.paragraphs.first().expect("cell para");
        if let IrParagraph::Text(cp) = cell_para0 {
            let cl = cp.cell_locator.as_ref().expect("nested cell_locator");
            assert_eq!(cl.row, 0);
            assert_eq!(cl.col, 0);
            assert_eq!(cl.cell_para, 0);
        } else {
            panic!("nested cell paragraph 가 Text 가 아님");
        }
    }

    #[test]
    fn build_ir_slice_text_only_unchanged() {
        // 표가 없는 blank doc — build_paragraph 분기 후에도 텍스트 path 가 *그대로 1건/문단*
        // 으로 동작하는지 회귀 검증.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("t".into()),
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        let total = core.document().sections[0].paragraphs.len();
        // 표 없는 blank 문서 → paragraphs 수 = 섹션 문단 수.
        assert_eq!(slice.paragraphs.len(), total);
        // 모두 Text 타입이어야.
        for p in &slice.paragraphs {
            assert!(matches!(p, IrParagraph::Text(_)));
        }
    }

    #[test]
    fn mode_returns_most_frequent_f64() {
        let v: Vec<f64> = vec![1.0, 2.0, 2.0, 3.0];
        assert_eq!(mode(&v), Some(2.0));
        let empty: Vec<f64> = vec![];
        assert_eq!(mode(&empty), None);
    }

    #[test]
    fn mode_ties_keep_first_string() {
        let v: Vec<String> = vec!["a".into(), "b".into(), "a".into(), "b".into()];
        assert_eq!(mode(&v), Some("a".into()));
    }

    #[test]
    fn compute_doc_defaults_from_empty_slice() {
        let slice = IrSlice {
            doc_meta: IrDocMeta {
                edit_session_id: "t".into(),
                page: 1,
                total_pages: 1,
                anchor: IrAnchor {
                    sec: 0,
                    para_start: 0,
                    para_end: 0,
                },
            },
            paragraphs: vec![],
        };
        let d = compute_doc_defaults(&slice);
        assert_eq!(d.run.bold, Some(false));
        assert_eq!(d.run.color.as_deref(), Some("#000000"));
        assert_eq!(d.run.font_size, Some(10.0));
        assert_eq!(d.run.font_name.as_deref(), Some("맑은 고딕"));
        assert_eq!(d.paragraph.align.as_deref(), Some("left"));
        assert_eq!(d.paragraph.line_height, Some(160));
    }

    #[test]
    fn omit_run_defaults_drops_matching() {
        let style = RunStyle {
            bold: Some(false),
            font_size: Some(22.0),
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
    fn omit_run_defaults_all_same_returns_none() {
        let s = RunStyle {
            bold: Some(false),
            ..Default::default()
        };
        let d = RunStyle {
            bold: Some(false),
            ..Default::default()
        };
        assert!(omit_run_style_defaults(&s, &d).is_none());
    }

    #[test]
    fn compact_text_single_run_inline() {
        let t = IrTextParagraph {
            id: "p_0_0".into(),
            sec: 0,
            para: 0,
            kind: "text",
            style: ParagraphStyle::default(),
            runs: vec![IrRun {
                char_offset: 0,
                length: 3,
                text: "ABC".into(),
                style: RunStyle::default(),
            }],
            cell_locator: None,
            page_break: None,
        };
        let defaults = DocDefaults {
            run: RunStyle::default(),
            paragraph: ParagraphStyle::default(),
        };
        let v = compact_text(&t, &defaults, false);
        assert!(v.get("runs").is_none(), "단일 plain run 은 runs 생략");
        assert_eq!(v["text"], "ABC");
        // Sub-3 v2 Phase 3 — id 항상 omit, type:"text" 도 omit.
        assert!(v.get("id").is_none(), "id 키 잔존");
        assert!(v.get("type").is_none(), "기본 type 'text' 가 명시되어 있음");
        // omit_sec=false 시나리오 — sec 키 유지.
        assert_eq!(v["sec"], 0);
    }

    #[test]
    fn compact_text_styled_run_keeps_runs() {
        let t = IrTextParagraph {
            id: "p_0_0".into(),
            sec: 0,
            para: 0,
            kind: "text",
            style: ParagraphStyle::default(),
            runs: vec![IrRun {
                char_offset: 0,
                length: 3,
                text: "ABC".into(),
                style: RunStyle {
                    bold: Some(true),
                    ..Default::default()
                },
            }],
            cell_locator: None,
            page_break: None,
        };
        let defaults = DocDefaults {
            run: RunStyle {
                bold: Some(false),
                ..Default::default()
            },
            paragraph: ParagraphStyle::default(),
        };
        let v = compact_text(&t, &defaults, false);
        assert!(v.get("text").is_none(), "styled run 은 runs 형태 유지");
        let runs = v["runs"].as_array().expect("runs array");
        assert_eq!(runs[0]["style"]["bold"], true);
        // Sub-3 v2 Phase 3 — 첫 run 의 char_offset:0 은 omit.
        assert!(
            runs[0].get("char_offset").is_none(),
            "첫 run 의 char_offset:0 이 잔존"
        );
    }

    #[test]
    fn compact_run_first_zero_offset_omitted() {
        // Sub-3 v2 Phase 3 — is_first=true + char_offset==0 → 키 omit.
        // is_first=false 이거나 offset != 0 → 키 유지.
        let r = IrRun {
            char_offset: 0,
            length: 3,
            text: "abc".into(),
            style: RunStyle::default(),
        };
        let defaults = DocDefaults {
            run: RunStyle::default(),
            paragraph: ParagraphStyle::default(),
        };
        let v_first = compact_run(&r, &defaults, true);
        assert!(v_first.get("char_offset").is_none(), "첫 run 0 → omit");

        let v_not_first = compact_run(&r, &defaults, false);
        assert_eq!(
            v_not_first["char_offset"], 0,
            "두 번째 이후 run 의 offset 0 은 유지 (실제로는 합쳐졌어야 하지만 안전망)"
        );

        let r2 = IrRun {
            char_offset: 5,
            length: 1,
            text: "d".into(),
            style: RunStyle::default(),
        };
        let v_first_nonzero = compact_run(&r2, &defaults, true);
        assert_eq!(v_first_nonzero["char_offset"], 5, "첫 run 이라도 0 이 아니면 유지");
    }

    #[test]
    fn compact_ir_slice_blank_doc() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_compact_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: None,
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        let v = serde_json::to_value(&slice).unwrap();
        assert_eq!(v["defaults"]["run"]["bold"], false);
        assert_eq!(v["defaults"]["run"]["color"], "#000000");
        assert_eq!(v["doc_meta"]["anchor"]["sec"], 0);

        // Sub-3 v2 Phase 3 — blank 문서는 단일 sec 이므로 paragraph 의 sec 키 omit.
        // id 도 항상 omit, type:"text" 도 omit.
        if let Some(arr) = v["paragraphs"].as_array() {
            if let Some(p0) = arr.first() {
                assert!(p0.get("id").is_none(), "paragraph id 키 잔존");
                assert!(
                    p0.get("sec").is_none(),
                    "단일 sec 문서에서 paragraph sec 키 잔존"
                );
                // type 키는 부재이거나, 부재 시 기본 'text' — 'text' 가 명시되어 있으면 안 됨.
                assert!(
                    p0.get("type").is_none(),
                    "기본 type 'text' 가 명시되어 있음"
                );
            }
        }
    }

    #[test]
    fn compact_ir_slice_omits_sec_when_single() {
        // 명시적 omit_sec 시나리오 — blank 문서로 단일 sec 검증.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_compact_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: None,
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        let v = serde_json::to_value(&slice).unwrap();
        let arr = v["paragraphs"].as_array().expect("paragraphs");
        for p in arr {
            assert!(p.get("sec").is_none(), "단일 sec — paragraph sec 키 잔존: {}", p);
            assert!(p.get("id").is_none(), "id 키 잔존: {}", p);
        }
        // doc_meta.anchor.sec 는 그대로 유지.
        assert_eq!(v["doc_meta"]["anchor"]["sec"], 0);
    }

    #[test]
    fn compact_ir_slice_table_keeps_type_omits_id_sec() {
        // Sub-3 v2 Phase 3 — 표 entry 는 type:"table" 명시 유지, id 항상 omit,
        // 단일 sec 이면 sec omit.
        use rhwp::model::control::Control;
        use rhwp::model::table::{Cell, Table};

        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let mut core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");

        let mut table = Table {
            row_count: 1,
            col_count: 1,
            ..Default::default()
        };
        table.cells.push(Cell {
            col: 0,
            row: 0,
            col_span: 1,
            row_span: 1,
            width: 1000,
            height: 500,
            border_fill_id: 0,
            paragraphs: vec![rhwp::model::paragraph::Paragraph::default()],
            ..Default::default()
        });
        core.document_mut().sections[0].paragraphs[0]
            .controls
            .push(Control::Table(Box::new(table)));

        let slice = build_compact_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("t".into()),
                page: None,
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        let v = serde_json::to_value(&slice).unwrap();
        let arr = v["paragraphs"].as_array().expect("paragraphs");
        let tbl = arr
            .iter()
            .find(|p| p.get("type").and_then(|t| t.as_str()) == Some("table"))
            .expect("table entry");
        // type:"table" 명시 유지.
        assert_eq!(tbl["type"], "table");
        // id 항상 omit, 단일 sec — sec omit.
        assert!(tbl.get("id").is_none(), "table id 잔존");
        assert!(tbl.get("sec").is_none(), "단일 sec — table sec 잔존");
        // 셀 안 paragraph 의 sec 도 omit.
        if let Some(cells) = tbl.get("cells").and_then(|c| c.as_array()) {
            for c in cells {
                if let Some(paras) = c.get("paragraphs").and_then(|p| p.as_array()) {
                    for cp in paras {
                        assert!(
                            cp.get("sec").is_none(),
                            "셀 안 paragraph sec 잔존: {}",
                            cp
                        );
                        assert!(cp.get("id").is_none(), "셀 안 paragraph id 잔존: {}", cp);
                    }
                }
            }
        }
    }

    #[test]
    fn compact_border_4sides_same_to_all() {
        let spec = CellBorderSpec {
            width: Some(100),
            color: Some("#000000".into()),
            ..Default::default()
        };
        let b = CellBorder {
            left: Some(spec.clone()),
            right: Some(spec.clone()),
            top: Some(spec.clone()),
            bottom: Some(spec.clone()),
            all: None,
        };
        let c = compact_border(&b).expect("border");
        assert!(c.all.is_some());
        assert!(c.left.is_none() && c.right.is_none() && c.top.is_none() && c.bottom.is_none());
    }

    #[test]
    fn compact_border_4sides_none_returns_none() {
        let b = CellBorder::default();
        assert!(compact_border(&b).is_none());
    }

    #[test]
    fn build_ir_slice_with_page_0_blank() {
        // Sub-3 v2 — page=0 으로 호출 시 paginator 결과의 첫 페이지 paragraph 범위로
        // sec/start/end 가 매핑되거나, 매핑 실패 시 sec/para_start/para_end 폴백.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("t".into()),
                page: Some(0),
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        // 어느 경로든 anchor.sec 가 0 — blank 문서는 섹션이 1 개뿐.
        assert_eq!(slice.doc_meta.anchor.sec, 0);
    }

    #[test]
    fn build_ir_slice_with_page_out_of_range_falls_back() {
        // page=999 같은 범위 외 — page_to_para_range 가 None → fallback path 동작.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let slice = build_ir_slice(
            &core,
            &BuildOptions {
                sec: 0,
                para_start: 0,
                para_end: None,
                edit_session_id: Some("t".into()),
                page: Some(999),
                page_override_range: None,
                total_pages_override: None,
                ..Default::default()
            },
        );
        // fallback 은 opts.sec / para_start / para_end 사용 — sec=0, para_start=0.
        assert_eq!(slice.doc_meta.anchor.sec, 0);
        assert_eq!(slice.doc_meta.anchor.para_start, 0);
        // para_end 는 섹션 전체 문단 수.
        let total = core.document().sections[0].paragraphs.len();
        assert_eq!(slice.doc_meta.anchor.para_end, total);
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

    // ─── Sub-4 v2: PatchDiff / PatchTarget 검증 ───────────────────────────────

    fn paragraphs_target(values: Vec<serde_json::Value>) -> PatchTarget {
        PatchTarget::Paragraphs { paragraphs: values }
    }

    fn cell_target(v: serde_json::Value) -> PatchTarget {
        PatchTarget::Cell { cell: v }
    }

    fn body_range() -> AffectedRange {
        AffectedRange {
            section: 0,
            before: ParaRange::single(0),
            after: ParaRange::single(0),
            cell: None,
        }
    }

    fn cell_range(cell_idx: Option<usize>) -> AffectedRange {
        AffectedRange {
            section: 0,
            before: ParaRange::single(3),
            after: ParaRange::single(3),
            cell: Some(CellFocus {
                table_para: 3, row: 1, col: 2,
                cell_idx, cell_para: Some(0),
            }),
        }
    }

    #[test]
    fn patch_diff_summary_changed_false_when_identical_paragraphs() {
        let range = body_range();
        let before = paragraphs_target(vec![]);
        let after = paragraphs_target(vec![]);
        let diff = build_patch_diff("replace_runs", &range, before, after);
        assert_eq!(diff.op, "replace_runs");
        assert!(!diff.summary.changed);
        assert_eq!(diff.summary.before_text_len, 0);
        assert_eq!(diff.summary.after_text_len, 0);
    }

    #[test]
    fn patch_diff_summary_changed_true_on_text_diff() {
        let range = body_range();
        let before = paragraphs_target(vec![json!({"runs": [{"text": "abc"}]})]);
        let after = paragraphs_target(vec![json!({"runs": [{"text": "abcXY"}]})]);
        let diff = build_patch_diff("insert_text", &range, before, after);
        assert!(diff.summary.changed);
        assert_eq!(diff.summary.before_text_len, 3);
        assert_eq!(diff.summary.after_text_len, 5);
        assert_eq!(diff.summary.before_para_count, 1);
        assert_eq!(diff.summary.after_para_count, 1);
    }

    #[test]
    fn patch_diff_cell_target_text_len_counts_cell_paragraphs() {
        let range = cell_range(Some(5));
        let before = cell_target(json!({
            "row": 1, "col": 2,
            "paragraphs": [{"runs": [{"text": "이전"}]}],
        }));
        let after = cell_target(json!({
            "row": 1, "col": 2,
            "paragraphs": [{"runs": [{"text": "바뀐 값입니다"}]}],
        }));
        let diff = build_patch_diff("replace_cell_runs", &range, before, after);
        assert!(diff.summary.changed);
        assert_eq!(diff.summary.before_text_len, 2);
        assert_eq!(diff.summary.after_text_len, 7);
        // Cell target 은 paragraph 수가 항상 1 — 셀 한 칸.
        assert_eq!(diff.summary.before_para_count, 1);
        assert_eq!(diff.summary.after_para_count, 1);
    }

    #[test]
    fn patch_diff_paragraph_target_text_len_supports_compact_text_field() {
        // Sub-3 v2 compact 단일 run 축약 — paragraph 가 직접 `text` 키.
        let range = body_range();
        let before = paragraphs_target(vec![json!({"text": "hi"})]);
        let after = paragraphs_target(vec![json!({"text": "hello"})]);
        let diff = build_patch_diff("replace_runs", &range, before, after);
        assert_eq!(diff.summary.before_text_len, 2);
        assert_eq!(diff.summary.after_text_len, 5);
    }

    #[test]
    fn patch_diff_location_carries_ranges() {
        let range = AffectedRange {
            section: 1,
            before: ParaRange { start: 3, end: 4 },
            after: ParaRange { start: 3, end: 6 },
            cell: Some(CellFocus {
                table_para: 3, row: 1, col: 2,
                cell_idx: Some(7), cell_para: Some(0),
            }),
        };
        let diff = build_patch_diff(
            "insert_table", &range,
            paragraphs_target(vec![]),
            paragraphs_target(vec![]),
        );
        assert_eq!(diff.location.section, 1);
        assert_eq!(diff.location.para_start_before, 3);
        assert_eq!(diff.location.para_end_before, 4);
        assert_eq!(diff.location.para_start_after, 3);
        assert_eq!(diff.location.para_end_after, 6);
        let cell = diff.location.cell.as_ref().expect("cell focus");
        assert_eq!(cell.cell_idx, Some(7));
    }

    #[test]
    fn patch_diff_serializes_with_camel_case_and_cell_key() {
        let range = cell_range(Some(0));
        let diff = build_patch_diff(
            "replace_cell_runs", &range,
            cell_target(json!({"row": 1, "col": 2})),
            cell_target(json!({"row": 1, "col": 2, "paragraphs": [{"text": "ok"}]})),
        );
        let v = serde_json::to_value(&diff).unwrap();
        assert!(v["location"].get("paraStartBefore").is_some());
        assert!(v["summary"].get("beforeTextLen").is_some());
        // untagged enum → cell 키가 직접 노출되어야.
        assert!(v["before"].get("cell").is_some(), "Cell variant → 'cell' 키 필요");
        assert!(v["after"].get("cell").is_some());
        assert!(v["before"].get("paragraphs").is_none(), "Cell 일 때 paragraphs 키 없음");
    }

    #[test]
    fn extract_compact_cell_returns_cell_by_index() {
        // 표 paragraph 가 cells 키를 가진 compact 형식.
        let slice = CompactIrSlice {
            doc_meta: IrDocMeta {
                edit_session_id: "x".into(), page: 1, total_pages: 1,
                anchor: IrAnchor { sec: 0, para_start: 0, para_end: 1 },
            },
            paragraphs: vec![json!({
                "cells": [
                    {"row": 0, "col": 0, "paragraphs": []},
                    {"row": 0, "col": 1, "paragraphs": [{"text": "B"}]},
                    {"row": 1, "col": 0, "paragraphs": []},
                ],
            })],
            defaults: DocDefaults { run: RunStyle::default(), paragraph: ParagraphStyle::default() },
        };
        let cell = extract_compact_cell(&slice, Some(1)).expect("cell 1");
        assert_eq!(cell["col"], 1);
        assert_eq!(cell["paragraphs"][0]["text"], "B");

        // 범위 밖 인덱스 → None.
        assert!(extract_compact_cell(&slice, Some(99)).is_none());
        // cell_idx None → None.
        assert!(extract_compact_cell(&slice, None).is_none());
    }

    #[test]
    fn slice_to_target_falls_back_to_paragraphs_when_no_cell_focus() {
        let slice = CompactIrSlice {
            doc_meta: IrDocMeta {
                edit_session_id: "x".into(), page: 1, total_pages: 1,
                anchor: IrAnchor { sec: 0, para_start: 0, para_end: 1 },
            },
            paragraphs: vec![json!({"text": "본문"})],
            defaults: DocDefaults { run: RunStyle::default(), paragraph: ParagraphStyle::default() },
        };
        let range = body_range();
        match slice_to_target(slice, &range) {
            PatchTarget::Paragraphs { paragraphs } => {
                assert_eq!(paragraphs.len(), 1);
                assert_eq!(paragraphs[0]["text"], "본문");
            }
            _ => panic!("paragraphs 가 와야"),
        }
    }

    #[test]
    fn slice_to_target_extracts_cell_when_cell_idx_present() {
        let slice = CompactIrSlice {
            doc_meta: IrDocMeta {
                edit_session_id: "x".into(), page: 1, total_pages: 1,
                anchor: IrAnchor { sec: 0, para_start: 3, para_end: 4 },
            },
            paragraphs: vec![json!({
                "cells": [
                    {"row": 0, "col": 0, "paragraphs": []},
                    {"row": 0, "col": 1, "paragraphs": [{"text": "타깃"}]},
                ],
            })],
            defaults: DocDefaults { run: RunStyle::default(), paragraph: ParagraphStyle::default() },
        };
        let range = cell_range(Some(1));
        match slice_to_target(slice, &range) {
            PatchTarget::Cell { cell } => {
                assert_eq!(cell["col"], 1);
                assert_eq!(cell["paragraphs"][0]["text"], "타깃");
            }
            _ => panic!("cell variant 가 와야"),
        }
    }

    #[test]
    fn slice_to_target_falls_back_when_cell_idx_none() {
        // MergeCells 처럼 cell_idx 가 None 인 경우 — paragraphs 로 fallback.
        let slice = CompactIrSlice {
            doc_meta: IrDocMeta {
                edit_session_id: "x".into(), page: 1, total_pages: 1,
                anchor: IrAnchor { sec: 0, para_start: 3, para_end: 4 },
            },
            paragraphs: vec![json!({"cells": [{"row": 0, "col": 0, "paragraphs": []}]})],
            defaults: DocDefaults { run: RunStyle::default(), paragraph: ParagraphStyle::default() },
        };
        let range = cell_range(None);
        assert!(matches!(slice_to_target(slice, &range), PatchTarget::Paragraphs { .. }));
    }

    // ─── [Sub-7] PatchSummary.noChangeWarning 가시화 ─────────────────────────

    #[test]
    fn patch_summary_no_change_warning_present_when_unchanged() {
        // changed=false 면 noChangeWarning 필드가 채워져야 한다 — silent drop 사고
        // 응답에서 모델/클라가 "성공한 줄 알았는데 안 바뀜" 을 놓치지 않게.
        let range = body_range();
        let before = paragraphs_target(vec![json!({"runs": [{"text": "same"}]})]);
        let after = paragraphs_target(vec![json!({"runs": [{"text": "same"}]})]);
        let diff = build_patch_diff("set_cell_style", &range, before, after);
        assert!(!diff.summary.changed);
        let warn = diff.summary.no_change_warning.as_deref().expect("warning 필드 필요");
        assert!(
            warn.contains("schema"),
            "warning 메시지가 schema 확인을 안내해야 함: {warn}"
        );
    }

    #[test]
    fn patch_summary_no_change_warning_absent_when_changed() {
        // changed=true 면 noChangeWarning 필드는 None.
        let range = body_range();
        let before = paragraphs_target(vec![json!({"runs": [{"text": "a"}]})]);
        let after = paragraphs_target(vec![json!({"runs": [{"text": "b"}]})]);
        let diff = build_patch_diff("insert_text", &range, before, after);
        assert!(diff.summary.changed);
        assert!(
            diff.summary.no_change_warning.is_none(),
            "changed=true 일 때 warning 은 None 이어야 함"
        );
    }

    #[test]
    fn patch_summary_no_change_warning_serialized_in_json() {
        // 직렬화 시 noChangeWarning camelCase 키로 응답에 노출되는지.
        let range = body_range();
        let before = paragraphs_target(vec![json!({"runs": [{"text": "x"}]})]);
        let after = paragraphs_target(vec![json!({"runs": [{"text": "x"}]})]);
        let diff = build_patch_diff("set_cell_style", &range, before, after);
        let s = serde_json::to_string(&diff).unwrap();
        assert!(s.contains("\"noChangeWarning\""), "응답 JSON 에 noChangeWarning 키가 있어야 함: {s}");
    }

    #[test]
    fn patch_summary_no_change_warning_absent_in_json_when_changed() {
        // changed=true 일 때 noChangeWarning 키가 skip_serializing_if 로 제외되는지.
        let range = body_range();
        let before = paragraphs_target(vec![json!({"runs": [{"text": "a"}]})]);
        let after = paragraphs_target(vec![json!({"runs": [{"text": "b"}]})]);
        let diff = build_patch_diff("insert_text", &range, before, after);
        let s = serde_json::to_string(&diff).unwrap();
        assert!(!s.contains("noChangeWarning"), "changed=true 면 키 자체가 응답에 없어야 함: {s}");
    }

    /// m500 — paginator 미실행 / 빈 paginator 자리 fallback. core.pagination() 이 빈 자리에서
    /// total_pages 가 0 떨어지지 않고 1 fallback 인지 확인.
    #[test]
    fn doc_meta_total_pages_falls_back_to_one_for_empty_paginator() {
        let core = DocumentCore::new_empty();
        let opts = BuildOptions::default();
        let slice = build_ir_slice(&core, &opts);
        assert_eq!(slice.doc_meta.total_pages, 1, "빈 paginator → 1 fallback");
        // opts.page None → page_display = 1 (전체 의미의 default)
        assert_eq!(slice.doc_meta.page, 1);
    }

    /// m500 — opts.page = Some(0) (내부 0-based 첫 페이지) 입력 시 doc_meta.page = 1 표시 (1-based).
    /// m400 sub-2 의 main.rs 변환 (외부 1-based → 내부 0-based) 과 정합.
    #[test]
    fn doc_meta_page_one_based_display_when_opts_page_zero() {
        let core = DocumentCore::new_empty();
        let opts = BuildOptions {
            page: Some(0),
            page_override_range: None,
            total_pages_override: None,
            ..Default::default()
        };
        let slice = build_ir_slice(&core, &opts);
        // opts.page = 0 (내부 0-based 첫 페이지) → doc_meta.page = 1 (외부 1-based 표시)
        assert_eq!(slice.doc_meta.page, 1);
    }

    /// m500 — opts.page = Some(2) (내부 0-based 세번째 페이지) → doc_meta.page = 3 (1-based 표시).
    #[test]
    fn doc_meta_page_one_based_display_when_opts_page_two() {
        let core = DocumentCore::new_empty();
        let opts = BuildOptions {
            page: Some(2),
            page_override_range: None,
            total_pages_override: None,
            ..Default::default()
        };
        let slice = build_ir_slice(&core, &opts);
        assert_eq!(slice.doc_meta.page, 3);
    }

    // ─── 신규 detail / include_style / include_tables / max_text_chars 테스트 ────────────

    #[test]
    fn build_outline_slice_blank_doc_has_doc_meta_and_paragraphs() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let v = build_outline_slice(
            &core,
            &BuildOptions {
                detail: Detail::Outline,
                ..Default::default()
            },
        );
        // doc_meta + paragraphs 키 모두 존재.
        assert!(v.get("doc_meta").is_some(), "doc_meta 키 누락");
        let paras = v.get("paragraphs").and_then(|p| p.as_array()).unwrap();
        assert!(!paras.is_empty(), "paragraphs 비어 있음");
        // 첫 paragraph 는 text 또는 빈 텍스트 키 보유.
        let first = &paras[0];
        assert!(first.get("para").is_some());
        // outline 은 본문 text 또는 type:"table" 키 박혀 있음.
        assert!(first.get("text").is_some() || first.get("type").is_some());
    }

    /// outline 이 page break 문단에 `page_break: true` 마커를 노출한다 —
    /// 모델이 outline 만으로 어느 문단이 새 페이지 시작인지 보게 함(로그 0701 대응).
    #[test]
    fn build_outline_slice_exposes_page_break_marker() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let mut core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        // para 0 을 split 하며 새 문단(para 1)에 페이지 나눔 설정.
        core.insert_page_break_native(0, 0, 0).expect("page break");
        let v = build_outline_slice(
            &core,
            &BuildOptions {
                detail: Detail::Outline,
                ..Default::default()
            },
        );
        let paras = v.get("paragraphs").and_then(|p| p.as_array()).unwrap();
        // 어느 한 문단이 page_break:true 마커를 지녀야.
        let has_marker = paras
            .iter()
            .any(|p| p.get("page_break") == Some(&serde_json::json!(true)));
        assert!(has_marker, "outline 에 page_break 마커가 노출돼야");
        // break 없는 문단엔 마커가 생략돼야(None → 키 부재).
        let no_marker = paras.iter().filter(|p| p.get("page_break").is_none()).count();
        assert!(no_marker >= 1, "break 없는 문단엔 page_break 키 생략");
    }

    #[test]
    fn build_outline_slice_max_text_chars_truncates_and_marks() {
        // 빈 문서라 외부 텍스트가 없어, 강제 truncate 단정 테스트는
        // truncate_text 단위 테스트로 분리. 여기서는 호출이 panic 없이 도는지만 확인.
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let v = build_outline_slice(
            &core,
            &BuildOptions {
                detail: Detail::Outline,
                max_text_chars: Some(5),
                ..Default::default()
            },
        );
        assert!(v.get("paragraphs").is_some());
    }

    #[test]
    fn truncate_text_short_unchanged() {
        let (t, trunc) = truncate_text("hi", Some(10));
        assert_eq!(t, "hi");
        assert!(!trunc);
    }

    #[test]
    fn truncate_text_long_cuts_and_flags() {
        let (t, trunc) = truncate_text("abcdefghij", Some(3));
        assert_eq!(t, "abc");
        assert!(trunc);
    }

    #[test]
    fn truncate_text_none_passthrough() {
        let (t, trunc) = truncate_text("abcdef", None);
        assert_eq!(t, "abcdef");
        assert!(!trunc);
    }

    #[test]
    fn truncate_text_korean_char_boundary() {
        // 글자(char) 단위 — UTF-8 바이트 단위가 아님. 4 글자 → 4 글자 그대로.
        let (t, trunc) = truncate_text("한글입력", Some(2));
        assert_eq!(t, "한글");
        assert!(trunc);
    }

    #[test]
    fn build_structure_slice_blank_doc() {
        let bytes = include_bytes!("../../samples/hwpx/blank_hwpx.hwpx");
        let core = rhwp::document_core::DocumentCore::from_bytes(bytes).expect("load");
        let v = build_structure_slice(
            &core,
            &BuildOptions {
                detail: Detail::Structure,
                ..Default::default()
            },
        );
        let paras = v.get("paragraphs").and_then(|p| p.as_array()).unwrap();
        assert!(!paras.is_empty());
        // structure 단계의 본문 paragraph 는 char_count 키 + para 만. 본문 text 키 없음.
        let first = &paras[0];
        assert!(first.get("char_count").is_some() || first.get("type").is_some());
        assert!(first.get("text").is_none(), "structure 단계 자리 본문 text 박혀선 안 됨");
    }

    #[test]
    fn apply_style_filter_none_strips_style_keys() {
        let mut paragraphs = vec![serde_json::json!({
            "para": 0,
            "style": {"align": "center"},
            "runs": [{"text": "x", "style": {"bold": true}}],
        })];
        apply_style_filter(&mut paragraphs, StyleLevel::None);
        let p = &paragraphs[0];
        assert!(p.get("style").is_none(), "paragraph style 미제거");
        let runs = p.get("runs").and_then(|r| r.as_array()).unwrap();
        assert!(runs[0].get("style").is_none(), "run style 미제거");
    }

    #[test]
    fn apply_style_filter_essential_drops_non_whitelist_keys() {
        let mut paragraphs = vec![serde_json::json!({
            "para": 0,
            "runs": [{
                "text": "x",
                "style": {
                    "bold": true,
                    "char-spacing": 50,
                    "underline": true,
                    "font-size": 10.0,
                },
            }],
        })];
        apply_style_filter(&mut paragraphs, StyleLevel::Essential);
        let run = &paragraphs[0]["runs"][0];
        let style = run.get("style").unwrap().as_object().unwrap();
        // 화이트리스트 키만 남아야.
        assert!(style.contains_key("bold"));
        assert!(style.contains_key("font-size"));
        assert!(!style.contains_key("char-spacing"));
        assert!(!style.contains_key("underline"));
    }

    #[test]
    fn apply_style_filter_essential_paragraph_keys() {
        let mut paragraphs = vec![serde_json::json!({
            "para": 0,
            "style": {"align": "center", "indent": 100},
            "runs": [],
        })];
        apply_style_filter(&mut paragraphs, StyleLevel::Essential);
        let style = paragraphs[0].get("style").unwrap().as_object().unwrap();
        assert!(style.contains_key("align"));
        assert!(style.contains_key("indent"));
    }

    #[test]
    fn apply_table_filter_count_removes_cells_key() {
        let mut paragraphs = vec![serde_json::json!({
            "para": 0,
            "type": "table",
            "rows": 2,
            "cols": 2,
            "cells": [{"row": 0, "col": 0}],
        })];
        apply_table_filter(&mut paragraphs, TableLevel::Count);
        let p = &paragraphs[0];
        assert!(p.get("cells").is_none(), "Count 자리 cells 키 미제거");
        assert!(p.get("rows").is_some());
        assert!(p.get("cols").is_some());
    }

    #[test]
    fn apply_table_filter_structure_drops_cell_paragraphs() {
        let mut paragraphs = vec![serde_json::json!({
            "para": 0,
            "type": "table",
            "rows": 1,
            "cols": 1,
            "cells": [{
                "row": 0,
                "col": 0,
                "style": {"bgcolor": "#FFFFFF"},
                "paragraphs": [{"para": -1, "text": "셀 본문"}],
            }],
        })];
        apply_table_filter(&mut paragraphs, TableLevel::Structure);
        let cell = &paragraphs[0]["cells"][0];
        assert!(cell.get("paragraphs").is_none(), "Structure 자리 cell.paragraphs 미제거");
        assert!(cell.get("style").is_some(), "Structure 자리 style 보존");
    }

    #[test]
    fn apply_table_filter_full_no_op() {
        let original = serde_json::json!({
            "para": 0, "type": "table", "rows": 1, "cols": 1,
            "cells": [{"row": 0, "col": 0, "paragraphs": [{"para": -1, "text": "x"}]}],
        });
        let mut paragraphs = vec![original.clone()];
        apply_table_filter(&mut paragraphs, TableLevel::Full);
        assert_eq!(paragraphs[0], original);
    }
}
