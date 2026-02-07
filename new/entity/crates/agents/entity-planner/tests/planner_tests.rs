use async_trait::async_trait;
use common_error::Result;
use entity_core::{Agent, Message, TaskKind};
use entity_planner::*;
use roska_descriptor::change::{ChangeKind, Impact};
use std::path::PathBuf;

struct MockPlannerLlm {
    response: String,
}

#[async_trait]
impl PlannerLlm for MockPlannerLlm {
    async fn plan(&self, _prompt: &str, _max_tokens: usize) -> Result<PlannerLlmResponse> {
        Ok(PlannerLlmResponse {
            text: self.response.clone(),
            input_tokens: 500,
            output_tokens: 300,
        })
    }
}

fn sample_plan_yaml() -> &'static str {
    r#"goal: "Add pagination support to the user list endpoint"
steps:
  - index: 0
    action: "Add PaginationParams type"
    file: "src/types.rs"
    change_kind:
      type: TypeAdded
      name: "PaginationParams"
    target: "PaginationParams"
    instructions: "Add struct with page: usize, per_page: usize, with Default impl"
    optional: false
  - index: 1
    action: "Update list_users to accept pagination"
    file: "src/handlers.rs"
    change_kind:
      type: ParamAdded
      param:
        name: "pagination"
        type: "Query<PaginationParams>"
      pos: 1
    target: "list_users"
    instructions: "Add Query<PaginationParams> parameter, apply LIMIT/OFFSET"
    optional: false
  - index: 2
    action: "Add pagination import"
    file: "src/handlers.rs"
    change_kind:
      type: ImportAdded
      sym: "crate::types::PaginationParams"
    target: "PaginationParams"
    optional: false
impact: medium
affected_files:
  - "src/types.rs"
  - "src/handlers.rs"
dependencies:
  - [1, 0]
"#
}

#[test]
fn test_parse_plan() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );

    let plan = agent.parse_plan(sample_plan_yaml()).unwrap();
    assert_eq!(plan.goal, "Add pagination support to the user list endpoint");
    assert_eq!(plan.steps.len(), 3);
    assert!(matches!(plan.steps[0].change_kind, ChangeKind::TypeAdded { .. }));
    assert_eq!(plan.impact, Impact::Medium);
    assert_eq!(plan.affected_files.len(), 2);
    assert_eq!(plan.dependencies.len(), 1);
}

#[test]
fn test_validate_plan() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );

    let plan = agent.parse_plan(sample_plan_yaml()).unwrap();
    assert!(agent.validate_plan(&plan).is_ok());
}

#[test]
fn test_validate_empty_plan() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );

    let plan = ChangePlan {
        goal: "empty".to_string(),
        steps: vec![],
        impact: Impact::Low,
        affected_files: vec![],
        dependencies: vec![],
    };

    assert!(agent.validate_plan(&plan).is_err());
}

#[test]
fn test_validate_self_dependency() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );

    let plan = ChangePlan {
        goal: "bad".to_string(),
        steps: vec![PlanStep {
            index: 0,
            action: "do".to_string(),
            file: PathBuf::from("x.rs"),
            change_kind: ChangeKind::Rename { from: "x".into(), to: "y".into() },
            target: "x".to_string(),
            instructions: None,
            optional: false,
        }],
        impact: Impact::Low,
        affected_files: vec![],
        dependencies: vec![(0, 0)],
    };

    assert!(agent.validate_plan(&plan).is_err());
}

#[test]
fn test_build_prompt() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );

    let context = PlanContext {
        task: "Add caching".to_string(),
        file_summaries: vec![FileSummary {
            path: PathBuf::from("src/main.rs"),
            purpose: Some("Entry point".to_string()),
            function_names: vec!["main".to_string()],
            type_names: vec![],
            lines: 50,
        }],
        constraints: vec!["No breaking changes".to_string()],
    };

    let prompt = agent.build_prompt(&context);
    assert!(prompt.contains("Add caching"));
    assert!(prompt.contains("src/main.rs"));
    assert!(prompt.contains("No breaking changes"));
}

#[tokio::test]
async fn test_process_plan() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm {
            response: sample_plan_yaml().to_string(),
        }),
        PlannerConfig::default(),
    );

    let context = PlanContext {
        task: "Add pagination".to_string(),
        file_summaries: vec![],
        constraints: vec![],
    };

    let msg = Message::new(TaskKind::Plan, serde_yaml::to_string(&context).unwrap());
    let resp = agent.process(msg).await.unwrap();

    assert!(resp.is_success());
    assert!(resp.content.contains("pagination"));
}

#[test]
fn test_plan_yaml_roundtrip() {
    let plan = ChangePlan {
        goal: "test".to_string(),
        steps: vec![PlanStep {
            index: 0,
            action: "add function".to_string(),
            file: PathBuf::from("src/lib.rs"),
            change_kind: ChangeKind::FunctionAdded { name: "new_func".into() },
            target: "new_func".to_string(),
            instructions: Some("Add a public function".to_string()),
            optional: false,
        }],
        impact: Impact::Low,
        affected_files: vec![PathBuf::from("src/lib.rs")],
        dependencies: vec![],
    };

    let yaml = serde_yaml::to_string(&plan).unwrap();
    let parsed: ChangePlan = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.goal, "test");
    assert_eq!(parsed.steps.len(), 1);
}

#[test]
fn test_agent_metadata() {
    let agent = PlannerAgent::new(
        Box::new(MockPlannerLlm { response: String::new() }),
        PlannerConfig::default(),
    );
    assert_eq!(agent.agent_type(), "planner");
    assert_eq!(agent.cost_tier(), 2);
    assert!(agent.can_handle("plan"));
    assert!(!agent.can_handle("describe"));
}
