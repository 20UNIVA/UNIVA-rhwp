// BodyText 의 모든 record header (tag, level, size) 자체.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn tag_name(tag: u32) -> &'static str {
    match tag {
        0x42 => "PARA_HEADER",
        0x43 => "PARA_TEXT",
        0x44 => "PARA_CHAR_SHAPE",
        0x45 => "PARA_LINE_SEG",
        0x46 => "PARA_RANGE_TAG",
        0x47 => "CTRL_HEADER",
        0x48 => "LIST_HEADER",
        0x49 => "PAGE_DEF",
        0x4A => "FOOTNOTE_SHAPE",
        0x4B => "PAGE_BORDER_FILL",
        0x4C => "SHAPE_COMPONENT",
        0x4D => "TABLE",
        0x53 => "MEMO_LIST",
        _ => "?",
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let compressed: bool = std::env::args()
        .nth(2)
        .map(|s| s != "false")
        .unwrap_or(true);
    let data = fs::read(&path).unwrap();
    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(data)).unwrap();
    let mut s = cfb.open_stream("/BodyText/Section0").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let bytes = if compressed {
        let mut decoder = DeflateDecoder::new(&raw[..]);
        let mut v = Vec::new();
        decoder.read_to_end(&mut v).unwrap();
        v
    } else {
        raw
    };
    println!("총 BodyText size: {} bytes", bytes.len());
    let mut pos = 0;
    let mut n = 0;
    while pos + 4 <= bytes.len() && n < 50 {
        let header = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        let tag = header & 0x3FF;
        let level = (header >> 10) & 0x3FF;
        let size_field = (header >> 20) & 0xFFF;
        let (size, hdr_len) = if size_field == 0xFFF {
            let s = u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]);
            (s, 8)
        } else {
            (size_field, 4)
        };
        println!(
            "  [{}] pos={:5} tag=0x{:02X} ({}) level={} size={}",
            n,
            pos,
            tag,
            tag_name(tag),
            level,
            size
        );
        pos += hdr_len + size as usize;
        n += 1;
    }
}
