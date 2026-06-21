//! `<hp:tbl>` 표 직렬화.
//!
//! Stage 3 (#182): `Control::Table` IR → `<hp:tbl>` + `<hp:tr>` + `<hp:tc>` + `<hp:subList>` + 문단 재귀.
//!
//! 속성·자식 순서는 한컴 OWPML 공식 (hancom-io/hwpx-owpml-model, Apache 2.0)
//! `Class/Para/TableType.cpp` 의 `WriteElement()`, `InitMap()` 기준:
//!
//! ### `<hp:tbl>` 속성 순서 (부모 AbstractShapeObjectType + 자신)
//! id, zOrder, numberingType, textWrap, textFlow, lock, dropcapstyle,
//! pageBreak, repeatHeader, rowCnt, colCnt, cellSpacing, borderFillIDRef, noAdjust
//!
//! ### `<hp:tbl>` 자식 순서
//! sz, pos, outMargin, (caption, shapeComment, parameterset, metaTag — 옵셔널),
//! inMargin, (cellzoneList — 옵셔널), tr (루프), (label — 옵셔널)
//!
//! ### `<hp:tc>` 속성 순서
//! name, header, hasMargin, protect, editable, dirty, borderFillIDRef
//!
//! ### `<hp:tc>` 자식 순서
//! subList, cellAddr, cellSpan, cellSz, cellMargin
//!
//! ## 중요: table.attr 비트 연산 금지
//!
//! HWPX에서 `table.attr` 는 0인 경우가 많으므로 비트 연산으로 `textWrap/textFlow/pageBreak` 등을
//! 추출하면 안 된다. 반드시 `table.common.text_wrap`, `table.page_break` 등 파싱된 IR 필드를 사용.

use std::io::Write;

use quick_xml::Writer;

use crate::model::shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo};
use crate::model::table::{Cell, Table, TablePageBreak, VerticalAlign};

use super::context::SerializeContext;
use super::utils::{empty_tag, end_tag, start_tag, start_tag_attrs};
use super::SerializeError;

/// `<hp:tbl>` 직렬화.
pub fn write_table<W: Write>(
    w: &mut Writer<W>,
    table: &Table,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    // borderFillIDRef 참조 등록 (assert_all_refs_resolved 검증 대상)
    ctx.border_fill_ids.reference(table.border_fill_id);
    for zone in &table.zones {
        ctx.border_fill_ids.reference(zone.border_fill_id);
    }
    for cell in &table.cells {
        ctx.border_fill_ids.reference(cell.border_fill_id);
    }

    // --- <hp:tbl> 시작 태그 + 속성 ---
    let id_str = table.common.instance_id.to_string();
    let z_order = table.common.z_order.to_string();
    let text_wrap = text_wrap_str(table.common.text_wrap);
    let text_flow = text_flow_str(table.common.text_wrap);
    let lock = bool01(false);
    let page_break = table_page_break_str(table.page_break);
    let repeat_header = bool01(table.repeat_header);
    let row_cnt = table.row_count.to_string();
    let col_cnt = table.col_count.to_string();
    let cell_spacing = table.cell_spacing.to_string();
    let border_fill_id_ref = table.border_fill_id.to_string();
    let no_adjust = bool01((table.attr | table.raw_table_record_attr) & 0x08 != 0);

    start_tag_attrs(
        w,
        "hp:tbl",
        &[
            ("id", &id_str),
            ("zOrder", &z_order),
            ("numberingType", "TABLE"),
            ("textWrap", text_wrap),
            ("textFlow", text_flow),
            ("lock", lock),
            ("dropcapstyle", "None"),
            ("pageBreak", page_break),
            ("repeatHeader", repeat_header),
            ("rowCnt", &row_cnt),
            ("colCnt", &col_cnt),
            ("cellSpacing", &cell_spacing),
            ("borderFillIDRef", &border_fill_id_ref),
            ("noAdjust", no_adjust),
        ],
    )?;

    // --- 자식: sz, pos, outMargin, inMargin, tr[] ---
    write_sz(w, &table.common)?;
    write_pos(w, &table.common)?;
    write_out_margin(w, table)?;
    write_in_margin(w, table)?;

    // tr[]: 행 단위 반복. 각 행에 속한 셀 (cell.row == r) 을 col 오름차순으로 출력.
    for row_idx in 0..table.row_count {
        start_tag(w, "hp:tr")?;
        let mut row_cells: Vec<&Cell> = table.cells.iter().filter(|c| c.row == row_idx).collect();
        row_cells.sort_by_key(|c| c.col);
        for cell in row_cells {
            write_cell(w, cell, ctx)?;
        }
        end_tag(w, "hp:tr")?;
    }

    end_tag(w, "hp:tbl")?;
    Ok(())
}

