use std::fs;
fn main() {
    let doc = rhwp::parser::parse_document(&fs::read(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
    for id in [20, 21] {
        let ps = &doc.doc_info.para_shapes[id];
        println!("ps[{}]: line_spacing={} line_spacing_type={:?} line_spacing_v2={} spacing_before={} spacing_after={} attr={} attr2={} attr3={}",
            id, ps.line_spacing, ps.line_spacing_type, ps.line_spacing_v2,
            ps.spacing_before, ps.spacing_after, ps.attr1, ps.attr2, ps.attr3);
    }
}
