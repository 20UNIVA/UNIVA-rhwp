// HWP 파일의 CFB stream 비교. orig vs modified 두 hwp 파일의 stream 목록과
// 각 stream 크기 + 첫 32 byte hex 를 dump.

use std::fs::File;
use std::path::Path;

fn dump(label: &str, path: &Path) {
    println!("=== {} ({}) ===", label, path.display());
    let file = File::open(path).expect("open");
    let mut cfb = cfb::CompoundFile::open(file).expect("cfb open");
    let mut paths: Vec<String> = cfb
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    paths.sort();
    for p in paths {
        let entry = cfb.entry(&p).expect("entry");
        let size = entry.len();
        // 첫 32 byte 읽기
        let mut buf = vec![0u8; 32.min(size as usize)];
        if size > 0 {
            use std::io::Read;
            let mut s = cfb.open_stream(&p).expect("open stream");
            let _ = s.read(&mut buf);
        }
        let hex: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
        println!("  {:50} {:>8}  {}", p, size, hex);
    }
}

fn main() {
    let orig = std::env::args().nth(1).expect("orig path");
    let modd = std::env::args().nth(2).expect("modified path");
    dump("orig", Path::new(&orig));
    println!();
    dump("modified", Path::new(&modd));
}