fn write_sz<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let width = c.width.to_string();
    let height = c.height.to_string();
    empty_tag(
        w,
        "hp:sz",
        &[
            ("width", &width),
            ("widthRelTo", "ABSOLUTE"),
            ("height", &height),
            ("heightRelTo", "ABSOLUTE"),
            ("protect", "0"),
        ],
    )
}

fn write_pos<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let treat = bool01(c.treat_as_char);
    let vert_offset = c.vertical_offset.to_string();
    let horz_offset = c.horizontal_offset.to_string();
    empty_tag(
        w,
        "hp:pos",
        &[
            ("treatAsChar", treat),
            ("affectLSpacing", "0"),
            ("flowWithText", "1"),
            ("allowOverlap", "0"),
            ("holdAnchorAndSO", "0"),
            ("vertRelTo", vert_rel_to_str(c.vert_rel_to)),
            ("horzRelTo", horz_rel_to_str(c.horz_rel_to)),
            ("vertAlign", vert_align_str(c.vert_align)),
            ("horzAlign", horz_align_str(c.horz_align)),
            ("vertOffset", &vert_offset),
            ("horzOffset", &horz_offset),
        ],
    )
}

fn write_out_margin<W: Write>(w: &mut Writer<W>, t: &Table) -> Result<(), SerializeError> {
    let left = t.outer_margin_left.to_string();
    let right = t.outer_margin_right.to_string();
    let top = t.outer_margin_top.to_string();
    let bottom = t.outer_margin_bottom.to_string();
    empty_tag(
        w,
        "hp:outMargin",
        &[
            ("left", &left),
            ("right", &right),
            ("top", &top),
            ("bottom", &bottom),
        ],
    )
}

fn write_in_margin<W: Write>(w: &mut Writer<W>, t: &Table) -> Result<(), SerializeError> {
    let left = t.padding.left.to_string();
    let right = t.padding.right.to_string();
    let top = t.padding.top.to_string();
    let bottom = t.padding.bottom.to_string();
    empty_tag(
        w,
        "hp:inMargin",
        &[
            ("left", &left),
            ("right", &right),
            ("top", &top),
            ("bottom", &bottom),
        ],
    )
}

fn write_cell<W: Write>(
    w: &mut Writer<W>,
    cell: &Cell,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    let name = cell.field_name.as_deref().unwrap_or("");
    let header = bool01(cell.is_header);
    let has_margin = bool01(cell.apply_inner_margin);
    let border_ref = cell.border_fill_id.to_string();

    start_tag_attrs(
        w,
        "hp:tc",
        &[
            ("name", name),
            ("header", header),
            ("hasMargin", has_margin),
            ("protect", "0"),
            ("editable", "0"),
            ("dirty", "0"),
            ("borderFillIDRef", &border_ref),
        ],
    )?;

    // 자식 순서: subList, cellAddr, cellSpan, cellSz, cellMargin
    write_sub_list(w, cell, ctx)?;
    write_cell_addr(w, cell)?;
    write_cell_span(w, cell)?;
    write_cell_sz(w, cell)?;
    write_cell_margin(w, cell)?;

    end_tag(w, "hp:tc")?;
    Ok(())
}

