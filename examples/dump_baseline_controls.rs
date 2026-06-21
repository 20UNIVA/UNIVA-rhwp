// Task #m600-33 진단 — baseline diff 자리의 cell paragraph controls 자료 자체 variant 분류.
// usage: cargo run --example dump_baseline_controls -- <fixture path>

use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;
use rhwp::model::table::Table;
use std::fs;

fn variant_name(c: &Control) -> &'static str {
    match c {
        Control::SectionDef(_) => "SectionDef",
        Control::ColumnDef(_) => "ColumnDef",
        Control::Table(_) => "Table",
        Control::Shape(_) => "Shape",
        Control::Picture(_) => "Picture",
        Control::Header(_) => "Header",
        Control::Footer(_) => "Footer",
        Control::Footnote(_) => "Footnote",
        Control::Endnote(_) => "Endnote",
        Control::AutoNumber(_) => "AutoNumber",
        Control::NewNumber(_) => "NewNumber",
        Control::PageNumberPos(_) => "PageNumberPos",
        Control::Bookmark(_) => "Bookmark",
        Control::Hyperlink(_) => "Hyperlink",
        Control::Ruby(_) => "Ruby",
        Control::CharOverlap(_) => "CharOverlap",
        Control::PageHide(_) => "PageHide",
        Control::HiddenComment(_) => "HiddenComment",
        Control::Equation(_) => "Equation",
        Control::Field(_) => "Field",
        Control::Form(_) => "Form",
        Control::Unknown(_) => "Unknown",
    }
}

fn dump_para(prefix: &str, para: &Paragraph) {
    if !para.controls.is_empty() {
        let names: Vec<&str> = para.controls.iter().map(variant_name).collect();
        println!("{} controls={:?}", prefix, names);
    }
    for ctrl in &para.controls {
        if let Control::Table(t) = ctrl {
            dump_table(prefix, t);
        }
    }
}

fn dump_table(prefix: &str, table: &Table) {
    for cell in &table.cells {
        for (pi, p) in cell.paragraphs.iter().enumerate() {
            let prefix2 = format!("{}.tbl.cell({},{}).p{}", prefix, cell.row, cell.col, pi);
            dump_para(&prefix2, p);
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("fixture path");
    let data = fs::read(&path).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    for (si, sec) in doc.sections.iter().enumerate() {
        for (pi, p) in sec.paragraphs.iter().enumerate() {
            let prefix = format!("s{}.p{}", si, pi);
            dump_para(&prefix, p);
        }
    }
}
