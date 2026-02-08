use kkr_project::{ProjectRef, RefLevel};

#[test]
fn test_ref_level_progression() {
    assert_eq!(RefLevel::Basic.next(), RefLevel::Intermediate);
    assert_eq!(RefLevel::Intermediate.next(), RefLevel::Advanced);
    assert_eq!(RefLevel::Advanced.next(), RefLevel::Expert);
    assert_eq!(RefLevel::Expert.next(), RefLevel::Expert);
}

#[test]
fn test_ref_level_estimate() {
    assert_eq!(RefLevel::estimate(3, 10, false), RefLevel::Basic);
    assert_eq!(RefLevel::estimate(15, 50, false), RefLevel::Intermediate);
    assert_eq!(RefLevel::estimate(15, 50, true), RefLevel::Advanced);
    assert_eq!(RefLevel::estimate(50, 200, true), RefLevel::Advanced);
}

#[test]
fn test_search_query() {
    let query = ProjectRef::search_query("C to JavaScript compiler", RefLevel::Basic);
    assert!(query.contains("structured project with modules"));
    assert!(query.contains("C to JavaScript compiler"));
}

#[test]
fn test_from_web() {
    let ref_proj = ProjectRef::from_web(
        "tinycc",
        "https://github.com/example/tinycc",
        "A tiny C compiler",
        RefLevel::Advanced,
    );
    assert_eq!(ref_proj.name, "tinycc");
    assert_eq!(ref_proj.level, RefLevel::Advanced);
}