fn write_sub_list<W: Write>(
    w: &mut Writer<W>,
    cell: &Cell,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    start_tag_attrs(
        w,
        "hp:subList",
        &[
            ("id", ""),
            (
                "textDirection",
                if cell.text_direction == 1 {
                    "VERTICAL"
                } else {
                    "HORIZONTAL"
                },
            ),
            ("lineWrap", "BREAK"),
            ("vertAlign", cell_vert_align_str(cell.vertical_align)),
            ("linkListIDRef", "0"),
            ("linkListNextIDRef", "0"),
            ("textWidth", "0"),
            ("textHeight", "0"),
            ("hasTextRef", "0"),
            ("hasNumRef", "0"),
        ],
    )?;

    // 셀 내부 문단 재귀 — 각 문단은 간단한 <hp:p><hp:run><hp:t>텍스트</hp:t></hp:run></hp:p> 구조
    for (pi, para) in cell.paragraphs.iter().enumerate() {
        ctx.para_shape_ids.reference(para.para_shape_id);
        ctx.style_ids.reference(para.style_id as u16);
        if let Some(cs_ref) = para.char_shapes.first() {
            ctx.char_shape_ids.reference(cs_ref.char_shape_id);
        }

        let pi_str = pi.to_string();
        let ppr = para.para_shape_id.to_string();
        let sp = para.style_id.to_string();
        start_tag_attrs(
            w,
            "hp:p",
            &[
                ("id", &pi_str),
                ("paraPrIDRef", &ppr),
                ("styleIDRef", &sp),
                ("pageBreak", "0"),
                ("columnBreak", "0"),
                ("merged", "0"),
            ],
        )?;

        // Task #m600-30 — char_shapes 자료를 자체 자체 자체 별도 <hp:run> 자체 박음.
        // 종전 자료는 char_shapes.first() 의 char_shape_id 자체 자체 자체 자체 단일 run 만
        // 박아 *부분 char_shape (run-level styling — bold·italic·color)* 손실. char_shapes
        // 의 start_pos 자료는 paragraph.text 의 utf16 offset 자체.
        let text_u16: Vec<u16> = para.text.encode_utf16().collect();
        let total_u16 = text_u16.len() as u32;
        let css = &para.char_shapes;
        if css.is_empty() || text_u16.is_empty() {
            // text 가 비어있어도 char_shapes 의 첫 char_shape_id 자료는 보존해야 한다 —
            // 빈 paragraph 자체 자체 자체 자체 자체 char_shape 자료 (다음 단락의 기본
            // 스타일·후속 paragraph 의 cascade 기준) 가 손실되면 round-trip 시 결함.
            let cs = css.first().map(|r| r.char_shape_id).unwrap_or(0);
            if let Some(c) = css.first() {
                ctx.char_shape_ids.reference(c.char_shape_id);
            }
            let cs_str = cs.to_string();
            start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs_str)])?;
            write_cell_text(w, &para.text)?;
            end_tag(w, "hp:run")?;
        } else {
            for (i, cs_ref) in css.iter().enumerate() {
                ctx.char_shape_ids.reference(cs_ref.char_shape_id);
                // char_shapes.start_pos 자체 자체 PARA_CHAR_SHAPE record 자료라 paragraph end
                // marker·control 자체 자체 포함된 utf16 자료. text_u16.len() 자체 자체 clamp.
                let start = (cs_ref.start_pos as usize).min(text_u16.len());
                let end_raw = css
                    .get(i + 1)
                    .map(|c| c.start_pos as usize)
                    .unwrap_or(text_u16.len());
                let end = end_raw.min(text_u16.len());
                if end <= start {
                    continue;
                }
                let segment_u16 = &text_u16[start..end];
                let segment = String::from_utf16_lossy(segment_u16);
                let cs_str = cs_ref.char_shape_id.to_string();
                start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs_str)])?;
                write_cell_text(w, &segment)?;
                end_tag(w, "hp:run")?;
            }
            let _ = total_u16;
        }
        // Task #m600-28 — cell paragraph 의 controls 도 박음 (nested table·picture 등).
        // 종전 자료는 controls 자체 자체 자체 무시하여 표 안 표·표 안 그림이 round-trip 시 손실.
        // controls 자체 자체 자체 별도 hp:run 으로 박음 (char_shape run 자체 자체 자체 자체 분리).
        if !para.controls.is_empty() {
            let ctrl_cs = css.first().map(|r| r.char_shape_id).unwrap_or(0);
            let ctrl_cs_str = ctrl_cs.to_string();
            // hp:run 안 자체 자체 자체 박는 자료 — Table·Picture (HWPX 자체 자체 자체 hp:run
            // 자식 자체 자체 박힘).
            let has_inline_in_run = para.controls.iter().any(|c| {
                matches!(
                    c,
                    crate::model::control::Control::Table(_)
                        | crate::model::control::Control::Picture(_)
                )
            });
            if has_inline_in_run {
                start_tag_attrs(w, "hp:run", &[("charPrIDRef", &ctrl_cs_str)])?;
                for ctrl in &para.controls {
                    match ctrl {
                        crate::model::control::Control::Table(t) => write_table(w, t, ctx)?,
                        crate::model::control::Control::Picture(pic) => {
                            crate::serializer::hwpx::picture::write_picture(w, pic, ctx)?;
                        }
                        _ => {}
                    }
                }
                end_tag(w, "hp:run")?;
            }
            // Task #m600-33 — cell paragraph 의 Field·Bookmark·Hyperlink 박음.
            // 종전 자료는 _ 자체 무시되어 cell 안 페이지번호·날짜·하이퍼링크·책갈피 자체
            // round-trip 시 손실. HWPX 자체 자체 자체 자체 *hp:p > hp:ctrl > hp:fieldBegin*
            // wrapper 자체 자체 자체 (hp:run 자식 자체 자체 — parser 자체 자체 자체 hp:run
            // 자체 자체 자체 자체 자체 자체 hp:fieldBegin 자체 자체 자체 자체 자체 skip).
            for ctrl in &para.controls {
                match ctrl {
                    crate::model::control::Control::Field(f) => {
                        start_tag(w, "hp:ctrl")?;
                        crate::serializer::hwpx::field::write_field_begin(w, f)?;
                        crate::serializer::hwpx::field::write_field_end(w, f.field_id)?;
                        end_tag(w, "hp:ctrl")?;
                    }
                    crate::model::control::Control::Bookmark(bm) => {
                        start_tag(w, "hp:ctrl")?;
                        crate::serializer::hwpx::field::write_bookmark(w, bm)?;
                        end_tag(w, "hp:ctrl")?;
                    }
                    crate::model::control::Control::Hyperlink(link) => {
                        start_tag(w, "hp:ctrl")?;
                        crate::serializer::hwpx::field::write_hyperlink_begin(w, link, 0)?;
                        crate::serializer::hwpx::field::write_field_end(w, 0)?;
                        end_tag(w, "hp:ctrl")?;
                    }
                    _ => {}
                }
            }
        }

        // <hp:linesegarray> — para.line_segs IR 그대로 직렬화 (Task #m600-25 fix).
        // 종전 자료가 *cell paragraph 의 line_segs 자료를 무시하고 단일 정적 lineseg
        // (vertsize=1000, spacing=600) 하드코딩*해 클라 새로고침 시 paginate 깨짐.
        //
        // 추가 자료 (Task #m600-25 cycle 2): HWP5 의 *1 lineseg per paragraph* 규약이
        // HWPX *LinesegTextRunReflow* 비표준으로 검출되어 클라 reflow auto-fix 가 cell
        // 서식·border 자료에 부수효과. 셀 폭 기반 reflow_line_segs 호출 결과 자료를
        // HWPX 정합 line_segs 자료로 직렬화. paragraph 자료 자체는 mutate 안 함 (clone).
        let reflowed_segs: Vec<crate::model::paragraph::LineSeg> =
            if !para.text.is_empty() && para.line_segs.len() <= 1 {
                if let Some(styles) = &ctx.resolved_styles {
                    let mut p = para.clone();
                    let available_px = (cell.width as f64) * ctx.dpi / 7200.0;
                    crate::renderer::composer::reflow_line_segs(
                        &mut p,
                        available_px,
                        styles,
                        ctx.dpi,
                    );
                    p.line_segs
                } else {
                    para.line_segs.clone()
                }
            } else {
                para.line_segs.clone()
            };

        start_tag(w, "hp:linesegarray")?;
        if !reflowed_segs.is_empty() {
            for seg in &reflowed_segs {
                let textpos = seg.text_start.to_string();
                let vertpos = seg.vertical_pos.to_string();
                let vertsize = seg.line_height.to_string();
                let textheight = seg.text_height.to_string();
                let baseline = seg.baseline_distance.to_string();
                let spacing = seg.line_spacing.to_string();
                let horzpos = seg.column_start.to_string();
                let horzsize = seg.segment_width.to_string();
                let flags = seg.tag.to_string();
                empty_tag(
                    w,
                    "hp:lineseg",
                    &[
                        ("textpos", &textpos),
                        ("vertpos", &vertpos),
                        ("vertsize", &vertsize),
                        ("textheight", &textheight),
                        ("baseline", &baseline),
                        ("spacing", &spacing),
                        ("horzpos", &horzpos),
                        ("horzsize", &horzsize),
                        ("flags", &flags),
                    ],
                )?;
            }
        } else {
            // line_segs 비어 있는 자리만 fallback (Document::default() 등).
            empty_tag(
                w,
                "hp:lineseg",
                &[
                    ("textpos", "0"),
                    ("vertpos", "0"),
                    ("vertsize", "1000"),
                    ("textheight", "1000"),
                    ("baseline", "850"),
                    ("spacing", "600"),
                    ("horzpos", "0"),
                    ("horzsize", "12964"),
                    ("flags", "393216"),
                ],
            )?;
        }
        end_tag(w, "hp:linesegarray")?;

        end_tag(w, "hp:p")?;
    }

    end_tag(w, "hp:subList")?;
    Ok(())
}

