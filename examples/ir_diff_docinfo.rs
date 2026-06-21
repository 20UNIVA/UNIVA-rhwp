// 원본 hwp 자체 수정본 hwp 자체 자체 DocInfo IR 자체 비교.

use std::fs;
fn main() {
    let a = std::env::args().nth(1).unwrap();
    let b = std::env::args().nth(2).unwrap();
    let doc_a = rhwp::parser::parse_document(&fs::read(&a).unwrap()).unwrap();
    let doc_b = rhwp::parser::parse_document(&fs::read(&b).unwrap()).unwrap();
    let di_a = &doc_a.doc_info;
    let di_b = &doc_b.doc_info;
    println!("=== DocInfo IR 자체 비교 ===");
    println!("                 orig    modified");
    println!("bin_data_list:   {:6}  {:6}", di_a.bin_data_list.len(), di_b.bin_data_list.len());
    println!("font_faces:      {:6}  {:6}", di_a.font_faces.len(), di_b.font_faces.len());
    println!("border_fills:    {:6}  {:6}", di_a.border_fills.len(), di_b.border_fills.len());
    println!("char_shapes:     {:6}  {:6}", di_a.char_shapes.len(), di_b.char_shapes.len());
    println!("tab_defs:        {:6}  {:6}", di_a.tab_defs.len(), di_b.tab_defs.len());
    println!("numberings:      {:6}  {:6}", di_a.numberings.len(), di_b.numberings.len());
    println!("bullets:         {:6}  {:6}", di_a.bullets.len(), di_b.bullets.len());
    println!("para_shapes:     {:6}  {:6}", di_a.para_shapes.len(), di_b.para_shapes.len());
    println!("styles:          {:6}  {:6}", di_a.styles.len(), di_b.styles.len());

    let fa_rd_a: usize = di_a.font_faces.iter().map(|v| v.iter().filter(|f| f.raw_data.is_some()).count()).sum();
    let fa_rd_b: usize = di_b.font_faces.iter().map(|v| v.iter().filter(|f| f.raw_data.is_some()).count()).sum();
    let fa_len_a: usize = di_a.font_faces.iter().map(|v| v.len()).sum();
    let fa_len_b: usize = di_b.font_faces.iter().map(|v| v.len()).sum();
    let cs_rd_a = di_a.char_shapes.iter().filter(|c| c.raw_data.is_some()).count();
    let cs_rd_b = di_b.char_shapes.iter().filter(|c| c.raw_data.is_some()).count();
    let ps_rd_a = di_a.para_shapes.iter().filter(|p| p.raw_data.is_some()).count();
    let ps_rd_b = di_b.para_shapes.iter().filter(|p| p.raw_data.is_some()).count();
    let bf_rd_a = di_a.border_fills.iter().filter(|b| b.raw_data.is_some()).count();
    let bf_rd_b = di_b.border_fills.iter().filter(|b| b.raw_data.is_some()).count();
    let st_rd_a = di_a.styles.iter().filter(|s| s.raw_data.is_some()).count();
    let st_rd_b = di_b.styles.iter().filter(|s| s.raw_data.is_some()).count();

    println!("\n=== raw_data 보존 카운트 (Some) ===");
    println!("                      orig                modified");
    println!("font_faces.raw_data:  {:>5}/{:<5}        {:>5}/{:<5}",
        fa_rd_a, fa_len_a, fa_rd_b, fa_len_b);
    println!("char_shapes.raw_data: {:>5}/{:<5}        {:>5}/{:<5}",
        cs_rd_a, di_a.char_shapes.len(), cs_rd_b, di_b.char_shapes.len());
    println!("para_shapes.raw_data: {:>5}/{:<5}        {:>5}/{:<5}",
        ps_rd_a, di_a.para_shapes.len(), ps_rd_b, di_b.para_shapes.len());
    println!("border_fills.raw_data:{:>5}/{:<5}        {:>5}/{:<5}",
        bf_rd_a, di_a.border_fills.len(), bf_rd_b, di_b.border_fills.len());
    println!("styles.raw_data:      {:>5}/{:<5}        {:>5}/{:<5}",
        st_rd_a, di_a.styles.len(), st_rd_b, di_b.styles.len());

    // raw_data total bytes
    let cs_bytes_a: usize = di_a.char_shapes.iter().filter_map(|c| c.raw_data.as_ref().map(|v| v.len())).sum();
    let cs_bytes_b: usize = di_b.char_shapes.iter().filter_map(|c| c.raw_data.as_ref().map(|v| v.len())).sum();
    let ps_bytes_a: usize = di_a.para_shapes.iter().filter_map(|p| p.raw_data.as_ref().map(|v| v.len())).sum();
    let ps_bytes_b: usize = di_b.para_shapes.iter().filter_map(|p| p.raw_data.as_ref().map(|v| v.len())).sum();
    let bf_bytes_a: usize = di_a.border_fills.iter().filter_map(|b| b.raw_data.as_ref().map(|v| v.len())).sum();
    let bf_bytes_b: usize = di_b.border_fills.iter().filter_map(|b| b.raw_data.as_ref().map(|v| v.len())).sum();
    let st_bytes_a: usize = di_a.styles.iter().filter_map(|s| s.raw_data.as_ref().map(|v| v.len())).sum();
    let st_bytes_b: usize = di_b.styles.iter().filter_map(|s| s.raw_data.as_ref().map(|v| v.len())).sum();
    let fa_bytes_a: usize = di_a.font_faces.iter().flat_map(|v| v.iter()).filter_map(|f| f.raw_data.as_ref().map(|v| v.len())).sum();
    let fa_bytes_b: usize = di_b.font_faces.iter().flat_map(|v| v.iter()).filter_map(|f| f.raw_data.as_ref().map(|v| v.len())).sum();
    println!("\n=== raw_data total bytes ===");
    println!("                 orig    modified");
    println!("font_faces:    {:6}    {:6}", fa_bytes_a, fa_bytes_b);
    println!("char_shapes:   {:6}    {:6}", cs_bytes_a, cs_bytes_b);
    println!("para_shapes:   {:6}    {:6}", ps_bytes_a, ps_bytes_b);
    println!("border_fills:  {:6}    {:6}", bf_bytes_a, bf_bytes_b);
    println!("styles:        {:6}    {:6}", st_bytes_a, st_bytes_b);

    // 첫 char_shape, para_shape, border_fill raw_data 비교
    if let (Some(ra), Some(rb)) = (
        di_a.char_shapes[0].raw_data.as_ref(),
        di_b.char_shapes[0].raw_data.as_ref(),
    ) {
        println!("\nchar_shapes[0] raw_data: orig={} mod={} same={}",
            ra.len(), rb.len(), ra == rb);
        println!("  orig: {}", ra.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        println!("  mod : {}", rb.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        // diff 첫 위치
        for (i, (a, b)) in ra.iter().zip(rb.iter()).enumerate() {
            if a != b {
                println!("  ⚠ 첫 차이 byte {}: orig=0x{:02x} mod=0x{:02x}", i, a, b);
                break;
            }
        }
    }
    if let (Some(ra), Some(rb)) = (
        di_a.para_shapes[0].raw_data.as_ref(),
        di_b.para_shapes[0].raw_data.as_ref(),
    ) {
        println!("para_shapes[0] raw_data: orig={} mod={} same={}",
            ra.len(), rb.len(), ra == rb);
    }
    if let (Some(ra), Some(rb)) = (
        di_a.border_fills[0].raw_data.as_ref(),
        di_b.border_fills[0].raw_data.as_ref(),
    ) {
        println!("border_fills[0] raw_data: orig={} mod={} same={}",
            ra.len(), rb.len(), ra == rb);
    }
    if let (Some(ra), Some(rb)) = (
        di_a.styles[0].raw_data.as_ref(),
        di_b.styles[0].raw_data.as_ref(),
    ) {
        println!("styles[0] raw_data: orig={} mod={} same={}",
            ra.len(), rb.len(), ra == rb);
    }

    println!("\n=== Section paragraph 자체 비교 ===");
    for (i, s) in doc_a.sections.iter().enumerate() {
        if let Some(sb) = doc_b.sections.get(i) {
            println!("  s{} orig={} mod={}", i, s.paragraphs.len(), sb.paragraphs.len());
        }
    }
}
