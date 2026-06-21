// PAGE_BORDER_FILL record (tag 0x4B) byte-level dump.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn dump(label: &str, path: &str) {
    let data = fs::read(path).unwrap();
    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(data)).unwrap();
    let mut s = cfb.open_stream("/BodyText/Section0").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let mut d = DeflateDecoder::new(&raw[..]);
    let mut bytes = Vec::new();
    d.read_to_end(&mut bytes).unwrap();
    println!("=== {} ===", label);
    let mut pos = 0;
    let mut count = 0;
    while pos + 4 <= bytes.len() && count < 5 {
        let h = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]);
        let tag = h & 0x3FF;
        let sf = (h >> 20) & 0xFFF;
        let (size, hdrlen) = if sf == 0xFFF {
            (u32::from_le_bytes([bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7]]), 8)
        } else { (sf, 4) };
        if tag == 0x4B {
            let payload = &bytes[pos+hdrlen..(pos+hdrlen+size as usize)];
            let bf_id = u16::from_le_bytes([payload[0], payload[1]]);
            print!("  [{}] PAGE_BORDER_FILL size={} bf_id={}, payload:", count, size, bf_id);
            for b in payload { print!(" {:02x}", b); }
            println!();
            count += 1;
        }
        pos += hdrlen + size as usize;
        if pos > bytes.len() { break; }
    }
}

fn main() {
    let orig = std::env::args().nth(1).unwrap();
    let modd = std::env::args().nth(2).unwrap();
    dump("orig", &orig);
    println!();
    dump("pink_cell", &modd);
}
