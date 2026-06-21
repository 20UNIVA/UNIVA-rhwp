// FileHeader 256 byte 의 압축 플래그·버전·encrypt 자료 dump.
// HWP5 spec §3.1.1: FileHeader (256 bytes)
//   offset 0..32: signature "HWP Document File\0\0\0..."
//   offset 32..36: version (uint32 LE)
//   offset 36..40: properties (bit 0 = compressed, bit 1 = encrypted, bit 2 = distributed)
//   ... 나머지 reserved

use std::fs::File;
use std::io::Read;

fn dump(label: &str, path: &str) {
    let mut cfb = cfb::CompoundFile::open(File::open(path).unwrap()).unwrap();
    let mut s = cfb.open_stream("/FileHeader").unwrap();
    let mut buf = vec![0u8; 256];
    s.read_exact(&mut buf).unwrap();

    let sig = String::from_utf8_lossy(&buf[..32]);
    let version = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
    let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
    println!("=== {} ===", label);
    println!("  signature: {:?}", sig.trim_end_matches('\0'));
    println!(
        "  version:   {}.{}.{}.{}",
        (version >> 24) & 0xff,
        (version >> 16) & 0xff,
        (version >> 8) & 0xff,
        version & 0xff
    );
    println!("  properties: 0x{:08x}", props);
    println!("    compressed:  {}", props & 0x01 != 0);
    println!("    encrypted:   {}", props & 0x02 != 0);
    println!("    distributed: {}", props & 0x04 != 0);
    println!("    script:      {}", props & 0x08 != 0);
    println!("    drm:         {}", props & 0x10 != 0);
    println!("    xml-template:{}", props & 0x20 != 0);
    println!("    history:     {}", props & 0x40 != 0);
    println!("    sign:        {}", props & 0x80 != 0);
    println!("    cert-encrypt:{}", props & 0x100 != 0);
    println!("    sign-spare:  {}", props & 0x200 != 0);
    println!("    cert-drm:    {}", props & 0x400 != 0);
    println!("    ccl:         {}", props & 0x800 != 0);
}

fn main() {
    let a = std::env::args().nth(1).unwrap();
    let b = std::env::args().nth(2).unwrap();
    dump("orig", &a);
    println!();
    dump("modified", &b);
}