fn write_cell_text<W: Write>(w: &mut Writer<W>, text: &str) -> Result<(), SerializeError> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    // <hp:t>text</hp:t>
    w.write_event(Event::Start(BytesStart::new("hp:t")))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    if !text.is_empty() {
        w.write_event(Event::Text(BytesText::new(text)))
            .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    }
    w.write_event(Event::End(BytesEnd::new("hp:t")))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

fn write_cell_addr<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let col = cell.col.to_string();
    let row = cell.row.to_string();
    empty_tag(w, "hp:cellAddr", &[("colAddr", &col), ("rowAddr", &row)])
}

fn write_cell_span<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let cs = cell.col_span.max(1).to_string();
    let rs = cell.row_span.max(1).to_string();
    empty_tag(w, "hp:cellSpan", &[("colSpan", &cs), ("rowSpan", &rs)])
}

fn write_cell_sz<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let w_s = cell.width.to_string();
    let h_s = cell.height.to_string();
    empty_tag(w, "hp:cellSz", &[("width", &w_s), ("height", &h_s)])
}

fn write_cell_margin<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let l = cell.padding.left.to_string();
    let r = cell.padding.right.to_string();
    let t = cell.padding.top.to_string();
    let b = cell.padding.bottom.to_string();
    empty_tag(
        w,
        "hp:cellMargin",
        &[("left", &l), ("right", &r), ("top", &t), ("bottom", &b)],
    )
}

