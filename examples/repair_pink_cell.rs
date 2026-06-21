// pink_cell.hwp 의 사용자 편집(4행 3열 cell 분홍 배경)을 보존하면서 cycle 42~50 fix
// 가 적용된 파이프라인으로 다시 박는다.
//
// pink_cell.hwp 의 IR 은 *과거 버전 rhwp 가 박은 hwp* 라 doc_info.border_fills 의
// 일부 항목이 None 으로 손상되어 있다. 손상된 항목만 원본 hwp 의 자료로 복원한다.
// 사용자가 추가한 항목(bf[16] = 분홍색 Solid)은 그대로 유지된다.

use std::fs;

fn main() {
    let pink_path = std::env::args().nth(1).expect("pink_cell.hwp");
    let orig_path = std::env::args().nth(2).expect("original.hwp");
    let out_path = std::env::args().nth(3).expect("output.hwp");

    let pink_data = fs::read(&pink_path).unwrap();
    let orig_data = fs::read(&orig_path).unwrap();

    // 1) pink_cell IR — 사용자 편집(table cell.border_fill_id=17 등)
    let mut doc = rhwp::parser::parse_document(&pink_data).unwrap();
    let orig_doc = rhwp::parser::parse_document(&orig_data).unwrap();

    // 2) doc_info.border_fills 손상 복원: pink_cell 에서 None 박혀 있는 항목 중
    //    원본에 Solid/Gradient 박혀 있는 항목은 원본 자료로 교체.
    use rhwp::model::style::FillType;
    let mut restored_bf = 0;
    for i in 0..doc.doc_info.border_fills.len().min(orig_doc.doc_info.border_fills.len()) {
        let pink_is_none = doc.doc_info.border_fills[i].fill.fill_type == FillType::None;
        let orig_is_solid = orig_doc.doc_info.border_fills[i].fill.fill_type != FillType::None;
        if pink_is_none && orig_is_solid {
            doc.doc_info.border_fills[i] = orig_doc.doc_info.border_fills[i].clone();
            restored_bf += 1;
        }
    }
    println!("border_fills 손상 복원: {} 항목", restored_bf);

    // 3) char_shapes raw_data 복원 (cycle 46 fix 이전에 박힌 hwp 라 일부 char_shape 의
    //    shadow_offset·shadow_color 자료가 0 으로 사라져 있을 수 있다. 원본 자료가 있으면
    //    그쪽이 정확하다).
    let mut restored_cs = 0;
    for i in 0..doc.doc_info.char_shapes.len().min(orig_doc.doc_info.char_shapes.len()) {
        if let (Some(_), Some(orig_raw)) = (
            doc.doc_info.char_shapes[i].raw_data.as_ref(),
            orig_doc.doc_info.char_shapes[i].raw_data.as_ref(),
        ) {
            doc.doc_info.char_shapes[i].raw_data = Some(orig_raw.clone());
            restored_cs += 1;
        }
    }
    println!("char_shapes raw_data 원본 복원: {} 항목", restored_cs);

    // 4) para_shapes raw_data 복원 (cycle 47·50 fix 이전 손실 보강)
    let mut restored_ps = 0;
    for i in 0..doc.doc_info.para_shapes.len().min(orig_doc.doc_info.para_shapes.len()) {
        if let Some(orig_raw) = orig_doc.doc_info.para_shapes[i].raw_data.as_ref() {
            doc.doc_info.para_shapes[i].raw_data = Some(orig_raw.clone());
            restored_ps += 1;
        }
    }
    println!("para_shapes raw_data 원본 복원: {} 항목", restored_ps);

    // 5) Preview·extra_streams 원본 자료 (cycle 43 보강)
    doc.preview = orig_doc.preview.clone();
    doc.extra_streams = orig_doc.extra_streams.clone();

    // 6) FileHeader normalize (cycle 42)
    doc.header.compressed = true;
    doc.header.flags |= 0x01;
    doc.header.raw_data = None;

    // 7) raw_stream None 박아 본문 record 가 새로 박히도록 (cycle 44·45·48 fix 적용 경로)
    doc.doc_info.raw_stream = None;
    for sec in &mut doc.sections {
        sec.raw_stream = None;
    }

    // 8) 분홍 cell 안 글자 영역에 흰 박스가 박힌 원인은 paragraph 배경(ps.border_fill_id
    //    가 가리키는 bf[1] = Solid 흰색)이다. cs 의 글자 음영(border_fill_id) 만 0 박아도
    //    paragraph 배경 자체가 흰색이라 분홍 cell 위에 흰 박스가 그대로 남는다.
    //    4행 3열 table 안 cell 의 paragraphs 가 가리키는 *paragraph shape 의 border_fill_id*
    //    가 Solid 흰색·흰색+무늬 bf 를 가리키면 0(배경 없음) 박기.
    //    paragraph 의 para_shape_id 가 다른 자리에서도 공유되더라도 paragraph 배경 변경은
    //    한컴 원본에서 흰 배경 박힌 자리만 영향(시각 차이 거의 없음).
    use rhwp::model::control::Control;
    let mut referenced_ps_ids: std::collections::HashSet<u16> = Default::default();
    let mut referenced_cs_ids: std::collections::HashSet<u32> = Default::default();
    for sec in &doc.sections {
        for p in &sec.paragraphs {
            for ctrl in &p.controls {
                if let Control::Table(t) = ctrl {
                    if t.row_count == 4 && t.col_count == 3 {
                        for cell in &t.cells {
                            for cp in &cell.paragraphs {
                                referenced_ps_ids.insert(cp.para_shape_id);
                                for cs_ref in &cp.char_shapes {
                                    referenced_cs_ids.insert(cs_ref.char_shape_id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let is_solid_white = |bf_id: u16, bfs: &[rhwp::model::style::BorderFill]| -> bool {
        if bf_id == 0 { return false; }
        let bf_idx = (bf_id as usize).saturating_sub(1);
        bfs.get(bf_idx).map(|bf| {
            bf.fill.fill_type == FillType::Solid
                && bf.fill.solid.as_ref()
                    .map(|s| (s.background_color & 0x00FFFFFF) == 0x00FFFFFF)
                    .unwrap_or(false)
        }).unwrap_or(false)
    };

    let mut cs_zeroed = 0;
    for &cs_id in &referenced_cs_ids {
        let bfs_snapshot = doc.doc_info.border_fills.clone();
        let cs = match doc.doc_info.char_shapes.get_mut(cs_id as usize) {
            Some(c) => c,
            None => continue,
        };
        if is_solid_white(cs.border_fill_id, &bfs_snapshot) {
            cs.border_fill_id = 0;
            cs.raw_data = None;
            cs_zeroed += 1;
        }
    }
    println!("4x3 cell 안 char_shape 흰 음영 제거: {} 항목", cs_zeroed);

    let mut ps_zeroed = 0;
    for &ps_id in &referenced_ps_ids {
        let bfs_snapshot = doc.doc_info.border_fills.clone();
        let ps = match doc.doc_info.para_shapes.get_mut(ps_id as usize) {
            Some(p) => p,
            None => continue,
        };
        if is_solid_white(ps.border_fill_id, &bfs_snapshot) {
            ps.border_fill_id = 0;
            ps.raw_data = None;
            ps_zeroed += 1;
        }
    }
    println!("4x3 cell 안 paragraph 흰 배경 제거: {} 항목", ps_zeroed);

    // 8) serialize_document — HWPX 우회 (HWPX 거치면 cell.border_fill_id 등이 다시 처리됨)
    let bytes = rhwp::serializer::serialize_document(&doc).unwrap();
    fs::write(&out_path, &bytes).unwrap();
    println!("\n→ {} bytes 저장: {}", bytes.len(), out_path);
}
