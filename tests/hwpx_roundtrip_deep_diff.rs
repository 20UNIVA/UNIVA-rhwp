//! HWPX/HWP round-trip deep diff snapshot test (Task #m600-31).
//!
//! 목적 — cycle 25~30 처럼 HWPX serializer 의 cell 안 자료 (paragraph 의 char_shapes·
//! para_shape·line_segs·controls) 가 round-trip 시 손실되는 결함을 자동으로 catch.
//! 기존 `ir-diff` 명령은 paragraph 단위 비교만 박아 cell 안쪽·PageDef·Table.common 자료
//! 손실은 시각 보고가 들어오기 전까지 catch 못 했음.
//!
//! 방식 — 각 fixture 별:
//!   1. 원본 parse → IR_orig
//!   2. serialize_hwpx → re-parse → IR_hwpx
//!   3. serialize_document → re-parse → IR_hwp
//!   4. IR_orig vs IR_hwpx, IR_orig vs IR_hwp 의 deep diff (PageDef·Table·cell paragraph)
//!   5. baseline 파일과 비교 — 처음엔 baseline 박고, 이후엔 동일하면 통과
//!
//! 새 결함이 들어와 손실이 늘어나면 baseline 과 다르므로 fail. 줄어들면 baseline 자료를
//! 갱신해서 좁혀 들어간다.

use std::fs;
use std::path::Path;

use rhwp::model::control::Control;
use rhwp::model::document::Document;
use rhwp::model::table::Table;

#[derive(Debug, Default)]
struct Diffs(Vec<String>);

impl Diffs {
    fn push(&mut self, s: String) {
        self.0.push(s);
    }
    fn finalize(mut self) -> String {
        self.0.sort();
        self.0.join("\n")
    }
}

fn diff_docs(orig: &Document, back: &Document, tag: &str) -> Vec<String> {
    let mut d = Diffs::default();

    // PageDef — 각 section
    let n_secs = orig.sections.len().min(back.sections.len());
    if orig.sections.len() != back.sections.len() {
        d.push(format!(
            "[{tag}] sections.len() {} != {}",
            orig.sections.len(),
            back.sections.len()
        ));
    }
    for si in 0..n_secs {
        let po = &orig.sections[si].section_def.page_def;
        let pb = &back.sections[si].section_def.page_def;
        macro_rules! cmp {
            ($field:ident) => {
                if po.$field != pb.$field {
                    d.push(format!(
                        "[{tag}] s{}.pageDef.{} {} != {}",
                        si,
                        stringify!($field),
                        po.$field,
                        pb.$field
                    ));
                }
            };
        }
        cmp!(width);
        cmp!(height);
        cmp!(margin_left);
        cmp!(margin_right);
        cmp!(margin_top);
        cmp!(margin_bottom);
        cmp!(margin_header);
        cmp!(margin_footer);
        cmp!(margin_gutter);
    }

    // section.paragraphs — top level tables 자료
    for si in 0..n_secs {
        let po = &orig.sections[si].paragraphs;
        let pb = &back.sections[si].paragraphs;
        if po.len() != pb.len() {
            d.push(format!(
                "[{tag}] s{}.paragraphs.len() {} != {}",
                si,
                po.len(),
                pb.len()
            ));
        }
        let n_p = po.len().min(pb.len());
        for pi in 0..n_p {
            diff_paragraph_controls(&po[pi].controls, &pb[pi].controls, &format!("s{si}.p{pi}"), tag, &mut d);
        }
    }

    d.0
}

fn diff_paragraph_controls(
    a: &[Control],
    b: &[Control],
    path: &str,
    tag: &str,
    d: &mut Diffs,
) {
    let n = a.len().min(b.len());
    if a.len() != b.len() {
        d.push(format!(
            "[{tag}] {path}.controls.len() {} != {}",
            a.len(),
            b.len()
        ));
    }
    for ci in 0..n {
        match (&a[ci], &b[ci]) {
            (Control::Table(ta), Control::Table(tb)) => {
                diff_table(ta, tb, &format!("{path}.c{ci}"), tag, d);
            }
            (Control::Picture(_), Control::Picture(_)) => { /* 자료 비교 추후 확장 */ }
            (Control::Shape(_), Control::Shape(_)) => {}
            (Control::Equation(_), Control::Equation(_)) => {}
            (x, y) => {
                let xk = control_kind(x);
                let yk = control_kind(y);
                if xk != yk {
                    d.push(format!("[{tag}] {path}.c{ci} kind {xk} != {yk}"));
                }
            }
        }
    }
}

fn control_kind(c: &Control) -> &'static str {
    match c {
        Control::Table(_) => "Table",
        Control::Picture(_) => "Picture",
        Control::Shape(_) => "Shape",
        Control::Equation(_) => "Equation",
        Control::Footnote(_) => "Footnote",
        Control::Endnote(_) => "Endnote",
        Control::Field(_) => "Field",
        Control::Form(_) => "Form",
        Control::Ruby(_) => "Ruby",
        Control::CharOverlap(_) => "CharOverlap",
        _ => "Other",
    }
}

