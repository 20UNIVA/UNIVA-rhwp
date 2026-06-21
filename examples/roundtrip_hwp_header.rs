// HWP parse → serialize round-trip 시 FileHeader compressed flag·raw_data 자체 변화 진단.
// 사용자 보고: rhwp 가 박은 hwp 가 한컴에서 손상 알람. 원인 자체 진단.

use std::fs;

fn dump_header(label: &str, bytes: &[u8]) {
    let cfb = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).unwrap();
    let mut cfb = cfb;
    let mut s = cfb.open_stream("/FileHeader").unwrap();
    let mut buf = vec![0u8; 256];
    use std::io::Read;
    s.read_exact(&mut buf).unwrap();
    let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
    println!(
        "  [{}] properties=0x{:08x} compressed={}",
        label,
        props,
        props & 0x01 != 0
    );
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/hwpx_roundtrip/baseline_business_table.hwp".to_string());
    let data = fs::read(&path).unwrap();
    println!("=== 원본 ===");
    dump_header("orig", &data);

    let doc = rhwp::parser::parse_document(&data).unwrap();
    println!("\n=== IR 자체 ===");
    println!(
        "  header.compressed={} flags=0x{:08x} raw_data={:?}",
        doc.header.compressed,
        doc.header.flags,
        doc.header.raw_data.as_ref().map(|v| v.len())
    );

    // serialize_document 자체 자체 호출
    let serialized = rhwp::serializer::serialize_document(&doc).unwrap();
    println!("\n=== serialize_document 결과 ===");
    dump_header("ser", &serialized);

    // DocumentCore::from_bytes → export_hwp_native 자체 호출 자체 자체
    let core = rhwp::document_core::DocumentCore::from_bytes(&data).unwrap();
    let core_out = core.export_hwp_native().unwrap();
    println!("\n=== DocumentCore.export_hwp_native ===");
    dump_header("core", &core_out);

    // HWPX 자체 자체 export → re-parse → export_hwp 자체 자체 변환 시뮬레이션
    let mut core2 = rhwp::document_core::DocumentCore::from_bytes(&data).unwrap();
    let hwpx_bytes = core2.export_hwpx_native().unwrap();
    println!("\n--- HWPX 거쳐서 자체 ---");
    println!("  hwpx bytes len={}", hwpx_bytes.len());
    let mut core3 = rhwp::document_core::DocumentCore::from_bytes(&hwpx_bytes).unwrap();
    println!(
        "  re-parse 자체 IR header.compressed={} flags=0x{:08x} raw_data={:?}",
        core3.document().header.compressed,
        core3.document().header.flags,
        core3.document().header.raw_data.as_ref().map(|v| v.len())
    );
    let final_out = core3.export_hwp_with_adapter().unwrap();
    println!("--- HWPX→HWP adapter 후 ---");
    dump_header("final", &final_out);
}
