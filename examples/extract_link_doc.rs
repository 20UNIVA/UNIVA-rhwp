// saved/blank2010.hwp 에서 /DocOptions/_LinkDoc raw stream (압축 자체 그대로) 추출.

use std::fs;
use std::io::Read;

fn main() {
    let path = std::env::args().nth(1).expect("blank2010.hwp path");
    let out_link = std::env::args().nth(2).expect("output _LinkDoc.bin path");
    let out_summary = std::env::args().nth(3).expect("output hwp_summary.bin path");

    let mut cfb =
        cfb::CompoundFile::open(fs::File::open(&path).unwrap()).expect("cfb open");

    // _LinkDoc — raw (압축 그대로)
    let mut buf = Vec::new();
    cfb.open_stream("/DocOptions/_LinkDoc")
        .expect("open _LinkDoc")
        .read_to_end(&mut buf)
        .unwrap();
    println!("_LinkDoc raw: {} bytes (첫 16: {:?})", buf.len(), &buf[..16.min(buf.len())]);
    fs::write(&out_link, &buf).unwrap();

    // HwpSummaryInformation
    buf.clear();
    let summary_paths = ["/\u{0005}HwpSummaryInformation", "/HwpSummaryInformation"];
    let mut wrote = false;
    for p in summary_paths {
        if cfb.exists(p) {
            cfb.open_stream(p)
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            println!(
                "{}: {} bytes (첫 16: {:?})",
                p,
                buf.len(),
                &buf[..16.min(buf.len())]
            );
            fs::write(&out_summary, &buf).unwrap();
            wrote = true;
            break;
        }
    }
    if !wrote {
        println!("HwpSummaryInformation 없음");
    }
}