// ---------- enum 변환 헬퍼 ----------

fn bool01(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

fn text_wrap_str(w: TextWrap) -> &'static str {
    use TextWrap::*;
    match w {
        Square => "SQUARE",
        Tight => "TIGHT",
        Through => "THROUGH",
        TopAndBottom => "TOP_AND_BOTTOM",
        BehindText => "BEHIND_TEXT",
        InFrontOfText => "IN_FRONT_OF_TEXT",
    }
}

/// textFlow: TextWrap 에 따라 결정 (한컴 관찰값 기준).
fn text_flow_str(w: TextWrap) -> &'static str {
    use TextWrap::*;
    match w {
        Square | Tight | Through => "BOTH_SIDES",
        _ => "BOTH_SIDES",
    }
}

fn table_page_break_str(pb: TablePageBreak) -> &'static str {
    use TablePageBreak::*;
    // Task #m600-28 — HWPX parser (section.rs:1402-1404) 의 주석 정합:
    //   HWPX "CELL"  ↔ HWP5 RowBreak  (한컴 명명 — 셀 안 내부 분할 허용)
    //   HWPX "TABLE" ↔ HWP5 CellBreak (한컴 명명 — 셀 단위로 표를 나눔)
    // 종전 매핑이 반대로 박혀 round-trip 시 RowBreak ↔ CellBreak 가 서로 바뀜.
    match pb {
        None => "NONE",
        RowBreak => "CELL",
        CellBreak => "TABLE",
    }
}

fn vert_rel_to_str(v: VertRelTo) -> &'static str {
    use VertRelTo::*;
    match v {
        Paper => "PAPER",
        Page => "PAGE",
        Para => "PARA",
    }
}

fn horz_rel_to_str(h: HorzRelTo) -> &'static str {
    use HorzRelTo::*;
    match h {
        Paper => "PAPER",
        Page => "PAGE",
        Column => "COLUMN",
        Para => "PARA",
    }
}

fn vert_align_str(v: VertAlign) -> &'static str {
    use VertAlign::*;
    match v {
        Top => "TOP",
        Center => "CENTER",
        Bottom => "BOTTOM",
        Inside => "INSIDE",
        Outside => "OUTSIDE",
    }
}

