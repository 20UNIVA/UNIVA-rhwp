use std::fs;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = &args[1];
    let dst = &args[2];
    let data = fs::read(src).expect("read src");
    let doc = rhwp::parser::parse_document(&data).expect("parse");
    // 표 raw_ctrl_data 길이 보고
    for (si, sec) in doc.sections.iter().enumerate() {
        for (pi, para) in sec.paragraphs.iter().enumerate() {
            for (ci, ctrl) in para.controls.iter().enumerate() {
                if let rhwp::model::control::Control::Table(t) = ctrl {
                    eprintln!("[parsed] s{}.p{}.c{} table raw_ctrl_data.len={} outer_margin=({},{},{},{}) size=({}x{}) wrap={:?} tac={} vrt={:?} hrt={:?} pb={:?}",
                        si, pi, ci, t.raw_ctrl_data.len(),
                        t.outer_margin_left, t.outer_margin_right, t.outer_margin_top, t.outer_margin_bottom,
                        t.common.width, t.common.height, t.common.text_wrap, t.common.treat_as_char,
                        t.common.vert_rel_to, t.common.horz_rel_to, t.page_break);
                }
            }
        }
    }
    let bytes = rhwp::serialize_document(&doc).expect("serialize");
    fs::write(dst, &bytes).expect("write dst");
    eprintln!("OK: wrote {} bytes -> {}", bytes.len(), dst);
}
