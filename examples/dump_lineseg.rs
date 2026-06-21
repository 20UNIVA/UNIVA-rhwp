use rhwp::model::control::Control;
use std::fs;

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let target_p = std::env::args().nth(2).expect("p").parse::<usize>().unwrap();
    let target_r = std::env::args().nth(3).expect("row").parse::<u16>().unwrap();
    let target_c = std::env::args().nth(4).expect("col").parse::<u16>().unwrap();
    let data = fs::read(&path).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    let para = &doc.sections[0].paragraphs[target_p];
    for ctrl in &para.controls {
        if let Control::Table(t) = ctrl {
            for cell in &t.cells {
                if cell.row == target_r && cell.col == target_c {
                    println!("cell({},{}) width={} height={}", cell.row, cell.col, cell.width, cell.height);
                    for (pi, p) in cell.paragraphs.iter().enumerate() {
                        println!("  p{} text={:?} char_count={} line_segs={}", pi, p.text, p.char_count, p.line_segs.len());
                        for (li, ls) in p.line_segs.iter().enumerate() {
                            println!("    ls[{}] ts={} vpos={} lh={} th={} sw={}", li, ls.text_start, ls.vertical_pos, ls.line_height, ls.text_height, ls.segment_width);
                        }
                    }
                }
            }
        }
    }
}
