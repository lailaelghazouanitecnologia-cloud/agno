use kkr_refactor::{RefactorEngine, PropagationResult, Change};
use std::path::{Path, PathBuf};

#[test]
fn test_apply_rename_across_files() {
    let mut engine = RefactorEngine::new();
    engine.add_file("src/main.rs", "use crate::Config;\nfn main() { let c = Config::new(); }");
    engine.add_file("src/config.rs", "pub struct Config { pub name: String }");
    engine.add_file("tests/test.rs", "use my_crate::Config;\n#[test] fn test() { Config::new(); }");

    let result = engine.apply_rename("Config", "Settings");
    assert!(result.is_success());
    assert_eq!(result.files_modified.len(), 3);
    assert!(result.changes_applied >= 4);

    assert!(engine.get_content(Path::new("src/main.rs")).unwrap().contains("Settings"));
    assert!(engine.get_content(Path::new("src/config.rs")).unwrap().contains("Settings"));
    assert!(!engine.get_content(Path::new("src/config.rs")).unwrap().contains("Config"));
}

#[test]
fn test_apply_changes_removal() {
    let mut engine = RefactorEngine::new();
    engine.add_file("src/main.rs", "use crate::old_function;\nfn main() { old_function(); }");

    let changes = vec![Change::Removal {
        name: "old_function".to_string(),
        file: PathBuf::from("src/lib.rs"),
    }];

    let result = engine.apply_changes(&changes);
    assert!(!result.errors.is_empty()); // Should flag the reference
    assert!(result.errors[0].contains("old_function"));
}

#[test]
fn test_propagation_result_summary() {
    let result = PropagationResult {
        files_modified: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
        changes_applied: 5,
        errors: Vec::new(),
    };
    assert_eq!(result.summary(), "Propagated 5 changes across 2 files");
}
