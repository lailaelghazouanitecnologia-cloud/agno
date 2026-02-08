use kkr_plan::{builtin_templates, PlanTemplate};

#[test]
fn test_parse_kkr_template() {
    let toml = r#"
[template]
name = "api-endpoint"
description = "Create a REST API endpoint"

[[step]]
name = "analyze"
action = "Parse existing routes"
target = "tool:ast_parse"

[[step]]
name = "create-model"
action = "Create the data model"
target = "worker"
depends_on = ["analyze"]

[[step]]
name = "create-route"
action = "Create the route handler"
target = "worker"
depends_on = ["analyze"]

[[step]]
name = "validate"
action = "Verify endpoint works"
target = "validator"
depends_on = ["create-model", "create-route"]
"#;
    let template = PlanTemplate::from_kkr(toml).unwrap();

    assert_eq!(template.name, "api-endpoint");
    assert_eq!(template.steps.len(), 4);
    assert_eq!(template.required_tools, vec!["ast_parse"]);
}

#[test]
fn test_template_to_plan_ast() {
    let toml = r#"
[template]
name = "simple"

[[step]]
name = "analyze"
action = "Parse code"
target = "tool:ast_parse"

[[step]]
name = "implement"
action = "Write code"
target = "worker"
depends_on = ["analyze"]
"#;
    let template = PlanTemplate::from_kkr(toml).unwrap();
    let plan = template.to_plan_ast();

    let tools = plan.referenced_tools();
    assert_eq!(tools, vec!["ast_parse"]);
    assert!(plan.node_count() >= 3);
}

#[test]
fn test_builtin_templates() {
    let templates = builtin_templates();
    assert!(templates.len() >= 2);

    let feature = templates
        .iter()
        .find(|t| t.name == "implement-feature")
        .unwrap();
    assert_eq!(feature.steps.len(), 4);

    let fix = templates
        .iter()
        .find(|t| t.name == "fix-error")
        .unwrap();
    assert!(fix.required_tools.contains(&"errordb".to_string()));
}
