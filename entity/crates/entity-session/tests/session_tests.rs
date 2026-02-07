use entity_session::*;
use std::path::PathBuf;

fn make_turn(index: usize, tokens: usize, files: Vec<&str>) -> Turn {
    Turn {
        index,
        timestamp: chrono::Utc::now(),
        input: TurnInput {
            role: MessageRole::User,
            content: format!("turn {} input", index),
            context_files: Vec::new(),
        },
        output: TurnOutput {
            content: format!("turn {} output", index),
            status: TurnStatus::Success,
            files_modified: files.into_iter().map(PathBuf::from).collect(),
        },
        tool_calls: Vec::new(),
        tokens_used: TokenUsageSummary {
            input_tokens: tokens / 2,
            output_tokens: tokens / 2,
            tool_tokens: 0,
        },
        duration_ms: 100,
    }
}

#[test]
fn test_session_new() {
    let session = Session::new(SessionConfig::default());
    assert_eq!(session.status, SessionStatus::Active);
    assert_eq!(session.turns.len(), 0);
    assert_eq!(session.modified_files.len(), 0);
    assert!(!session.id.is_empty());
}

#[test]
fn test_session_config_default() {
    let config = SessionConfig::default();
    assert_eq!(config.max_turns, 50);
    assert_eq!(config.max_tokens, 500_000);
    assert_eq!(config.checkpoint_interval, 5);
    assert!(config.keep_full_history);
}

#[test]
fn test_add_turn() {
    let mut session = Session::new(SessionConfig::default());
    let turn = make_turn(0, 100, vec!["src/main.rs"]);
    session.add_turn(turn).unwrap();
    assert_eq!(session.turn_count(), 1);
    assert_eq!(session.total_usage.total(), 100);
    assert_eq!(session.modified_files.len(), 1);
}

#[test]
fn test_add_turn_to_completed_session() {
    let mut session = Session::new(SessionConfig::default());
    session.complete();
    let turn = make_turn(0, 100, vec![]);
    assert!(session.add_turn(turn).is_err());
}

#[test]
fn test_max_turns_limit() {
    let config = SessionConfig {
        max_turns: 2,
        ..SessionConfig::default()
    };
    let mut session = Session::new(config);
    session.add_turn(make_turn(0, 50, vec![])).unwrap();
    session.add_turn(make_turn(1, 50, vec![])).unwrap();
    assert!(session.add_turn(make_turn(2, 50, vec![])).is_err());
}

#[test]
fn test_checkpoint_and_rollback() {
    let mut session = Session::new(SessionConfig {
        checkpoint_interval: 0, // disable auto
        ..SessionConfig::default()
    });

    session.add_turn(make_turn(0, 100, vec!["a.rs"])).unwrap();
    session.add_turn(make_turn(1, 100, vec!["b.rs"])).unwrap();
    let cp_id = session.create_checkpoint("before risky change".to_string());
    session.add_turn(make_turn(2, 200, vec!["c.rs"])).unwrap();

    assert_eq!(session.turn_count(), 3);
    assert_eq!(session.total_usage.total(), 400);
    assert_eq!(session.modified_files.len(), 3);

    // Rollback
    let removed = session.rollback_to(&cp_id).unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(session.turn_count(), 2);
    assert_eq!(session.total_usage.total(), 200);
    assert_eq!(session.modified_files.len(), 2);
}

#[test]
fn test_rollback_nonexistent() {
    let mut session = Session::new(SessionConfig::default());
    assert!(session.rollback_to("nonexistent").is_err());
}

#[test]
fn test_undo() {
    let mut session = Session::new(SessionConfig::default());
    session.add_turn(make_turn(0, 100, vec!["a.rs"])).unwrap();
    session.add_turn(make_turn(1, 200, vec!["b.rs"])).unwrap();
    session.add_turn(make_turn(2, 300, vec!["c.rs"])).unwrap();

    let removed = session.undo(2);
    assert_eq!(removed.len(), 2);
    assert_eq!(session.turn_count(), 1);
    assert_eq!(session.total_usage.total(), 100);
}

