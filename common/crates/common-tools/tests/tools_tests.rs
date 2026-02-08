use common_tools::*;
use std::path::{Path, PathBuf};

#[test]
fn resolve_relative_path() {
    let base = Path::new("/workspace/project");
    let resolved = resolve_path(base, "src/main.rs").unwrap();
    assert_eq!(resolved, PathBuf::from("/workspace/project/src/main.rs"));
}

#[test]
fn resolve_absolute_path() {
    let base = Path::new("/workspace/project");
    let resolved = resolve_path(base, "/tmp/file.txt").unwrap();
    assert_eq!(resolved, PathBuf::from("/tmp/file.txt"));
}

#[test]
fn validate_path_in_workspace() {
    let ws = Path::new("/workspace");
    assert!(validate_in_workspace(Path::new("/workspace/src/lib.rs"), ws).is_ok());
    assert!(validate_in_workspace(Path::new("/etc/passwd"), ws).is_err());
}

#[test]
fn command_filter_blocks_dangerous() {
    let filter = CommandFilter::default_coding();
    assert!(filter.check("rm -rf /").is_err());
    assert!(filter.check("ls -la").is_ok());
    assert!(filter.check("cargo build").is_ok());
}

#[test]
fn truncation() {
    let long = "a".repeat(100);
    let truncated = truncate_output(&long, 50);
    assert!(truncated.len() < 100);
    assert!(truncated.contains("truncated"));

    let short = "hello";
    assert_eq!(truncate_output(short, 50), "hello");
}
