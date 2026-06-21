// CTRL_HEADER (tag 0x47, level=1) 자리의 ctrl_id + 처음 payload bytes 자체.

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
    println!("=== {} ===", label);
    let mut pos = 0;
    let mut idx = 0;
    while pos + 4 <= bytes.len() && idx < 200 {
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
            // ctrl_id (4 bytes) — LE reversed ASCII (e.g. 'secd')
            let id_str: String = payload[..4].iter().rev().map(|&b| b as char).collect();
            println!(
                "  pos={:5} CTRL_HEADER size={:3} ctrl_id={:?}",
                pos, size, id_str
            );
            // 첫 12 bytes payload
            print!("    payload: ");
            for b in payload.iter().take(12) {
                print!("{:02x} ", b);
            }
            println!();
        }
        pos += hdrlen + size as usize;
        idx += 1;
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
