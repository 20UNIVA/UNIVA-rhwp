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
        // p8 = nested table 자체 자체 자체
        let p8 = &outer_cell.paragraphs[8];
        println!("p8 controls: {}", p8.controls.len());
        for c in &p8.controls {
            if let Control::Table(inner) = c {
                println!("nested table found, {}x{}", inner.row_count, inner.col_count);
                let target = inner.cells.iter().find(|c| c.row == 1 && c.col == 1).unwrap();
                println!("(1, 1) paragraphs.len()={}", target.paragraphs.len());
                for (pi, p) in target.paragraphs.iter().enumerate() {
                    println!("  [p{}] text={:?} char_count={} char_offsets_len={} char_shapes_len={} line_segs_len={}",
                        pi, p.text, p.char_count, p.char_offsets.len(), p.char_shapes.len(), p.line_segs.len());
                }
            }
        }
    }
}
