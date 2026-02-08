use common_traits::*;

struct TestComponent {
    name: String,
    healthy: bool,
}

impl Named for TestComponent {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Describable for TestComponent {
    fn summary(&self) -> String {
        format!("Component: {}", self.name)
    }

    fn tags(&self) -> Vec<String> {
        vec!["test".into()]
    }
}

impl Lifecycle for TestComponent {
    fn is_healthy(&self) -> bool {
        self.healthy
    }
}

impl TokenEstimate for TestComponent {
    fn token_estimate(&self, depth: u8) -> usize {
        match depth {
            0 => 5,
            1 => 20,
            2 => 50,
            _ => 100,
        }
    }
}

#[test]
fn test_named() {
    let c = TestComponent { name: "parser".into(), healthy: true };
    assert_eq!(c.name(), "parser");
}

#[test]
fn test_describable() {
    let c = TestComponent { name: "parser".into(), healthy: true };
    assert!(c.summary().contains("parser"));
    assert_eq!(c.tags(), vec!["test"]);
    assert_eq!(c.description(), c.summary());
}

#[test]
fn test_lifecycle() {
    let c = TestComponent { name: "healthy".into(), healthy: true };
    assert!(c.is_healthy());

    let c2 = TestComponent { name: "sick".into(), healthy: false };
    assert!(!c2.is_healthy());
}

#[test]
fn test_token_estimate() {
    let c = TestComponent { name: "x".into(), healthy: true };
    assert!(c.token_estimate(0) < c.token_estimate(1));
    assert!(c.token_estimate(1) < c.token_estimate(2));
    assert!(c.token_estimate(2) < c.token_estimate(3));
}
