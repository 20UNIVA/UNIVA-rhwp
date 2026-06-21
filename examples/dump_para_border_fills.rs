// para_shape border_fill_id + raw_data 비교.
use std::fs;
fn main() {
    let a = std::env::args().nth(1).unwrap();
    let b = std::env::args().nth(2).unwrap();
    let doc_a = rhwp::parser::parse_document(&fs::read(&a).unwrap()).unwrap();
    let doc_b = rhwp::parser::parse_document(&fs::read(&b).unwrap()).unwrap();
    println!("=== ParaShape border_fill_id 비교 ===");
    for i in 0..doc_a.doc_info.para_shapes.len().max(doc_b.doc_info.para_shapes.len()) {
        let a_id = doc_a.doc_info.para_shapes.get(i).map(|p| p.border_fill_id);
        let b_id = doc_b.doc_info.para_shapes.get(i).map(|p| p.border_fill_id);
        if a_id != b_id {
            println!("  ps[{}] orig.border_fill_id={:?} pink.border_fill_id={:?}", i, a_id, b_id);
        }
    }
    println!("\n=== border_fills 비교 ===");
    for i in 0..doc_a.doc_info.border_fills.len().max(doc_b.doc_info.border_fills.len()) {
        let a_bf = doc_a.doc_info.border_fills.get(i);
        let b_bf = doc_b.doc_info.border_fills.get(i);
        let a_summary = a_bf.map(|bf| format!("{:?}", bf.fill.fill_type));
        let b_summary = b_bf.map(|bf| format!("{:?}", bf.fill.fill_type));
        if a_summary != b_summary {
            println!("  bf[{}] orig={:?} pink={:?}", i, a_summary, b_summary);
        }
    }
}
