//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

#![allow(dead_code)]  // 구현 진행 중 일시 허용. Phase 5 종료 시 제거.

use rhwp::document_core::DocumentCore;
use rhwp::model::style::{
    Alignment, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape, UnderlineType,
};
use rhwp::model::table::Cell;
use rhwp::model::ColorRef;
use rhwp::renderer::style_resolver::{
    detect_lang_category, primary_font_name, ResolvedBorderStyle, ResolvedCharStyle,
};
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
/// 빈 문단 (len=0) 에 대해서는 `char_offset=0, length=0, text="", style=default` 1건을
/// 반환 — IR slice 가 빈 문단도 "1개 run" 으로 표현하기로 한 init.md spec 정합.
fn collect_runs<F>(text: &str, len: usize, mut style_at: F) -> Vec<IrRun>
where
    F: FnMut(usize) -> RunStyle,
{
    if len == 0 {
        return vec![IrRun {
            char_offset: 0,
            length: 0,
            text: String::new(),
            style: RunStyle::default(),
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

/// `build_ir_slice` 의 입력 파라미터.
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildIRSlice` 의 옵션 객체와 정합. `sec`
/// 은 섹션 인덱스, `para_start..para_end` 는 *반열림 구간*. `para_end == None` 이면 섹션의
/// 마지막 문단까지, `edit_session_id == None` 이면 현재 시각 (ms) 기반 자동 생성.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub sec: usize,
    pub para_start: usize,
    pub para_end: Option<usize>,
    pub edit_session_id: Option<String>,
}

/// IR slice 진입점 — *텍스트 path 만* 처리 (표 처리는 Phase 4).
///
/// 옛 ts `rhwp-studio/src/llm-replay/ir-builder.ts::buildIRSlice` 의 Rust 대응 중 텍스트 부분.
/// `para_start..para_end` 가 섹션의 문단 수를 초과하면 끝쪽 경계를 잘라 panic 없이 빈 slice 를
/// 반환. `edit_session_id` 미지정 시 `std::time::SystemTime::now()` 기반 ms 타임스탬프로 채움.
pub fn build_ir_slice(core: &DocumentCore, opts: &BuildOptions) -> IrSlice {
    let sec = opts.sec;
    let total = core.document().sections[sec].paragraphs.len();
    let start = opts.para_start.min(total);
    let end = opts.para_end.unwrap_or(total).min(total);

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
        paragraphs.push(IrParagraph::Text(build_text_paragraph(core, sec, p)));
    }

    IrSlice {
        doc_meta: IrDocMeta {
            edit_session_id,
            page: 1,
            total_pages: 1,
            anchor: IrAnchor {
                sec,
                para_start: start,
                para_end: end,
            },
        },
        paragraphs,
    }
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
            },
        );
        assert!(slice.doc_meta.edit_session_id.starts_with("ed_"));
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
