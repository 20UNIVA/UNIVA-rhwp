// col_def CTRL_HEADER (tag 0x47, level=1) byte-level dump.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn dump_col_def(label: &str, path: &str, compressed: bool) {
    let data = fs::read(path).unwrap();
    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(data)).unwrap();
    let mut s = cfb.open_stream("/BodyText/Section0").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let bytes = if compressed {
        let mut d = DeflateDecoder::new(&raw[..]);
        let mut v = Vec::new();
        d.read_to_end(&mut v).unwrap();
        v
    } else {
        raw
    };
    // CTRL_HEADER (tag=0x47, level=1) 첫 자리 — ctrl_id 처음 4 bytes 박혀 col_def 식별 ('dloc' LE = 0x636F6C64)
    let mut pos = 0;
    while pos + 4 <= bytes.len() {
        let h = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        let tag = h & 0x3FF;
        let level = (h >> 10) & 0x3FF;
        let sf = (h >> 20) & 0xFFF;
        let (size, hdrlen) = if sf == 0xFFF {
            (u32::from_le_bytes([bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7]]), 8)
        } else { (sf, 4) };
        if tag == 0x47 && level == 1 && size >= 30 {
            let payload = &bytes[pos+hdrlen..(pos+hdrlen+size as usize)];
            let ctrl_id_str = std::str::from_utf8(&payload[..4]).unwrap_or("?").chars().rev().collect::<String>();
            if ctrl_id_str.trim() == "cold" {
                println!("=== {} (size={}, ctrl_id={:?}) ===", label, size, ctrl_id_str);
                print!("  ");
                for (i, b) in payload.iter().enumerate() {
                    if i > 0 && i % 8 == 0 { print!("\n  "); }
                    print!("{:02x} ", b);
                }
                println!();
                return;
            }
        }
        pos += hdrlen + size as usize;
        if pos > bytes.len() { break; }
    }
    println!("=== {} — col_def 미발견 ===", label);
}

fn main() {
    let orig = std::env::args().nth(1).unwrap();
    let modd = std::env::args().nth(2).unwrap();
    dump_col_def("orig", &orig, true);
    println!();
    dump_col_def("modified", &modd, false);
}