fn diff_table(a: &Table, b: &Table, path: &str, tag: &str, d: &mut Diffs) {
    // Table.common 의 핵심 자료
    macro_rules! cmp_common {
        ($field:ident) => {
            if a.common.$field != b.common.$field {
                d.push(format!(
                    "[{tag}] {path}.tbl.common.{} {:?} != {:?}",
                    stringify!($field),
                    a.common.$field,
                    b.common.$field
                ));
            }
        };
    }
    cmp_common!(width);
    cmp_common!(height);
    cmp_common!(text_wrap);
    cmp_common!(treat_as_char);
    cmp_common!(vert_rel_to);
    cmp_common!(horz_rel_to);
    if a.outer_margin_left != b.outer_margin_left
        || a.outer_margin_right != b.outer_margin_right
        || a.outer_margin_top != b.outer_margin_top
        || a.outer_margin_bottom != b.outer_margin_bottom
    {
        d.push(format!(
            "[{tag}] {path}.tbl.outer_margin ({},{},{},{}) != ({},{},{},{})",
            a.outer_margin_left,
            a.outer_margin_right,
            a.outer_margin_top,
            a.outer_margin_bottom,
            b.outer_margin_left,
            b.outer_margin_right,
            b.outer_margin_top,
            b.outer_margin_bottom
        ));
    }
    if a.page_break != b.page_break {
        d.push(format!(
            "[{tag}] {path}.tbl.page_break {:?} != {:?}",
            a.page_break, b.page_break
        ));
    }
    if a.row_count != b.row_count || a.col_count != b.col_count {
        d.push(format!(
            "[{tag}] {path}.tbl.size {}x{} != {}x{}",
            a.row_count, a.col_count, b.row_count, b.col_count
        ));
    }
    if a.cells.len() != b.cells.len() {
        d.push(format!(
            "[{tag}] {path}.tbl.cells.len() {} != {}",
            a.cells.len(),
            b.cells.len()
        ));
    }
    let n = a.cells.len().min(b.cells.len());
    for ci in 0..n {
        let ca = &a.cells[ci];
        let cb = &b.cells[ci];
        let cell_path = format!("{path}.tbl.cell({},{})", ca.row, ca.col);
        if ca.paragraphs.len() != cb.paragraphs.len() {
            d.push(format!(
                "[{tag}] {}.paragraphs.len() {} != {}",
                cell_path,
                ca.paragraphs.len(),
                cb.paragraphs.len()
            ));
        }
        let np = ca.paragraphs.len().min(cb.paragraphs.len());
        for pi in 0..np {
            let pa = &ca.paragraphs[pi];
            let pb = &cb.paragraphs[pi];
            let pp = format!("{cell_path}.p{pi}");
            if pa.para_shape_id != pb.para_shape_id {
                d.push(format!(
                    "[{tag}] {pp}.para_shape_id {} != {}",
                    pa.para_shape_id, pb.para_shape_id
                ));
            }
            if pa.char_shapes.len() != pb.char_shapes.len() {
                d.push(format!(
                    "[{tag}] {pp}.char_shapes.len() {} != {}",
                    pa.char_shapes.len(),
                    pb.char_shapes.len()
                ));
            }
            if pa.line_segs.len() != pb.line_segs.len() {
                d.push(format!(
                    "[{tag}] {pp}.line_segs.len() {} != {}",
                    pa.line_segs.len(),
                    pb.line_segs.len()
                ));
            }
            // cell 안 nested table·picture 자료 재귀
            diff_paragraph_controls(&pa.controls, &pb.controls, &pp, tag, d);
        }
    }
}

fn check_fixture(rel_path: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let full = Path::new(manifest).join(rel_path);
    let data = fs::read(&full).unwrap_or_else(|e| panic!("read fixture {rel_path}: {e}"));
    let orig = rhwp::parser::parse_document(&data)
        .unwrap_or_else(|e| panic!("parse orig {rel_path}: {e}"));

    // HWPX round-trip
    let hwpx_bytes = rhwp::serializer::serialize_hwpx(&orig)
        .unwrap_or_else(|e| panic!("serialize_hwpx {rel_path}: {:?}", e));
    let hwpx_back = rhwp::parser::parse_document(&hwpx_bytes)
        .unwrap_or_else(|e| panic!("reparse hwpx {rel_path}: {e}"));
    let hwpx_diffs = diff_docs(&orig, &hwpx_back, "HWPX");

    // HWP round-trip
    let hwp_bytes = rhwp::serialize_document(&orig)
        .unwrap_or_else(|e| panic!("serialize_document {rel_path}: {:?}", e));
    let hwp_back = rhwp::parser::parse_document(&hwp_bytes)
        .unwrap_or_else(|e| panic!("reparse hwp {rel_path}: {e}"));
    let hwp_diffs = diff_docs(&orig, &hwp_back, "HWP");

    // snapshot baseline 자료 비교
    let mut all = Diffs::default();
    for s in hwpx_diffs.into_iter().chain(hwp_diffs.into_iter()) {
        all.push(s);
    }
    let actual = all.finalize();

    let baseline_path = format!("{}/{}.baseline.txt", manifest, rel_path);
    let bp = Path::new(&baseline_path);
    if !bp.exists() || std::env::var("RHWP_UPDATE_BASELINES").is_ok() {
        fs::write(bp, &actual).unwrap_or_else(|e| panic!("write baseline {baseline_path}: {e}"));
        eprintln!("baseline 박음: {baseline_path}");
        return;
    }
    let expected = fs::read_to_string(bp).unwrap();
    if actual != expected {
        eprintln!("====== actual ======\n{actual}\n====== expected ======\n{expected}\n======");
        panic!("baseline diff 자료 변경 ({rel_path}) — 손실 늘어남이면 fix, 줄어듦이면 RHWP_UPDATE_BASELINES=1 으로 갱신");
    }
}

#[test]
fn fixture_baseline_business_table() {
    check_fixture("samples/hwpx_roundtrip/baseline_business_table.hwp");
}

#[test]
fn fixture_multi_section_nested() {
    check_fixture("samples/hwpx_roundtrip/multi_section_nested.hwpx");
}

#[test]
fn fixture_pictures_equations() {
    check_fixture("samples/hwpx_roundtrip/pictures_equations.hwp");
}
