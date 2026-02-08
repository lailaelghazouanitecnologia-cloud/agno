use kkr_errordb::ErrorDb;

#[test]
fn test_record_first_time() {
    let mut db = ErrorDb::new();
    let recurring = db.record_error(
        "Parser crash on empty input",
        "Add empty input check at parser entry",
    ).unwrap();
    assert!(!recurring); // First time -- not recurring
    assert_eq!(db.len(), 1);
}

#[test]
fn test_record_recurring() {
    let mut db = ErrorDb::new();

    let r1 = db.record_error("Parser crash on empty input", "Add guard").unwrap();
    assert!(!r1);

    let r2 = db.record_error("Parser crash on empty input", "Add guard").unwrap();
    assert!(r2); // Second time -- recurring!
}

#[test]
fn test_find_solutions_only_recurring() {
    let mut db = ErrorDb::new();

    // Record once -- should not appear in solutions
    db.record_error("Timeout error", "Increase timeout").unwrap();
    assert!(db.find_solutions("Timeout error").is_empty());

    // Record again -- now it should appear
    db.record_error("Timeout error", "Increase timeout").unwrap();
    let solutions = db.find_solutions("Timeout error");
    assert_eq!(solutions.len(), 1);
}

#[test]
fn test_by_tag() {
    let mut db = ErrorDb::new();
    db.record_error_with_context(
        "Connection refused",
        "Start the server first",
        vec!["network".to_string(), "server".to_string()],
        Some("rust"),
        None,
    ).unwrap();

    assert_eq!(db.by_tag("network").len(), 1);
    assert_eq!(db.by_tag("database").len(), 0);
}

#[test]
fn test_persistence() {
    let dir = std::env::temp_dir().join("kkr_errordb_test");
    let path = dir.join("errors.json");

    // Clean up from previous runs
    let _ = std::fs::remove_file(&path);

    {
        let mut db = ErrorDb::from_file(&path).unwrap();
        db.record_error("test error", "test fix").unwrap();
        db.record_error("test error", "test fix").unwrap();
    }

    // Load again -- should have the record
    {
        let db = ErrorDb::from_file(&path).unwrap();
        assert_eq!(db.len(), 1);
        assert_eq!(db.all()[0].occurrences, 2);
    }

    // Clean up
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(dir);
}
