// PARA_HEADER record 자체 24 bytes byte-level dump. 원본 vs 수정본 비교.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn dump_first_para_header(label: &str, path: &str, compressed: bool) {
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
    // 첫 PARA_HEADER record (header 4 bytes 다음)
    let header = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let size = (header >> 20) & 0xFFF;
    println!("=== {} (size={}) ===", label, size);
    let payload = &bytes[4..(4 + size as usize)];
    print!("  ");
    for (i, b) in payload.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            print!("\n  ");
        }
        print!("{:02x} ", b);
    }
    println!();
}

fn main() {
    let orig = std::env::args().nth(1).unwrap();
    let modd = std::env::args().nth(2).unwrap();
    dump_first_para_header("orig", &orig, true);
    println!();
    dump_first_para_header("modified", &modd, false);
}
