// Task #m600-42 fix 시뮬레이션 — 손상된 hwp 의 FileHeader 를 normalize 한 뒤 재직렬화.
//
// 사용자 보고 시나리오 검증: rhwp-studio 가 박은 hwp 가 한컴에서 손상 알람.
// 본 example 은 그 hwp 를 cycle 42 의 normalize_file_header_for_hwp 자료와 동등하게
// 수정해 결과 hwp 가 한컴에서 열리는지 직접 검증할 수 있게 한다.
//
// usage:
//   repair_hwp <input.hwp> <output.hwp>                  — compressed-only fix
//   repair_hwp <input.hwp> <output.hwp> <original.hwp>   — compressed + Preview restore

use std::fs;

fn main() {
    let input = std::env::args().nth(1).expect("input path");
    let output = std::env::args().nth(2).expect("output path");
    let original = std::env::args().nth(3);

    let data = fs::read(&input).unwrap();
    let mut doc = rhwp::parser::parse_document(&data).unwrap();

    println!("=== 입력 IR ===");
    println!(
        "  compressed={} flags=0x{:08x} raw_data={:?}",
        doc.header.compressed,
        doc.header.flags,
        doc.header.raw_data.as_ref().map(|v| v.len())
    );
    println!(
        "  preview image len={:?} text len={:?}",
        doc.preview
            .as_ref()
            .and_then(|p| p.image.as_ref().map(|i| i.data.len())),
        doc.preview.as_ref().and_then(|p| p.text.as_ref().map(|t| t.len()))
    );

    // Task #m600-42 의 normalize_file_header_for_hwp 자료와 동등.
    doc.header.compressed = true;
    if doc.header.flags & 0x01 == 0 {
        doc.header.flags |= 0x01;
    }
    doc.header.raw_data = None;

    // optional: 원본 Preview 복원 (cycle 43 자료 시뮬레이션, HWP→HWP 경로엔 영향 없지만
    // PrvImage·PrvText 정합 검증 자료).
    if let Some(orig_path) = original {
        let orig_data = fs::read(&orig_path).unwrap();
        let orig_doc = rhwp::parser::parse_document(&orig_data).unwrap();
        doc.preview = orig_doc.preview;
        // Task #m600-44 — 원본 extra_streams (HwpSummary·_LinkDoc·Scripts) 통째 이식.
        // 수정본은 HwpSummary 가 fallback 461 bytes 로 박혀 원본 473 bytes 와 다름.
        doc.extra_streams = orig_doc.extra_streams;
        println!("  [원본 Preview + extra_streams 복원]");
        println!(
            "  preview image len={:?} text len={:?}",
            doc.preview
                .as_ref()
                .and_then(|p| p.image.as_ref().map(|i| i.data.len())),
            doc.preview.as_ref().and_then(|p| p.text.as_ref().map(|t| t.len()))
        );
        println!("  extra_streams: {} entries", doc.extra_streams.len());
        for (path, data) in &doc.extra_streams {
            println!("    {} : {} bytes", path, data.len());
        }
    }

    println!("\n=== normalize 후 IR ===");
    println!(
        "  compressed={} flags=0x{:08x} raw_data={:?}",
        doc.header.compressed,
        doc.header.flags,
        doc.header.raw_data.as_ref().map(|v| v.len())
    );

    let bytes = rhwp::serializer::serialize_document(&doc).unwrap();
    fs::write(&output, &bytes).unwrap();
    println!("\n→ {} bytes 저장: {}", bytes.len(), output);
}
