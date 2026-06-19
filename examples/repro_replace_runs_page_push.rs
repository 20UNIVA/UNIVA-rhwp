// 재현: server flow 와 *동등하게* RunSpec → EditOperation → apply 경로로 호출.

use rhwp::document_core::{DocumentCore, EditOperation, RunSpec};
use std::path::PathBuf;

fn dump_para(core: &DocumentCore, sec: usize, p: usize, tag: &str) {
    let para = &core.document().sections[sec].paragraphs[p];
    let text_short: String = if para.text.chars().count() > 60 {
        format!("{}...", para.text.chars().take(60).collect::<String>())
    } else {
        para.text.clone()
    };
    println!(
        "  [{}] para {} text_len={} line_segs={} text=\"{}\"",
        tag, p, para.text.chars().count(), para.line_segs.len(), text_short
    );
    for (i, ls) in para.line_segs.iter().enumerate() {
        println!(
            "    ls[{}]: vpos={} lh={} th={} sw={}",
            i, ls.vertical_pos, ls.line_height, ls.text_height, ls.segment_width
        );
    }
    for (i, cs) in para.char_shapes.iter().enumerate() {
        println!(
            "    cs[{}]: start_pos={} char_shape_id={}",
            i, cs.start_pos, cs.char_shape_id
        );
    }
}

fn dump_page_layout(core: &DocumentCore, tag: &str) {
    println!("=== {} ===", tag);
    println!("  pages = {}", core.page_count());
    let pagination = &core.pagination()[0];
    for (page_idx, page) in pagination.pages.iter().enumerate().take(6) {
        let para_idxs: Vec<usize> = page
            .column_contents
            .iter()
            .flat_map(|col| col.items.iter().map(|it| it.para_index()))
            .collect();
        let para_min = para_idxs.iter().min().copied().unwrap_or(99);
        let para_max = para_idxs.iter().max().copied().unwrap_or(0);
        println!(
            "  page {}: paras {:?} (min={}, max={})",
            page_idx, para_idxs, para_min, para_max
        );
    }
}

fn apply_replace_runs(core: &mut DocumentCore, section: usize, para: usize, runs_payload: &str) {
    let runs: Vec<RunSpec> = serde_json::from_str(runs_payload).expect("RunSpec deserialize");
    let op = EditOperation::ReplaceRuns { section, para, runs };
    core.apply_edit_op(&op).expect("apply replace_runs");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "examples/(안내)대구테크노파크_소상공인 AI공급기업 모집_ 안내문  (1).hwp",
            )
        });
    let bytes = std::fs::read(&path).expect("read file");
    let mut core = DocumentCore::from_bytes(&bytes).expect("parse");

    dump_page_layout(&core, "초기 상태");
    println!();

    let runs_3 = r##"[{"text":"□ Overview of AI Supplier Company Recruitment ","style":{"font-name":"HY울릉도M","highlight":"#FFFFFF"}}]"##;
    apply_replace_runs(&mut core, 0, 3, runs_3);
    dump_page_layout(&core, "para 3 영문 교체 후");
    dump_para(&core, 0, 3, "after p3");
    println!();

    let runs_4 = r##"[{"text":" ◦ Recruiting AI mentor companies and AI supplier companies needed for close support, from AI model building to commercialization, to create new value and differentiated products and services for small businesses (demand companies) in the Daegu-Gyeongbuk region by utilizing AI.","style":{"font-name":"휴먼명조","font-size":13.0,"highlight":"#FFFFFF"}}]"##;
    apply_replace_runs(&mut core, 0, 4, runs_4);
    dump_page_layout(&core, "para 4 영문 교체 후");
    dump_para(&core, 0, 4, "after p4");
    println!();

    let runs_6 = r##"[{"text":"   - Providing phased support for AI utilization model building -> AI business model implementation by leveraging the expertise and know-how of private companies (AI startups, private platform companies, etc.).","style":{"font-name":"휴먼명조","font-size":13.0,"highlight":"#FFFFFF"}}]"##;
    apply_replace_runs(&mut core, 0, 6, runs_6);
    dump_page_layout(&core, "para 6 영문 교체 후");
    dump_para(&core, 0, 6, "after p6");
}
