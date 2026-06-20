use std::fs;
use rhwp::model::control::Control;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let label = std::env::args().nth(2).unwrap_or("".to_string());
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    // s0.p4.c0 의 표 자료
    let sec = &doc.sections[0];
    let para = &sec.paragraphs[4];
    for (ci, c) in para.controls.iter().enumerate() {
        if let Control::Table(t) = c {
            println!("[{}] s0.p4.c{} table {}x{} cells={}", label, ci, t.row_count, t.col_count, t.cells.len());
            for (i, cell) in t.cells.iter().enumerate() {
                println!("  cell[{}] r={} c={} w={} h={} paragraphs={}",
                    i, cell.row, cell.col, cell.width, cell.height, cell.paragraphs.len());
                for (pi, p) in cell.paragraphs.iter().enumerate() {
                    let text_preview = if p.text.is_empty() { "(empty)".to_string() } else { p.text.chars().take(30).collect::<String>() };
                    let ctrl_types: Vec<&str> = p.controls.iter().map(|c| match c {
                        Control::Table(_) => "Table",
                        Control::Shape(_) => "Shape",
                        Control::Picture(_) => "Picture",
                        _ => "Other",
                    }).collect();
                    println!("    p{}.text={:?} controls={:?}", pi, text_preview, ctrl_types);
                }
            }
        }
    }
}
