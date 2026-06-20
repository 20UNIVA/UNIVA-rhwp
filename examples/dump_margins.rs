use std::fs;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let label = std::env::args().nth(2).unwrap_or("".to_string());
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    if let Some(sec) = doc.sections.first() {
        let pd = &sec.section_def.page_def;
        println!("[{}] pageDef width={} height={} margin: L={} R={} T={} B={} hdr={} ftr={} gutter={}",
            label, pd.width, pd.height,
            pd.margin_left, pd.margin_right, pd.margin_top, pd.margin_bottom,
            pd.margin_header, pd.margin_footer, pd.margin_gutter);
    }
    for (i, ps) in doc.doc_info.para_shapes.iter().enumerate().take(8) {
        println!("[{}] paraShape[{}] mgL={} mgR={} indent={}",
            label, i, ps.margin_left, ps.margin_right, ps.indent);
    }
}
