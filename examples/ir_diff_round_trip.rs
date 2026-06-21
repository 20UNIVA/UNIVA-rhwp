// 원본 hwp → HWPX → 다시 HWP 자체 자체 *전체 IR 자체 자체 자체*.
// adapter 의 normalize/materialize 함수 자체 자체 자체 자체 자체 자체 자체 변경 검증.

use std::fs;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let data = fs::read(&path).unwrap();
    let doc_orig = rhwp::parser::parse_document(&data).unwrap();

    let core = rhwp::document_core::DocumentCore::from_bytes(&data).unwrap();
    let hwpx = core.export_hwpx_native().unwrap();
    let mut core2 = rhwp::document_core::DocumentCore::from_bytes(&hwpx).unwrap();
    let _hwp = core2.export_hwp_with_adapter().unwrap();
    let doc_rt = rhwp::parser::parse_document(&_hwp).unwrap();

    println!("=== doc_info record 개수 ===");
    println!("                  orig    rt");
    println!("bin_data_list:    {:5}  {:5}", doc_orig.doc_info.bin_data_list.len(), doc_rt.doc_info.bin_data_list.len());
    let fa_a: usize = doc_orig.doc_info.font_faces.iter().map(|v| v.len()).sum();
    let fa_b: usize = doc_rt.doc_info.font_faces.iter().map(|v| v.len()).sum();
    println!("font_faces:       {:5}  {:5}", fa_a, fa_b);
    println!("border_fills:     {:5}  {:5}", doc_orig.doc_info.border_fills.len(), doc_rt.doc_info.border_fills.len());
    println!("char_shapes:      {:5}  {:5}", doc_orig.doc_info.char_shapes.len(), doc_rt.doc_info.char_shapes.len());
    println!("para_shapes:      {:5}  {:5}", doc_orig.doc_info.para_shapes.len(), doc_rt.doc_info.para_shapes.len());
    println!("styles:           {:5}  {:5}", doc_orig.doc_info.styles.len(), doc_rt.doc_info.styles.len());
    println!("numberings:       {:5}  {:5}", doc_orig.doc_info.numberings.len(), doc_rt.doc_info.numberings.len());

    println!("\n=== border_fills 자료 diff ===");
    let mut bf_diff = 0;
    for i in 0..doc_orig.doc_info.border_fills.len().max(doc_rt.doc_info.border_fills.len()) {
        let a = doc_orig.doc_info.border_fills.get(i);
        let b = doc_rt.doc_info.border_fills.get(i);
        match (a, b) {
            (Some(a), Some(b)) => {
                if format!("{:?}", a.fill.fill_type) != format!("{:?}", b.fill.fill_type) {
                    println!("  bf[{}] orig.fill_type={:?} rt.fill_type={:?}", i, a.fill.fill_type, b.fill.fill_type);
                    bf_diff += 1;
                } else if a.fill.fill_type == rhwp::model::style::FillType::Solid {
                    let a_solid = a.fill.solid.as_ref().unwrap();
                    let b_solid = b.fill.solid.as_ref().unwrap();
                    if a_solid.background_color != b_solid.background_color
                        || a_solid.pattern_color != b_solid.pattern_color {
                        println!("  bf[{}] orig=Solid(bg=0x{:08x},pc=0x{:08x}) rt=Solid(bg=0x{:08x},pc=0x{:08x})",
                            i, a_solid.background_color, a_solid.pattern_color,
                            b_solid.background_color, b_solid.pattern_color);
                        bf_diff += 1;
                    }
                }
            }
            _ => {
                println!("  bf[{}] orig={:?} rt={:?}", i, a.is_some(), b.is_some());
                bf_diff += 1;
            }
        }
    }
    println!("border_fills diff: {} entries", bf_diff);

    println!("\n=== char_shapes raw_data byte diff ===");
    let mut cs_diff = 0;
    for i in 0..doc_orig.doc_info.char_shapes.len().min(doc_rt.doc_info.char_shapes.len()) {
        let a = &doc_orig.doc_info.char_shapes[i];
        let b = &doc_rt.doc_info.char_shapes[i];
        match (&a.raw_data, &b.raw_data) {
            (Some(ra), Some(rb)) => {
                if ra != rb {
                    cs_diff += 1;
                }
            }
            _ => {}
        }
    }
    println!("char_shapes raw_data 다른 자료: {} / {}", cs_diff, doc_orig.doc_info.char_shapes.len());

    println!("\n=== para_shapes raw_data byte diff ===");
    let mut ps_diff = 0;
    let mut bytes_first_diff: std::collections::BTreeMap<usize, u32> = Default::default();
    for i in 0..doc_orig.doc_info.para_shapes.len().min(doc_rt.doc_info.para_shapes.len()) {
        let a = &doc_orig.doc_info.para_shapes[i];
        let b = &doc_rt.doc_info.para_shapes[i];
        match (&a.raw_data, &b.raw_data) {
            (Some(ra), Some(rb)) => {
                if ra != rb {
                    ps_diff += 1;
                    for (k, (x, y)) in ra.iter().zip(rb.iter()).enumerate() {
                        if x != y {
                            *bytes_first_diff.entry(k).or_insert(0) += 1;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    println!("para_shapes raw_data 다른 자료: {} / {}", ps_diff, doc_orig.doc_info.para_shapes.len());
    println!("  첫 차이 byte 위치 빈도:");
    for (k, v) in &bytes_first_diff {
        println!("    byte {}: {} ps", k, v);
    }

    // 첫 para_shape byte diff
    if let (Some(ra), Some(rb)) = (
        doc_orig.doc_info.para_shapes[0].raw_data.as_ref(),
        doc_rt.doc_info.para_shapes[0].raw_data.as_ref(),
    ) {
        println!("\n=== ps[0] raw_data diff ===");
        println!("  orig size={} rt size={}", ra.len(), rb.len());
        let a32 = u32::from_le_bytes([ra[0], ra[1], ra[2], ra[3]]);
        let b32 = u32::from_le_bytes([rb[0], rb[1], rb[2], rb[3]]);
        println!("  orig attr1 = 0x{:08x}, rt attr1 = 0x{:08x}, xor = 0x{:08x}", a32, b32, a32 ^ b32);
        if ra.len() == rb.len() {
            for (i, (a, b)) in ra.iter().zip(rb.iter()).enumerate() {
                if a != b {
                    println!("  byte {}: orig=0x{:02x} rt=0x{:02x}", i, a, b);
                }
            }
        }
    }

    println!("\n=== Section paragraph counts ===");
    for (i, s) in doc_orig.sections.iter().enumerate() {
        let rt_count = doc_rt.sections.get(i).map(|s| s.paragraphs.len()).unwrap_or(0);
        println!("  s{} orig={} rt={}", i, s.paragraphs.len(), rt_count);
    }
}
