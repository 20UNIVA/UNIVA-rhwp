//! IR slice 크기/형태 분석 도구. 단일 hwp/hwpx 입력에 대해 페이지별 IR slice 를
//! 여러 detail/style/table 조합으로 만들어 보고 JSON 크기·문자 수를 비교한다.
//! `cargo run --bin analyze_ir -- <path> [page]` 형태로 호출.
//!
//! ir_compact 는 rhwp-server main 바이너리의 private mod 이므로 path 속성으로 직접
//! 가져온다 — 새 lib crate 를 만들지 않고 분석만 진행하기 위함.

#[path = "../ir_compact.rs"]
mod ir_compact;

use ir_compact::{
    apply_style_filter, apply_table_filter, build_compact_ir_slice, build_outline_slice,
    build_structure_slice, BuildOptions, Detail, StyleLevel, TableLevel,
};
use rhwp::DocumentCore;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

fn parse_args() -> (PathBuf, u32) {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: analyze_ir <file> [page]");
        std::process::exit(2);
    }
    let path = PathBuf::from(&args[1]);
    let page: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    (path, page)
}

fn run_combo(
    core: &DocumentCore,
    page: u32,
    detail: Detail,
    style: StyleLevel,
    tables: TableLevel,
    max_text: Option<u32>,
) -> (String, usize, usize) {
    let opts = BuildOptions {
        sec: 0,
        para_start: 0,
        para_end: None,
        edit_session_id: Some("analyze".into()),
        page: if page >= 1 { Some(page - 1) } else { None },
        page_override_range: None,
        total_pages_override: None,
        detail,
        include_style: style,
        include_tables: tables,
        max_text_chars: max_text,
    };

    let label = format!(
        "detail={:?} style={:?} tables={:?} max_text={:?}",
        detail, style, tables, max_text
    );

    let v: Value = match detail {
        Detail::Compact | Detail::Raw => {
            let slice = build_compact_ir_slice(core, &opts);
            let mut v = serde_json::to_value(&slice).unwrap_or(Value::Null);
            if let Value::Object(ref mut m) = v {
                if let Some(Value::Array(arr)) = m.get_mut("paragraphs") {
                    apply_table_filter(arr, tables);
                    apply_style_filter(arr, style);
                }
            }
            v
        }
        Detail::Outline => build_outline_slice(core, &opts),
        Detail::Structure => build_structure_slice(core, &opts),
    };

    let pretty = serde_json::to_string_pretty(&v).unwrap_or_default();
    let raw = serde_json::to_string(&v).unwrap_or_default();
    (label, pretty.len(), raw.len())
}

fn dump_combo(
    core: &DocumentCore,
    page: u32,
    detail: Detail,
    style: StyleLevel,
    tables: TableLevel,
    max_text: Option<u32>,
    out_path: &str,
) {
    let opts = BuildOptions {
        sec: 0,
        para_start: 0,
        para_end: None,
        edit_session_id: Some("analyze".into()),
        page: if page >= 1 { Some(page - 1) } else { None },
        page_override_range: None,
        total_pages_override: None,
        detail,
        include_style: style,
        include_tables: tables,
        max_text_chars: max_text,
    };
    let v: Value = match detail {
        Detail::Compact | Detail::Raw => {
            let slice = build_compact_ir_slice(core, &opts);
            let mut v = serde_json::to_value(&slice).unwrap_or(Value::Null);
            if let Value::Object(ref mut m) = v {
                if let Some(Value::Array(arr)) = m.get_mut("paragraphs") {
                    apply_table_filter(arr, tables);
                    apply_style_filter(arr, style);
                }
            }
            v
        }
        Detail::Outline => build_outline_slice(core, &opts),
        Detail::Structure => build_structure_slice(core, &opts),
    };
    let pretty = serde_json::to_string_pretty(&v).unwrap_or_default();
    fs::write(out_path, &pretty).unwrap();
}

fn main() {
    let (path, page) = parse_args();
    let bytes = fs::read(&path).expect("입력 파일 읽기 실패");
    let core = DocumentCore::from_bytes(&bytes).expect("문서 파싱 실패");

    let total_pages: u32 = core.pagination().iter().map(|p| p.pages.len() as u32).sum();
    println!(
        "문서 로드: {} (총 {} 페이지) — 분석 대상 페이지: {}",
        path.display(),
        total_pages,
        page
    );
    println!();

    // 다양한 조합 — 페이지 1건 기준 크기 비교.
    let combos = [
        (Detail::Compact, StyleLevel::Full, TableLevel::Full, None),
        (Detail::Compact, StyleLevel::Essential, TableLevel::Full, None),
        (Detail::Compact, StyleLevel::None, TableLevel::Full, None),
        (Detail::Compact, StyleLevel::Essential, TableLevel::Structure, None),
        (Detail::Compact, StyleLevel::None, TableLevel::Structure, None),
        (Detail::Compact, StyleLevel::Essential, TableLevel::Count, None),
        (Detail::Compact, StyleLevel::None, TableLevel::Count, None),
        (Detail::Outline, StyleLevel::Essential, TableLevel::Full, None),
        (Detail::Outline, StyleLevel::None, TableLevel::Count, None),
        (Detail::Outline, StyleLevel::None, TableLevel::Count, Some(60)),
        (Detail::Structure, StyleLevel::None, TableLevel::Full, None),
        (Detail::Structure, StyleLevel::None, TableLevel::Count, None),
    ];

    println!(
        "{:<70} {:>10} {:>10}",
        "조합", "pretty", "min(no ws)"
    );
    println!("{}", "-".repeat(94));
    for (d, s, t, m) in combos {
        let (label, pretty_len, min_len) = run_combo(&core, page, d, s, t, m);
        println!("{:<70} {:>10} {:>10}", label, pretty_len, min_len);
    }

    // 대표 4 조합 sample 파일 저장 — 사용자가 직접 형태 비교.
    let outdir = "output/ir-analysis";
    fs::create_dir_all(outdir).ok();
    dump_combo(
        &core,
        page,
        Detail::Compact,
        StyleLevel::Full,
        TableLevel::Full,
        None,
        &format!("{outdir}/page{page}_compact_full.json"),
    );
    dump_combo(
        &core,
        page,
        Detail::Compact,
        StyleLevel::None,
        TableLevel::Structure,
        None,
        &format!("{outdir}/page{page}_compact_no_style_struct.json"),
    );
    dump_combo(
        &core,
        page,
        Detail::Outline,
        StyleLevel::None,
        TableLevel::Count,
        Some(60),
        &format!("{outdir}/page{page}_outline_thin.json"),
    );
    dump_combo(
        &core,
        page,
        Detail::Structure,
        StyleLevel::None,
        TableLevel::Count,
        None,
        &format!("{outdir}/page{page}_structure.json"),
    );
    dump_combo(
        &core,
        page,
        Detail::Compact,
        StyleLevel::None,
        TableLevel::Full,
        None,
        &format!("{outdir}/page{page}_compact_no_style_full_tables.json"),
    );
    println!();
    println!("4개 대표 조합 dump → {outdir}/");
}
