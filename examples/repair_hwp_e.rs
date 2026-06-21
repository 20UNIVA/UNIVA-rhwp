// repair_E — 원본 hwp 를 HWPX 거쳐서 다시 HWP 로 박은 결과 (사용자 시나리오 시뮬레이션).
// cycle 44 fix 적용된 코드 기준.

use std::fs;

fn main() {
    let orig_path = std::env::args().nth(1).expect("original.hwp");
    let out_path = std::env::args().nth(2).expect("output.hwp");

    let data = fs::read(&orig_path).unwrap();
    let core = rhwp::document_core::DocumentCore::from_bytes(&data).unwrap();
    let hwpx = core.export_hwpx_native().unwrap();
    let mut core2 = rhwp::document_core::DocumentCore::from_bytes(&hwpx).unwrap();
    let hwp = core2.export_hwp_with_adapter().unwrap();
    fs::write(&out_path, &hwp).unwrap();
    println!("→ {} bytes 저장: {}", hwp.len(), out_path);
}
