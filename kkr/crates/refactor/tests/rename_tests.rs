use kkr_refactor::RenameChange;
use std::path::PathBuf;

#[test]
fn test_rename_basic() {
    let change = RenameChange {
        file: PathBuf::from("test.rs"),
        old_name: "foo".to_string(),
        new_name: "bar".to_string(),
        locations: Vec::new(),
    };

    let result = change.apply("fn foo() { foo(); }");
    assert_eq!(result, "fn bar() { bar(); }");
}

#[test]
fn test_rename_respects_word_boundaries() {
    let change = RenameChange {
        file: PathBuf::from("test.rs"),
        old_name: "get".to_string(),
        new_name: "fetch".to_string(),
        locations: Vec::new(),
    };

    let result = change.apply("fn get() { get_all(); }");
    assert_eq!(result, "fn fetch() { get_all(); }");
}

#[test]
fn test_rename_count() {
    let change = RenameChange {
        file: PathBuf::from("test.rs"),
        old_name: "Config".to_string(),
        new_name: "Settings".to_string(),
        locations: Vec::new(),
    };

    let count = change.count_replacements("struct Config { } impl Config { fn new() -> Config { } }");
    assert_eq!(count, 3);
}
