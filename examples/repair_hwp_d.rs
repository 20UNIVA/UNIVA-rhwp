// repair_D — 수정본 IR 자체 *전체 doc_info record 자체 자체 자체 원본에서 통째 복사*.
// HWPX → HWP IR 변환 자체 자체 자체 record raw_data 자체 자체 자체 자체 한컴 손상 알람의
// 원인인지 확정 검증.

use std::fs;

fn main() {
    let modified = std::env::args().nth(1).expect("modified.hwp");
    let original = std::env::args().nth(2).expect("original.hwp");
    let output = std::env::args().nth(3).expect("output.hwp");

    let mod_data = fs::read(&modified).unwrap();
    let orig_data = fs::read(&original).unwrap();

    let mut doc_mod = rhwp::parser::parse_document(&mod_data).unwrap();
    let doc_orig = rhwp::parser::parse_document(&orig_data).unwrap();

    // FileHeader normalize (cycle 42)
    doc_mod.header.compressed = true;
    doc_mod.header.flags |= 0x01;
    doc_mod.header.raw_data = None;

    // Preview·extra_streams 자체 자체 (cycle 43·44)
    doc_mod.preview = doc_orig.preview.clone();
    doc_mod.extra_streams = doc_orig.extra_streams.clone();

    // doc_info record 자체 자체 *통째 이식*. record 개수가 일치해야 하므로 길이 검증.
    let di_m = &mut doc_mod.doc_info;
    let di_o = &doc_orig.doc_info;

    assert_eq!(di_m.font_faces.len(), di_o.font_faces.len());
    assert_eq!(di_m.char_shapes.len(), di_o.char_shapes.len());
    assert_eq!(di_m.para_shapes.len(), di_o.para_shapes.len());
    assert_eq!(di_m.border_fills.len(), di_o.border_fills.len());
    assert_eq!(di_m.styles.len(), di_o.styles.len());

    for (lang_m, lang_o) in di_m.font_faces.iter_mut().zip(di_o.font_faces.iter()) {
        for (fm, fo) in lang_m.iter_mut().zip(lang_o.iter()) {
            fm.raw_data = fo.raw_data.clone();
        }
    }
    for (cm, co) in di_m.char_shapes.iter_mut().zip(di_o.char_shapes.iter()) {
        cm.raw_data = co.raw_data.clone();
    }
    for (pm, po) in di_m.para_shapes.iter_mut().zip(di_o.para_shapes.iter()) {
        pm.raw_data = po.raw_data.clone();
    }
    for (bm, bo) in di_m.border_fills.iter_mut().zip(di_o.border_fills.iter()) {
        bm.raw_data = bo.raw_data.clone();
    }
    for (sm, so) in di_m.styles.iter_mut().zip(di_o.styles.iter()) {
        sm.raw_data = so.raw_data.clone();
    }
    // doc_properties raw_data 자체
    di_m.bin_data_list = di_o.bin_data_list.clone();

    println!("=== doc_info raw_data 이식 완료 ===");

    let bytes = rhwp::serializer::serialize_document(&doc_mod).unwrap();
    fs::write(&output, &bytes).unwrap();
    println!("→ {} bytes 저장: {}", bytes.len(), output);
}
