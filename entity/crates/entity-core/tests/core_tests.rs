use async_trait::async_trait;
use common_error::Result;
use entity_core::*;

// A mock agent for testing
struct MockAgent {
    name: String,
    cost: u8,
    handles: Vec<String>,
}

#[async_trait]
impl Agent for MockAgent {
    fn agent_type(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "mock agent for testing"
    }

    async fn process(&self, msg: Message) -> Result<Response> {
        Ok(Response::success(format!("[{}] processed: {}", self.name, msg.content))
            .with_usage(100, 50))
    }

    fn can_handle(&self, task_kind: &str) -> bool {
        self.handles.iter().any(|h| h == task_kind)
    }

    fn cost_tier(&self) -> u8 {
        self.cost
    }
}

fn mock_agent(name: &str, cost: u8, handles: Vec<&str>) -> Box<dyn Agent> {
    Box::new(MockAgent {
        name: name.to_string(),
        cost,
        handles: handles.into_iter().map(String::from).collect(),
    })
}

#[test]
fn test_message_construction() {
    let msg = Message::new(TaskKind::Describe, "fn main() {}")
        .with_context("file", "src/main.rs")
        .with_config("depth", "2");

    assert_eq!(msg.task, TaskKind::Describe);
    assert_eq!(msg.context.get("file"), Some(&"src/main.rs".to_string()));
    assert_eq!(msg.config.get("depth"), Some(&"2".to_string()));
}

#[test]
fn test_response() {
    let resp = Response::success("output")
        .with_usage(1000, 500);

    assert!(resp.is_success());
    assert_eq!(resp.usage.as_ref().unwrap().total(), 1500);
}

#[test]
fn test_response_error() {
    let resp = Response::error("something failed");
    assert!(!resp.is_success());
    assert_eq!(resp.status, ResponseStatus::Error);
}

#[test]
fn test_registry() {
    let mut reg = AgentRegistry::new();
    reg.register(mock_agent("descriptor", 1, vec!["describe"]));
    reg.register(mock_agent("planner", 2, vec!["plan"]));
    reg.register(mock_agent("coder", 3, vec!["code", "review"]));

    assert_eq!(reg.len(), 3);
    assert!(reg.get("descriptor").is_some());
    assert!(reg.get("nonexistent").is_none());
}

#[test]
fn test_cheapest_for() {
    let mut reg = AgentRegistry::new();
    reg.register(mock_agent("cheap-desc", 1, vec!["describe"]));
    reg.register(mock_agent("expensive-desc", 4, vec!["describe"]));
    reg.register(mock_agent("planner", 2, vec!["plan"]));

    let cheapest = reg.cheapest_for("describe").unwrap();
    assert_eq!(cheapest.cost_tier(), 1);
    assert_eq!(cheapest.agent_type(), "cheap-desc");
}

#[test]
fn test_agents_for() {
    let mut reg = AgentRegistry::new();
    reg.register(mock_agent("a", 1, vec!["describe"]));
    reg.register(mock_agent("b", 2, vec!["describe", "review"]));
    reg.register(mock_agent("c", 3, vec!["code"]));

    let describers = reg.agents_for("describe");
    assert_eq!(describers.len(), 2);
}

#[tokio::test]
async fn test_route_message() {
    let mut reg = AgentRegistry::new();
    reg.register(mock_agent("descriptor", 1, vec!["describe"]));

    let msg = Message::new(TaskKind::Describe, "fn hello() {}");
    let resp = route_message(&reg, msg).await.unwrap();

    assert!(resp.is_success());
    assert!(resp.content.contains("descriptor"));
}

#[tokio::test]
async fn test_route_no_agent() {
    let reg = AgentRegistry::new();
    let msg = Message::new(TaskKind::Code, "write code");
    let result = route_message(&reg, msg).await;

    assert!(result.is_err());
}

#[test]
fn test_task_kind_display() {
    assert_eq!(TaskKind::Describe.to_string(), "describe");
    assert_eq!(TaskKind::Plan.to_string(), "plan");
    assert_eq!(TaskKind::Custom("refactor".into()).to_string(), "custom:refactor");
}

#[test]
fn test_token_usage() {
    let usage = TokenUsage::new(500, 300);
    assert_eq!(usage.total(), 800);
}

#[test]
fn test_message_yaml_roundtrip() {
    let msg = Message::new(TaskKind::Describe, "code here")
        .with_context("file", "main.rs");

    let yaml = serde_yaml::to_string(&msg).unwrap();
    let parsed: Message = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.task, TaskKind::Describe);
    assert_eq!(parsed.content, "code here");
}
