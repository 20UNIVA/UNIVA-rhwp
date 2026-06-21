// char_shapes[0] raw_data byte-level dump.
use std::fs;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let label = std::env::args().nth(2).unwrap_or("?".into());
    let data = fs::read(&path).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    let cs = &doc.doc_info.char_shapes[0];
    if let Some(ref raw) = cs.raw_data {
        println!("=== {} char_shapes[0] ({} bytes) ===", label, raw.len());
        for (i, b) in raw.iter().enumerate() {
            if i % 8 == 0 { print!("\n  [{:2}] ", i); }
            print!("{:02x} ", b);
        }
        println!();
    }
    println!("\n  IR: shade_color=0x{:08x} shadow_offset=({},{}) shadow_color=0x{:08x}",
        cs.shade_color, cs.shadow_offset_x, cs.shadow_offset_y, cs.shadow_color);
}
