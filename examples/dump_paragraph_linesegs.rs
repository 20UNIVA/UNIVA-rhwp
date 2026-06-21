// 모든 paragraph 의 text + line_segs 자체 dump.
use std::fs;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let doc = rhwp::parser::parse_document(&fs::read(&path).unwrap()).unwrap();
    for (si, sec) in doc.sections.iter().enumerate() {
        for (pi, p) in sec.paragraphs.iter().enumerate() {
            let text_preview: String = p.text.chars().take(40).collect();
            println!(
                "s{}.p{} text={:?} char_count={} para_shape_id={} style_id={} line_segs={}",
                si, pi, text_preview, p.char_count, p.para_shape_id, p.style_id, p.line_segs.len()
            );
            for (li, ls) in p.line_segs.iter().enumerate() {
                println!(
                    "  ls[{}] text_start={} vert_pos={} line_height={} text_height={} baseline_dist={} line_spacing={} col_start={} seg_width={} tag=0x{:x}",
                    li, ls.text_start, ls.vertical_pos, ls.line_height, ls.text_height,
                    ls.baseline_distance, ls.line_spacing, ls.column_start, ls.segment_width, ls.tag
                );
            }
        }
    }

    println!("\n=== para_shapes ===");
    for (i, ps) in doc.doc_info.para_shapes.iter().enumerate() {
        println!(
            "  ps[{}] line_spacing={} spacing_before={} spacing_after={} margin_left={} margin_right={} indent={}",
            i, ps.line_spacing, ps.spacing_before, ps.spacing_after,
            ps.margin_left, ps.margin_right, ps.indent
        );
    }
}
