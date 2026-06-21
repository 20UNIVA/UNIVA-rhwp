// secd CTRL_HEADER 전체 payload byte-level dump.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn dump(label: &str, path: &str, compressed: bool) {
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
    let mut pos = 0;
    while pos + 4 <= bytes.len() {
        let h = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        let tag = h & 0x3FF;
        let level = (h >> 10) & 0x3FF;
        let sf = (h >> 20) & 0xFFF;
        let (size, hdrlen) = if sf == 0xFFF {
            (
                u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]),
                8,
            )
        } else {
            (sf, 4)
        };
        if tag == 0x47 && level == 1 {
            let payload = &bytes[pos + hdrlen..(pos + hdrlen + size as usize)];
            let id: String = payload[..4].iter().rev().map(|&b| b as char).collect();
            if id == "secd" {
                println!("=== {} secd (size={}) ===", label, size);
                for (i, b) in payload.iter().enumerate() {
                    if i % 8 == 0 {
                        print!("\n  [{:2}] ", i);
                    }
                    print!("{:02x} ", b);
                }
                println!();
                return;
            }
        }
        pos += hdrlen + size as usize;
        if pos > bytes.len() {
            break;
        }
    }
}

fn main() {
    let orig = std::env::args().nth(1).unwrap();
    let modd = std::env::args().nth(2).unwrap();
    dump("orig", &orig, true);
    println!();
    dump("modified", &modd, false);
}
