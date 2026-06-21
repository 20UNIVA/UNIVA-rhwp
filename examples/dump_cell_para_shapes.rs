use std::fs;
use rhwp::model::control::Control;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let label = std::env::args().nth(2).unwrap_or("".to_string());
    // table_para, row, col 자체 자체 자체 자체 자체
    let table_para: usize = std::env::args().nth(3).unwrap_or("2".to_string()).parse().unwrap();
    let target_row: u16 = std::env::args().nth(4).unwrap_or("1".to_string()).parse().unwrap();
    let target_col: u16 = std::env::args().nth(5).unwrap_or("1".to_string()).parse().unwrap();
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    let sec = &doc.sections[0];
    let outer_para = &sec.paragraphs[table_para];
    for c in &outer_para.controls {
        if let Control::Table(t) = c {
            for cell in &t.cells {
                if cell.row == target_row && cell.col == target_col {
                    println!("[{}] table_para={} cell({},{}) paragraphs.len()={}", label, table_para, target_row, target_col, cell.paragraphs.len());
                    for (pi, p) in cell.paragraphs.iter().enumerate() {
                        let first_cs = p.char_shapes.first().map(|cs| cs.char_shape_id).unwrap_or(0);
                        let cs_count = p.char_shapes.len();
                        println!("  [p{}] para_shape={} char_shape={} (count={}) text_len={} text={:?}",
                            pi, p.para_shape_id, first_cs, cs_count, p.text.chars().count(), p.text);
                    }
                    return;
                }
            }
        }
    }
}
