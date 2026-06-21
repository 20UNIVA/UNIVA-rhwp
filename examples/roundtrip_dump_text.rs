// Task #m600-41 진단 — Field 가 들어간 cell paragraph 의 round-trip 전/후 text 비교.
// usage: cargo run --example roundtrip_dump_text -- <hwp/hwpx path> <p_idx> <row> <col>

use rhwp::model::control::Control;
use std::fs;

fn dump_para(label: &str, doc: &rhwp::model::document::Document, p_idx: usize, row: u16, col: u16) {
    let para = &doc.sections[0].paragraphs[p_idx];
    for ctrl in &para.controls {
        if let Control::Table(t) = ctrl {
            for cell in &t.cells {
                if cell.row == row && cell.col == col {
                    for (pi, p) in cell.paragraphs.iter().enumerate() {
                        let names: Vec<&str> = p
                            .controls
                            .iter()
                            .map(|c| match c {
                                Control::Field(_) => "Field",
                                Control::Hyperlink(_) => "Hyperlink",
                                Control::Bookmark(_) => "Bookmark",
                                _ => "Other",
                            })
                            .collect();
                        println!(
                            "  [{}] p{} text={:?} char_count={} controls={:?}",
                            label, pi, p.text, p.char_count, names
                        );
                    }
                }
            }
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let p_idx: usize = std::env::args().nth(2).expect("p_idx").parse().unwrap();
    let row: u16 = std::env::args().nth(3).expect("row").parse().unwrap();
    let col: u16 = std::env::args().nth(4).expect("col").parse().unwrap();

    let data = fs::read(&path).unwrap();
    let doc_orig = rhwp::parser::parse_document(&data).unwrap();
    println!("=== 원본 ===");
    dump_para("orig", &doc_orig, p_idx, row, col);

    // serialize_hwpx → re-parse
    let hwpx_bytes = rhwp::serializer::serialize_hwpx(&doc_orig).expect("serialize_hwpx");
    let doc_hwpx = rhwp::parser::parse_document(&hwpx_bytes).expect("re-parse hwpx");
    println!("=== HWPX round-trip ===");
    dump_para("hwpx", &doc_hwpx, p_idx, row, col);
}
