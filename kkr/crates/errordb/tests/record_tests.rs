use kkr_errordb::ErrorRecord;

#[test]
fn test_create_record() {
    let record = ErrorRecord::new(
        "El parser falla con tokens multi-byte",
        "Usar char_indices() en vez de bytes()",
    );
    assert_eq!(record.occurrences, 1);
    assert!(!record.is_recurring());
}

#[test]
fn test_bump_makes_recurring() {
    let mut record = ErrorRecord::new("error", "fix");
    assert!(!record.is_recurring());
    record.bump();
    assert!(record.is_recurring());
}

#[test]
fn test_matching() {
    let record = ErrorRecord::new(
        "Connection timeout after 30 seconds",
        "Increase timeout to 60 seconds",
    );
    // Exact match should work
    assert!(record.matches("Connection timeout after 30 seconds"));
    // Different numbers but same words
    assert!(record.matches("Connection timeout after 60 seconds"));
}

#[test]
fn test_display() {
    let record = ErrorRecord::new("problem text", "solution text");
    let display = record.display();
    assert!(display.contains("Problema:"));
    assert!(display.contains("Solución:"));
}
