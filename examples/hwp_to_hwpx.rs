use std::fs;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let dst = std::env::args().nth(2).unwrap();
    let data = fs::read(&src).unwrap();
    let doc = rhwp::parser::parse_document(&data).unwrap();
    let bytes = rhwp::serializer::serialize_hwpx(&doc).unwrap();
    fs::write(&dst, &bytes).unwrap();
    eprintln!("ok");
}
