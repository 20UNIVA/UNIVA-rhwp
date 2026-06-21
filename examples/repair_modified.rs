// rhwp_modified.hwp 의 사용자 편집 내용을 보존하면서 cycle 42~46 fix 가 적용된
// 파이프라인으로 다시 박아 한컴이 열 수 있는 hwp 생성.
//
// 단계:
//   1. rhwp_modified parse → IR (사용자 편집 내용 포함)
//   2. 원본 hwp 의 Preview 자체 가져와 IR 에 박음 (cycle 43 fix 대비 보강)
//   3. HWPX export → re-parse → adapter → 새 hwp
//   4. cycle 44 (PARA_HEADER 24), 45 (SectionDef 47), 46 (shadow attr) 자체 적용

use std::fs;

fn main() {
    let modified_path = std::env::args().nth(1).expect("modified.hwp path");
    let original_path = std::env::args().nth(2).expect("original.hwp path");
    let output_path = std::env::args().nth(3).expect("output.hwp path");

    let modified_data = fs::read(&modified_path).unwrap();
    let original_data = fs::read(&original_path).unwrap();

    // 1. 수정본 → DocumentCore → HWPX export → re-parse
    //    HWPX-style IR 으로 만들면 raw_stream·raw_ctrl_extra·raw_header_extra·raw_ctrl_data
    //    등 모든 raw_* 자료가 비어 있어 cycle 44·45·46 fix 자체 적용됨.
    let core = rhwp::document_core::DocumentCore::from_bytes(&modified_data).unwrap();
    let hwpx_bytes = core.export_hwpx_native().unwrap();
    let mut doc = rhwp::parser::parse_document(&hwpx_bytes).unwrap();

    // 2. 원본 hwp 의 Preview·extra_streams 자체 박음 (cycle 43 fix 보강).
    let orig_doc = rhwp::parser::parse_document(&original_data).unwrap();
    doc.preview = orig_doc.preview.clone();
    doc.extra_streams = orig_doc.extra_streams.clone();

    println!(
        "최종 IR Preview:  image={:?} bytes",
        doc.preview.as_ref().and_then(|p| p.image.as_ref().map(|i| i.data.len()))
    );
    println!("최종 extra_streams: {} entries", doc.extra_streams.len());

    // 3. HWPX → HWP IR adapter (FileHeader compressed normalize 포함)
    use rhwp::document_core::converters::hwpx_to_hwp::convert_if_hwpx_source;
    use rhwp::parser::FileFormat;
    let _report = convert_if_hwpx_source(&mut doc, FileFormat::Hwpx);

    // 4. serialize_document — cycle 44·45·46 fix 자체 자체 자체 자체 자체
    let bytes = rhwp::serializer::serialize_document(&doc).unwrap();
    fs::write(&output_path, &bytes).unwrap();
    println!("\n→ {} bytes 저장: {}", bytes.len(), output_path);
}