fn horz_align_str(h: HorzAlign) -> &'static str {
    use HorzAlign::*;
    match h {
        Left => "LEFT",
        Center => "CENTER",
        Right => "RIGHT",
        Inside => "INSIDE",
        Outside => "OUTSIDE",
    }
}

fn cell_vert_align_str(v: VerticalAlign) -> &'static str {
    use VerticalAlign::*;
    match v {
        Top => "TOP",
        Center => "CENTER",
        Bottom => "BOTTOM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Document;
    use crate::model::paragraph::Paragraph;
    use crate::model::table::{Cell, Table};
    use crate::serializer::hwpx::context::SerializeContext;

    fn empty_table(rows: u16, cols: u16) -> Table {
        let mut t = Table::default();
        t.row_count = rows;
        t.col_count = cols;
        for r in 0..rows {
            for c in 0..cols {
                let mut cell = Cell::default();
                cell.col = c;
                cell.row = r;
                cell.col_span = 1;
                cell.row_span = 1;
                cell.width = 1000;
                cell.height = 300;
                cell.paragraphs.push(Paragraph::default());
                t.cells.push(cell);
            }
        }
        t.rebuild_grid();
        t
    }

    fn serialize(table: &Table) -> String {
        let doc = Document::default();
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_table(&mut w, table, &mut ctx).expect("write_table");
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn tbl_root_attrs_in_canonical_order() {
        let t = empty_table(2, 3);
        let xml = serialize(&t);
        assert!(xml.contains("<hp:tbl "), "should emit <hp:tbl>: {}", xml);
        // id → zOrder → numberingType → textWrap → textFlow → lock → dropcapstyle →
        // pageBreak → repeatHeader → rowCnt → colCnt → cellSpacing → borderFillIDRef → noAdjust
        let ip = xml.find("id=").unwrap();
        let zp = xml.find("zOrder=").unwrap();
        let nt = xml.find("numberingType=").unwrap();
        let tw = xml.find("textWrap=").unwrap();
        let tf = xml.find("textFlow=").unwrap();
        let rc = xml.find("rowCnt=").unwrap();
        let cc = xml.find("colCnt=").unwrap();
        let bf = xml.find("borderFillIDRef=").unwrap();
        let na = xml.find("noAdjust=").unwrap();
        assert!(
            ip < zp && zp < nt && nt < tw && tw < tf && tf < rc && rc < cc && cc < bf && bf < na
        );
    }

    #[test]
    fn tr_count_matches_row_count() {
        let t = empty_table(4, 2);
        let xml = serialize(&t);
        assert_eq!(xml.matches("<hp:tr>").count(), 4);
    }

    #[test]
    fn tc_count_matches_cell_count() {
        let t = empty_table(2, 3);
        let xml = serialize(&t);
        assert_eq!(xml.matches("<hp:tc ").count(), 6);
    }

    #[test]
    fn cells_have_canonical_child_order() {
        let t = empty_table(1, 1);
        let xml = serialize(&t);
        // subList → cellAddr → cellSpan → cellSz → cellMargin
        let sl = xml.find("<hp:subList ").unwrap();
        let ca = xml.find("<hp:cellAddr ").unwrap();
        let cs = xml.find("<hp:cellSpan ").unwrap();
        let cz = xml.find("<hp:cellSz ").unwrap();
        let cm = xml.find("<hp:cellMargin ").unwrap();
        assert!(sl < ca && ca < cs && cs < cz && cz < cm);
    }

    #[test]
    fn cell_addr_reflects_coordinates() {
        let t = empty_table(2, 2);
        let xml = serialize(&t);
        assert!(xml.contains(r#"<hp:cellAddr colAddr="0" rowAddr="0"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="1" rowAddr="0"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="0" rowAddr="1"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="1" rowAddr="1"/>"#));
    }

    #[test]
    fn cell_span_defaults_to_one() {
        let t = empty_table(1, 1);
        let xml = serialize(&t);
        assert!(xml.contains(r#"<hp:cellSpan colSpan="1" rowSpan="1"/>"#));
    }

    #[test]
    fn border_fill_id_ref_registered_in_ctx() {
        let doc = Document::default();
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let mut t = empty_table(1, 1);
        t.border_fill_id = 99;
        t.cells[0].border_fill_id = 99;
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_table(&mut w, &t, &mut ctx).unwrap();
        // 99 는 등록되지 않은 borderFill → unresolved
        assert!(ctx.border_fill_ids.unresolved().contains(&99u16));
    }
}
