use std::fs;
use rhwp::model::control::Control;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    let sec = &doc.sections[0];
    let outer_para = &sec.paragraphs[4];
    if let Control::Table(outer) = &outer_para.controls[0] {
        let outer_cell = &outer.cells[0];
        let p8 = &outer_cell.paragraphs[8];
        if let Some(Control::Table(inner)) = p8.controls.first() {
            println!("nested {}x{} cells:", inner.row_count, inner.col_count);
            for cell in &inner.cells {
                let text = cell.paragraphs.first().map(|p| p.text.clone()).unwrap_or_default();
                println!("  ({}, {}) text={:?}", cell.row, cell.col, text);
            }
        } else {
            println!("p8 controls 안 Table 없음 — controls={:?}",
                p8.controls.iter().map(|c| match c {
                    Control::Table(_) => "Table",
                    _ => "Other",
                }).collect::<Vec<_>>());
        }
    }
}
