// BodyText 의 첫 record (paragraph header) 자체 hex dump.
// HWP5 record header: tag_id(10bit) + level(10bit) + size(12bit) + (extended size if size==0xFFF)
//
// usage: dump_bodytext_first_record <hwp path>

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let data = fs::read(&path).unwrap();

    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(data)).unwrap();
    let mut s = cfb.open_stream("/BodyText/Section0").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();

    // raw deflate decode
    let mut decoder = DeflateDecoder::new(&raw[..]);
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).unwrap();
    println!("decoded BodyText size: {}", decoded.len());

    // 첫 200 bytes hex
    let n = 200.min(decoded.len());
    print!("첫 {} bytes:\n  ", n);
    for (i, b) in decoded[..n].iter().enumerate() {
        if i > 0 && i % 16 == 0 {
            print!("\n  ");
        }
        print!("{:02x} ", b);
    }
    println!();

    // 첫 record header parse
    if decoded.len() >= 4 {
        let header = u32::from_le_bytes([decoded[0], decoded[1], decoded[2], decoded[3]]);
        let tag_id = header & 0x3FF;
        let level = (header >> 10) & 0x3FF;
        let size = (header >> 20) & 0xFFF;
        let actual_size = if size == 0xFFF {
            u32::from_le_bytes([decoded[4], decoded[5], decoded[6], decoded[7]])
        } else {
            size
        };
        println!(
            "\n첫 record: tag_id=0x{:X} ({}), level={}, size={}",
            tag_id, tag_id, level, actual_size
        );
    }
}
