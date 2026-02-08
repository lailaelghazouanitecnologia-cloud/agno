use kkr_ast::{AstParser, DualView, DescriptiveView, ConcreteView, Language, ProjectAst};
use std::path::PathBuf;

fn sample_project() -> ProjectAst {
    let mut project = ProjectAst::new("/test/project");

    let rust_src = r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
}

pub fn create(name: &str) -> Config {
    Config { name: name.to_string() }
}
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/config.rs"), rust_src, Language::Rust,
    ));

    let rust_src2 = r#"
use crate::config::Config;

pub trait Service {
    fn start(&self);
}

pub fn run(config: Config) {
    println!("{}", config.name);
}
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/main.rs"), rust_src2, Language::Rust,
    ));

    project
}

#[test]
fn test_dual_view() {
    let project = sample_project();
    let dual = DualView::from_project(&project);

    // Descriptive
    assert_eq!(dual.descriptive.modules.len(), 2);
    assert!(!dual.descriptive.summary.is_empty());
    let desc_text = dual.descriptive.render();
    assert!(desc_text.contains("Descriptive View"));
    assert!(desc_text.contains("config.rs"));

    // Concrete
    assert_eq!(dual.concrete.files.len(), 2);
    let conc_text = dual.concrete.render();
    assert!(conc_text.contains("Concrete View"));
    assert!(conc_text.contains("fn create"));
}

#[test]
fn test_descriptive_relationships() {
    let project = sample_project();
    let desc = DescriptiveView::from_project(&project);

    assert!(!desc.relationships.is_empty());
    assert!(desc.relationships.iter().any(|r| r.kind == "imports"));
}

#[test]
fn test_concrete_symbols() {
    let project = sample_project();
    let conc = ConcreteView::from_project(&project);

    let config_file = conc.files.iter()
        .find(|f| f.path.contains("config"))
        .unwrap();

    assert!(config_file.symbols.iter().any(|s| s.name == "Config"));
    assert!(config_file.symbols.iter().any(|s| s.name == "create"));
}

#[test]
fn test_token_estimates() {
    let project = sample_project();
    let dual = DualView::from_project(&project);

    // Descriptive should be cheaper than concrete
    assert!(dual.descriptive.token_estimate < dual.concrete.token_estimate);
}
