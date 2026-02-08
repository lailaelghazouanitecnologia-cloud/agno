use kkr_project::Template;
use kkr_ast::AstLevel;

#[test]
fn test_parse_kkr_template() {
    let kkr = r#"
[template]
name = "api-endpoint"
description = "Create a new REST API endpoint"

[context]
include_files = ["src/routes/*.rs", "src/models/*.rs"]
ast_level = "symbol"
max_tokens = 4000

[[plan]]
name = "create-model"
agent = "worker"
action = "Create the data model"

[[plan]]
name = "create-route"
agent = "worker"
action = "Create the route handler"
depends_on = ["create-model"]

[[plan]]
name = "validate"
agent = "validator"
action = "Verify the endpoint compiles"

[[validation]]
name = "compiles"
check = "cargo build succeeds"
severity = "error"
"#;
    let template = Template::from_kkr(kkr).unwrap();
    assert_eq!(template.name, "api-endpoint");
    assert_eq!(template.plan.len(), 3);
    assert!(matches!(&template.plan[0].strategy,
        kkr_plan::StepStrategy::AgentDispatch { agent_role, .. } if agent_role == "worker"));
    assert!(matches!(&template.plan[2].strategy,
        kkr_plan::StepStrategy::AgentDispatch { agent_role, .. } if agent_role == "validator"));
    assert_eq!(template.validation.len(), 1);
    assert_eq!(template.context.ast_level, AstLevel::Symbol);
}

#[test]
fn test_roundtrip() {
    let kkr = r#"
[template]
name = "test"
description = "Test template"

[context]
include_files = ["src/*.rs"]
ast_level = "file"

[[plan]]
name = "step1"
agent = "worker"
action = "Do something"
"#;
    let template = Template::from_kkr(kkr).unwrap();
    let serialized = template.to_kkr();
    assert!(serialized.contains("name = \"test\""));
    assert!(serialized.contains("ast_level = \"file\""));
}
