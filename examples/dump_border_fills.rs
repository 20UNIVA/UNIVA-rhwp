// BorderFill·PageBorderFill·Cell.border_fill_id·SectionDef.page_border_fill 자체 dump.

use rhwp::model::control::Control;
use std::fs;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();

    println!("=== doc_info.border_fills (총 {}) ===", doc.doc_info.border_fills.len());
    for (i, bf) in doc.doc_info.border_fills.iter().enumerate() {
        let fill_summary = match bf.fill.fill_type {
            rhwp::model::style::FillType::None => "None".to_string(),
            rhwp::model::style::FillType::Solid => {
                if let Some(solid) = &bf.fill.solid {
                    format!("Solid(background=0x{:08x}, pattern_color=0x{:08x}, pattern_type={})",
                        solid.background_color, solid.pattern_color, solid.pattern_type)
                } else {
                    "Solid(no solid_fill)".to_string()
                }
            }
            ft => format!("{:?}", ft),
        };
        println!("  bf[{}] fill={}", i, fill_summary);
    }

    println!("\n=== Section page_border_fill ===");
    for (si, sec) in doc.sections.iter().enumerate() {
        let pbf = &sec.section_def.page_border_fill;
        println!("  s{} page_border_fill_id={}", si, pbf.border_fill_id);
        for (ei, extra) in sec.section_def.extra_page_border_fills.iter().enumerate() {
            println!("    extra[{}] border_fill_id={}", ei, extra.border_fill_id);
        }
    }

    println!("\n=== Table cell border_fill_ids ===");
    for (si, sec) in doc.sections.iter().enumerate() {
        for (pi, p) in sec.paragraphs.iter().enumerate() {
            for ctrl in &p.controls {
                if let Control::Table(t) = ctrl {
                    println!("  s{}.p{} table {}x{} ({} cells) table.border_fill_id={} zones={}",
                        si, pi, t.row_count, t.col_count, t.cells.len(),
                        t.border_fill_id, t.zones.len());
                    for (zi, z) in t.zones.iter().enumerate() {
                        println!("    zone[{}] start=({},{}) end=({},{}) border_fill_id={}",
                            zi, z.start_row, z.start_col, z.end_row, z.end_col, z.border_fill_id);
                    }
                    for cell in &t.cells {
                        println!("    cell({},{}) border_fill_id={}",
                            cell.row, cell.col, cell.border_fill_id);
                    }
                }
            }
        }
    }
}
