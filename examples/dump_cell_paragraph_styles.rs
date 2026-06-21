// pink_cell_fixed 의 4행3열 table cell 안 paragraph 의 para_shape·char_shape 참조 확인.
use rhwp::model::control::Control;
use std::fs;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    for (pi, p) in doc.sections[0].paragraphs.iter().enumerate() {
        for ctrl in &p.controls {
            if let Control::Table(t) = ctrl {
                if t.row_count == 4 && t.col_count == 3 {
                    println!("=== 4x3 table at s0.p{} ===", pi);
                    for cell in &t.cells {
                        if cell.row == 0 && cell.col == 0 {
                            println!("cell(0,0) border_fill_id={}", cell.border_fill_id);
                            for (cpi, cp) in cell.paragraphs.iter().enumerate() {
                                println!("  p{} para_shape_id={} char_shapes={}",
                                    cpi, cp.para_shape_id, cp.char_shapes.len());
                                for cs_ref in &cp.char_shapes {
                                    let cs = &doc.doc_info.char_shapes[cs_ref.char_shape_id as usize];
                                    println!("    char_shape[{}] border_fill_id={} base_size={} text_color=0x{:08x}",
                                        cs_ref.char_shape_id, cs.border_fill_id, cs.base_size, cs.text_color);
                                }
                                let ps = &doc.doc_info.para_shapes[cp.para_shape_id as usize];
                                println!("    para_shape[{}] border_fill_id={}",
                                    cp.para_shape_id, ps.border_fill_id);
                            }
                        }
                    }
                }
            }
        }
    }
}
