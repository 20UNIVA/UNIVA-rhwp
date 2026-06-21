// HWP → HWPX → HWP round-trip 후 PrvImage·PrvText 크기 비교.

use std::fs;

fn dump_preview(label: &str, bytes: &[u8]) {
    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(bytes.to_vec())).unwrap();
    for path in ["/PrvImage", "/PrvText"] {
        if let Ok(entry) = cfb.entry(path) {
            println!("  [{}] {} : {} bytes", label, path, entry.len());
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let data = fs::read(&path).unwrap();
    println!("=== 원본 ===");
    dump_preview("orig", &data);

    let mut core = rhwp::document_core::DocumentCore::from_bytes(&data).unwrap();
    let hwpx = core.export_hwpx_native().unwrap();
    let mut core2 = rhwp::document_core::DocumentCore::from_bytes(&hwpx).unwrap();
    let hwp = core2.export_hwp_with_adapter().unwrap();
    println!("\n=== HWP→HWPX→HWP round-trip ===");
    dump_preview("rt", &hwp);
}
