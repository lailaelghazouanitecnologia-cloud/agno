use kkr_graph::{ProjectGraph, NodeKind};
use kkr_ast::{AstParser, Language, ProjectAst};
use std::path::PathBuf;

fn sample_graph() -> ProjectGraph {
    let mut project = ProjectAst::new("/test");

    let src_a = r#"
use crate::config;

pub fn process(data: &str) -> String {
    data.to_string()
}

pub struct Processor {
    pub name: String,
}
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/processor.rs"), src_a, Language::Rust,
    ));

    let src_b = r#"
pub struct Config {
    pub max: usize,
}

pub trait Configurable {
    fn configure(&self) -> Config;
}
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/config.rs"), src_b, Language::Rust,
    ));

    ProjectGraph::from_ast(&project)
}

#[test]
fn test_query_files() {
    let graph = sample_graph();
    let files = graph.query().files().count();
    assert_eq!(files, 2);
}

#[test]
fn test_query_symbols() {
    let graph = sample_graph();
    let symbols = graph.query().symbols().count();
    assert!(symbols >= 3); // process, Processor, Config, Configurable, configure
}

#[test]
fn test_query_by_kind() {
    let graph = sample_graph();
    let functions = graph.query().kind(NodeKind::Function).count();
    assert!(functions >= 1);
}

#[test]
fn test_query_name_contains() {
    let graph = sample_graph();
    let results = graph.query().name_contains("config").count();
    assert!(results >= 1);
}

#[test]
fn test_graph_build_from_ast() {
    let graph = sample_graph();
    assert!(graph.node_count() > 0);
    assert!(graph.edge_count() > 0); // At least Contains edges
}

#[test]
fn test_affected_by_change() {
    let graph = sample_graph();
    // Find Config struct
    let config_nodes = graph.find_symbol("Config");
    assert!(!config_nodes.is_empty());

    let affected = graph.affected_by_change(config_nodes[0]);
    // Config's containing file should be affected
    assert!(!affected.is_empty());
}
