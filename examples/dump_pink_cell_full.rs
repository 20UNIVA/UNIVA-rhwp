// 4행 3열 table 의 모든 cell 의 모든 paragraph 의 모든 char_shape + 참조 border_fill 정보를
// 박는다. 흰 박스 원인 추적용.
use rhwp::model::control::Control;
use rhwp::model::style::FillType;
use std::fs;

fn fill_desc(bf: &rhwp::model::style::BorderFill) -> String {
    match bf.fill.fill_type {
        FillType::None => "None".to_string(),
        FillType::Solid => {
            let s = bf.fill.solid.as_ref().unwrap();
            format!("Solid(bg=0x{:08x}, pc=0x{:08x})", s.background_color, s.pattern_color)
        }
        FillType::Gradient => "Gradient".to_string(),
        FillType::Image => "Image".to_string(),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("hwp path");
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    println!("=== {} ===", path);
    println!("doc_info: border_fills={} char_shapes={} para_shapes={}",
        doc.doc_info.border_fills.len(),
        doc.doc_info.char_shapes.len(),
        doc.doc_info.para_shapes.len());

    for (pi, p) in doc.sections[0].paragraphs.iter().enumerate() {
        for ctrl in &p.controls {
            if let Control::Table(t) = ctrl {
                if t.row_count == 4 && t.col_count == 3 {
                    println!("\n=== 4x3 table at s0.p{} (cells={}) ===", pi, t.cells.len());
                    for cell in &t.cells {
                        // HWP5 spec: cell.border_fill_id 는 1-based ID — 실제 자리는 id-1
                        let bf_idx = cell.border_fill_id.saturating_sub(1) as usize;
                        let bf = doc.doc_info.border_fills.get(bf_idx);
                        println!("\ncell({},{}) border_fill_id={} -> bf[{}]={}",
                            cell.row, cell.col, cell.border_fill_id, bf_idx,
                            bf.map(fill_desc).unwrap_or_else(|| "?".into()));
                        for (cpi, cp) in cell.paragraphs.iter().enumerate() {
                            let ps = doc.doc_info.para_shapes.get(cp.para_shape_id as usize);
                            let ps_bf = ps.map(|s| s.border_fill_id).unwrap_or(0);
                            println!("  p{} text={:?} char_count={} para_shape_id={} (ps.border_fill_id={}) char_shapes={}",
                                cpi, &cp.text, cp.char_count, cp.para_shape_id, ps_bf, cp.char_shapes.len());
                            for cs_ref in &cp.char_shapes {
                                let cs = doc.doc_info.char_shapes.get(cs_ref.char_shape_id as usize);
                                let cs_bf = cs.map(|c| c.border_fill_id).unwrap_or(0);
                                let cs_bf_idx = cs_bf.saturating_sub(1) as usize;
                                let cs_bf_fill = doc.doc_info.border_fills.get(cs_bf_idx);
                                let cs_bf_desc = cs_bf_fill.map(fill_desc).unwrap_or_else(|| "?".into());
                                let text_color = cs.map(|c| c.text_color).unwrap_or(0);
                                let shade_color = cs.map(|c| c.shade_color).unwrap_or(0);
                                println!("    [start_pos={}] char_shape[{}] border_fill_id={} -> bf[{}]={} text_color=0x{:08x} shade=0x{:08x}",
                                    cs_ref.start_pos, cs_ref.char_shape_id,
                                    cs_bf, cs_bf_idx, cs_bf_desc, text_color, shade_color);
                            }
                        }
                        // cell.raw_list_extra dump
                        println!("  cell.raw_list_extra ({} bytes):", cell.raw_list_extra.len());
                        for (i, b) in cell.raw_list_extra.iter().enumerate() {
                            if i % 16 == 0 { print!("\n    [{:2}]", i); }
                            print!(" {:02x}", b);
                        }
                        println!();
                    }
                }
            }
        }
    }
}
