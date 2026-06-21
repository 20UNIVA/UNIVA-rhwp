// HWP → HWPX → 다시 HWP 경로로 박은 결과의 BodyText record 크기 확인.
// cycle 44~47 의 fix 효과 검증용.

use flate2::read::DeflateDecoder;
use std::fs;
use std::io::Read;

fn dump_records(label: &str, bytes: &[u8]) {
    println!("=== {} ===", label);
    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(bytes.to_vec())).unwrap();
    let mut s = cfb.open_stream("/BodyText/Section0").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let mut d = DeflateDecoder::new(&raw[..]);
    let mut decoded = Vec::new();
    d.read_to_end(&mut decoded).unwrap();

    let tag_name = |t: u32| match t {
        0x42 => "PARA_HEADER",
        0x43 => "PARA_TEXT",
        0x44 => "PARA_CHAR_SHAPE",
        0x45 => "PARA_LINE_SEG",
        0x47 => "CTRL_HEADER",
        0x48 => "LIST_HEADER",
        0x4D => "TABLE",
        _ => "_",
    };
    let mut pos = 0;
    let mut shown = std::collections::HashMap::<u32, usize>::new();
    let limits = [(0x42u32, 4), (0x47u32, 3), (0x48u32, 3), (0x4Du32, 2)];
    while pos + 4 <= decoded.len() {
        let h = u32::from_le_bytes([decoded[pos], decoded[pos + 1], decoded[pos + 2], decoded[pos + 3]]);
        let tag = h & 0x3FF;
        let level = (h >> 10) & 0x3FF;
        let sf = (h >> 20) & 0xFFF;
        let (size, hdrlen) = if sf == 0xFFF {
            (
                u32::from_le_bytes([decoded[pos + 4], decoded[pos + 5], decoded[pos + 6], decoded[pos + 7]]),
                8,
            )
        } else {
            (sf, 4)
        };
        for &(t, n) in &limits {
            if tag == t {
                let c = shown.entry(t).or_insert(0);
                if *c < n {
                    println!("  pos={:5} tag=0x{:02X} ({}) level={} size={}", pos, tag, tag_name(tag), level, size);
                    *c += 1;
                }
            }
        }
        pos += hdrlen + size as usize;
        if pos > decoded.len() {
            break;
        }
    }
}

fn main() {
    let orig_path = std::env::args().nth(1).expect("orig.hwp");
    let orig = fs::read(&orig_path).unwrap();

    // 1) passthrough (HWP → IR → HWP)
    let core_pass = rhwp::document_core::DocumentCore::from_bytes(&orig).unwrap();
    let pass = core_pass.export_hwp_native().unwrap();
    dump_records("passthrough (HWP → IR → HWP)", &pass);

    // 2) HWPX 경유 (HWP → HWPX → re-parse → adapter → HWP)
    let core_a = rhwp::document_core::DocumentCore::from_bytes(&orig).unwrap();
    let hwpx = core_a.export_hwpx_native().unwrap();
    let mut core_b = rhwp::document_core::DocumentCore::from_bytes(&hwpx).unwrap();
    let via_hwpx = core_b.export_hwp_with_adapter().unwrap();
    dump_records("\nHWPX 경유 (HWP → HWPX → re-parse → HWP)", &via_hwpx);
}
