// p0 (애국가 제목) 의 char_shapes + 연결된 doc_info.char_shapes 자체 자체.
use std::fs;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    let p0 = &doc.sections[0].paragraphs[0];
    println!("=== p0 (애국가 제목) ===");
    println!("  text={:?} char_count={}", p0.text, p0.char_count);
    println!("  char_shapes (CharShapeRef list): {}", p0.char_shapes.len());
    for cs_ref in &p0.char_shapes {
        println!("    start_pos={} char_shape_id={}", cs_ref.start_pos, cs_ref.char_shape_id);
    }
    println!("\n=== doc_info.char_shapes 자체 ===");
    let used_ids: Vec<u32> = p0.char_shapes.iter().map(|r| r.char_shape_id).collect();
    for &id in &used_ids {
        let cs = &doc.doc_info.char_shapes[id as usize];
        println!("  cs[{}]: base_size={} text_color=0x{:08x} bold={} italic={} font_ids[0]={} relative_sizes[0]={}",
            id, cs.base_size, cs.text_color, cs.bold, cs.italic, cs.font_ids[0], cs.relative_sizes[0]);
    }
}
