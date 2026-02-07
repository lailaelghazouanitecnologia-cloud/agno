use common_serde::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Sample {
    name: String,
    value: i32,
}

#[test]
fn test_yaml_roundtrip() {
    let s = Sample { name: "test".into(), value: 42 };
    let yaml = to_string(&s, Format::Yaml).unwrap();
    let parsed: Sample = from_yaml(&yaml).unwrap();
    assert_eq!(parsed, s);
}

#[test]
fn test_json_roundtrip() {
    let s = Sample { name: "test".into(), value: 42 };
    let json = to_string(&s, Format::JsonCompact).unwrap();
    let parsed: Sample = from_json(&json).unwrap();
    assert_eq!(parsed, s);
}

#[test]
fn test_auto_detect_json() {
    let json = r#"{"name": "auto", "value": 7}"#;
    let parsed: Sample = from_str_auto(json).unwrap();
    assert_eq!(parsed.name, "auto");
}

#[test]
fn test_auto_detect_yaml() {
    let yaml = "name: auto\nvalue: 7\n";
    let parsed: Sample = from_str_auto(yaml).unwrap();
    assert_eq!(parsed.name, "auto");
}

#[test]
fn test_truncate_for_tokens() {
    let short = "hello";
    assert_eq!(truncate_for_tokens(short, 100), "hello");

    let long = "a".repeat(1000);
    let truncated = truncate_for_tokens(&long, 10);
    assert!(truncated.len() < 1000);
    assert!(truncated.contains("truncated"));
}

#[test]
fn test_merge_yaml() {
    let base = "name: old\nport: 8080\n";
    let overlay = "name: new\ndebug: true\n";
    let merged = merge_yaml(base, overlay).unwrap();
    assert!(merged.contains("new"));
    assert!(merged.contains("8080"));
    assert!(merged.contains("true"));
}