#[test]
fn test_undo_more_than_available() {
    let mut session = Session::new(SessionConfig::default());
    session.add_turn(make_turn(0, 100, vec![])).unwrap();
    let removed = session.undo(10);
    assert_eq!(removed.len(), 1);
    assert_eq!(session.turn_count(), 0);
}

#[test]
fn test_format_history() {
    let mut session = Session::new(SessionConfig::default());
    for i in 0..5 {
        session.add_turn(make_turn(i, 100, vec![])).unwrap();
    }
    let history = session.format_history(1_000_000);
    // Each turn produces 2 entries (user + assistant)
    assert_eq!(history.len(), 10);
    assert_eq!(history[0].role, MessageRole::User);
    assert_eq!(history[1].role, MessageRole::Assistant);
}

#[test]
fn test_session_complete() {
    let mut session = Session::new(SessionConfig::default());
    session.complete();
    assert_eq!(session.status, SessionStatus::Completed);
}

#[test]
fn test_session_fail() {
    let mut session = Session::new(SessionConfig::default());
    session.fail("out of tokens");
    assert_eq!(session.status, SessionStatus::Failed);
    assert_eq!(session.metadata.get("failure_reason").unwrap(), "out of tokens");
}

#[test]
fn test_session_summary() {
    let session = Session::new(SessionConfig::default());
    let summary = session.summary();
    assert!(summary.contains("Active"));
    assert!(summary.contains("0 turns"));
}

#[test]
fn test_token_usage_summary() {
    let mut usage = TokenUsageSummary::default();
    assert_eq!(usage.total(), 0);
    usage.input_tokens = 100;
    usage.output_tokens = 200;
    usage.tool_tokens = 50;
    assert_eq!(usage.total(), 350);

    let other = TokenUsageSummary {
        input_tokens: 10,
        output_tokens: 20,
        tool_tokens: 5,
    };
    usage.add(&other);
    assert_eq!(usage.total(), 385);
}

#[test]
fn test_session_serialization() {
    let mut session = Session::new(SessionConfig::default());
    session.add_turn(make_turn(0, 100, vec!["test.rs"])).unwrap();

    let json = session.to_json().unwrap();
    let restored = Session::from_json(&json).unwrap();
    assert_eq!(restored.id, session.id);
    assert_eq!(restored.turn_count(), 1);
    assert_eq!(restored.modified_files.len(), 1);
}

#[test]
fn test_auto_checkpoint() {
    let config = SessionConfig {
        checkpoint_interval: 2,
        ..SessionConfig::default()
    };
    let mut session = Session::new(config);
    session.add_turn(make_turn(0, 50, vec![])).unwrap();
    assert_eq!(session.checkpoints.len(), 0);
    session.add_turn(make_turn(1, 50, vec![])).unwrap();
    assert_eq!(session.checkpoints.len(), 1); // auto at turn 2
}

#[test]
fn test_modified_files_dedup() {
    let mut session = Session::new(SessionConfig::default());
    session.add_turn(make_turn(0, 50, vec!["a.rs", "b.rs"])).unwrap();
    session.add_turn(make_turn(1, 50, vec!["a.rs", "c.rs"])).unwrap();
    // a.rs should not be duplicated
    assert_eq!(session.modified_files.len(), 3);
}

#[test]
fn test_session_store_roundtrip() {
    let dir = std::env::temp_dir().join("entity_session_test");
    let _ = std::fs::remove_dir_all(&dir);

    let store = SessionStore::new(dir.clone());
    let mut session = Session::new(SessionConfig::default());
    session.add_turn(make_turn(0, 100, vec!["x.rs"])).unwrap();

    let path = store.save(&session).unwrap();
    assert!(path.exists());

    let loaded = store.load(&session.id).unwrap();
    assert_eq!(loaded.id, session.id);
    assert_eq!(loaded.turn_count(), 1);

    let list = store.list().unwrap();
    assert!(list.contains(&session.id));

    store.delete(&session.id).unwrap();
    assert!(store.load(&session.id).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}
