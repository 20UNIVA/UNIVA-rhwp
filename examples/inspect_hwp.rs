use std::fs;
use rhwp::model::control::Control;
use rhwp::model::table::Table;

fn visit(t: &Table, nested: &mut usize, pictures: &mut usize, shapes: &mut usize, equations: &mut usize, cells_total: &mut usize) {
    *cells_total += t.cells.len();
    for cell in &t.cells {
        for para in &cell.paragraphs {
            for c in &para.controls {
                match c {
                    Control::Table(nt) => { *nested += 1; visit(nt, nested, pictures, shapes, equations, cells_total); }
                    Control::Picture(_) => *pictures += 1,
                    Control::Shape(_) => *shapes += 1,
                    Control::Equation(_) => *equations += 1,
                    _ => {}
                }
            }
        }
    }
}

fn main() {
    let src = std::env::args().nth(1).unwrap();
    let data = fs::read(&src).unwrap();
    let doc = match rhwp::parser::parse_document(&data) {
        Ok(d) => d,
        Err(e) => { eprintln!("parse 실패: {e}"); return; }
    };
    let mut tables = 0usize;
    let mut nested_tables = 0usize;
    let mut pictures = 0usize;
    let mut shapes = 0usize;
    let mut equations = 0usize;
    let mut paragraphs = 0usize;
    let mut cells_total = 0usize;
    for sec in &doc.sections {
        paragraphs += sec.paragraphs.len();
        for para in &sec.paragraphs {
            for c in &para.controls {
                match c {
                    Control::Table(t) => { tables += 1; visit(t, &mut nested_tables, &mut pictures, &mut shapes, &mut equations, &mut cells_total); }
                    Control::Picture(_) => pictures += 1,
                    Control::Shape(_) => shapes += 1,
                    Control::Equation(_) => equations += 1,
                    _ => {}
                }
            }
        }
    }
    let pd = &doc.sections[0].section_def.page_def;
    println!("  sections={} paragraphs={} tables={} nested_tables={} cells={} pictures={} shapes={} equations={}",
        doc.sections.len(), paragraphs, tables, nested_tables, cells_total, pictures, shapes, equations);
    println!("  pageDef: {}x{} margin L={} R={} T={} B={}",
        pd.width, pd.height, pd.margin_left, pd.margin_right, pd.margin_top, pd.margin_bottom);
}
