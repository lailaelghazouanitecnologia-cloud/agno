use roska_descriptor::*;
use std::path::PathBuf;

#[test]
fn test_file_descriptor() {
    let mut file = FileDescriptor::new("src/main.rs", 100);
    file.purpose = Some("Entry point".into());
    file.imports.push(ImportEntry::new("std::io::Read").with_ctx("Binary stream reading"));
    file.exports.push(ExportEntry::func("main"));

    assert_eq!(file.file, PathBuf::from("src/main.rs"));
    assert_eq!(file.imports[0].sym, "std::io::Read");
    assert_eq!(file.imports[0].ctx.as_deref(), Some("Binary stream reading"));
}

#[test]
fn test_token_estimates() {
    let mut file = FileDescriptor::new("src/lib.rs", 500);
    file.imports = vec![ImportEntry::new("a"), ImportEntry::new("b")];

    let overview = file.token_estimate(Depth::Overview);
    let structure = file.token_estimate(Depth::Structure);
    let detail = file.token_estimate(Depth::Detail);

    assert!(overview < structure);
    assert!(structure < detail);
}

#[test]
fn test_data_entries() {
    let c = DataEntry::constant("MAX_SIZE", "usize", "65536");
    assert!(!c.mutable);
    assert_eq!(c.val.as_deref(), Some("65536"));

    let s = DataEntry::static_mut("CACHE", "Mutex<HashMap>");
    assert!(s.mutable);
}
