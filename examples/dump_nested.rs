use std::fs;
use rhwp::model::control::Control;
fn count_nested(table: &rhwp::model::table::Table, depth: usize, label: &str) {
    for (ci, cell) in table.cells.iter().enumerate() {
        for (pi, para) in cell.paragraphs.iter().enumerate() {
            for (cci, c) in para.controls.iter().enumerate() {
                if let Control::Table(t) = c {
                    println!("{}{} cell[{}]({},{}) p{}.c{} nested table {}x{} cells={} raw_ctrl_data.len={}",
                        "  ".repeat(depth), label, ci, cell.row, cell.col, pi, cci,
                        t.row_count, t.col_count, t.cells.len(), t.raw_ctrl_data.len());
                    count_nested(t, depth+1, label);
                }
            }
        }
    }
}
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let label = std::env::args().nth(2).unwrap_or("".to_string());
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    for (si, sec) in doc.sections.iter().enumerate() {
        for (pi, para) in sec.paragraphs.iter().enumerate() {
            for (ci, c) in para.controls.iter().enumerate() {
                if let Control::Table(t) = c {
                    println!("[{}] s{}.p{}.c{} TOP table {}x{} cells={} raw_ctrl_data.len={}",
                        label, si, pi, ci, t.row_count, t.col_count, t.cells.len(), t.raw_ctrl_data.len());
                    count_nested(t, 1, &label);
                }
            }
        }
    }
}
