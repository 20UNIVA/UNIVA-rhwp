use std::fs;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let idx: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    let cs = &doc.doc_info.char_shapes[idx];
    println!("cs[{}] base_size={} border_fill_id={} text_color=0x{:08x} shade_color=0x{:08x}",
        idx, cs.base_size, cs.border_fill_id, cs.text_color, cs.shade_color);
    if let Some(raw) = cs.raw_data.as_ref() {
        print!("  raw ({} bytes):", raw.len());
        for (i, b) in raw.iter().enumerate() {
            if i % 8 == 0 { print!("\n    [{:2}]", i); }
            print!(" {:02x}", b);
        }
        println!();
    }
}
