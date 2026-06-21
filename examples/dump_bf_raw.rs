// bf[N] raw_data byte-level dump.
use std::fs;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let idx: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    let bf = &doc.doc_info.border_fills[idx];
    println!("bf[{}] fill_type={:?}", idx, bf.fill.fill_type);
    if let Some(raw) = bf.raw_data.as_ref() {
        println!("  raw_data ({} bytes):", raw.len());
        for (i, b) in raw.iter().enumerate() {
            if i % 8 == 0 { print!("\n    [{:2}]", i); }
            print!(" {:02x}", b);
        }
        println!();
    } else {
        println!("  raw_data = None");
    }
}
