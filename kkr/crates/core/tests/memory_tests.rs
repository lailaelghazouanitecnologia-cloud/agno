use kkr_core::memory::{
    estimate_tokens, InMemoryStorage, Memory, MemoryConfig, MemoryEntry, MemoryStorage,
    TrimStrategy,
};
use kkr_core::{Message, Role};

fn make_message(content: &str) -> Message {
    Message {
        role: Role::User,
        content: content.to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

#[test]
fn test_estimate_tokens() {
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("Hi"), 1);
    assert_eq!(estimate_tokens("Test"), 1);
    assert_eq!(estimate_tokens("Hello"), 2);
    assert_eq!(estimate_tokens("12345678"), 2);

    let long_text = "a".repeat(100);
    assert_eq!(estimate_tokens(&long_text), 25);
}

#[test]
fn test_memory_config_validation() {
    let valid = MemoryConfig::default();
    assert!(valid.is_valid());

    let invalid = MemoryConfig {
        max_messages: 0,
        ..Default::default()
    };
    assert!(!invalid.is_valid());
}

#[test]
fn test_memory_add_and_retrieve() {
    let mut memory = Memory::new();

    memory.add(make_message("Hello"));
    memory.add(Message {
        role: Role::Assistant,
        content: "Hi there!".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    assert_eq!(memory.len(), 2);
    let messages = memory.messages();
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there!");
}

#[test]
fn test_memory_with_session() {
    let mut memory = Memory::new()
        .with_session("session-123")
        .with_user("user-456");

    memory.add(make_message("Test message"));

    assert_eq!(memory.session_id(), Some("session-123"));
    assert_eq!(memory.user_id(), Some("user-456"));
}

#[test]
fn test_memory_state() {
    let mut memory = Memory::new();

    memory.set_state("count", 42);
    memory.set_state("name", "test");

    assert_eq!(memory.get_state::<i32>("count"), Some(42));
    assert_eq!(memory.get_state::<String>("name"), Some("test".to_string()));
    assert_eq!(memory.get_state::<i32>("missing"), None);
}

#[test]
fn test_chat_history() {
    let mut memory = Memory::new();

    memory.add(Message {
        role: Role::System,
        content: "System".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });
    memory.add(make_message("User 1"));
    memory.add(Message {
        role: Role::Assistant,
        content: "Assistant 1".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });
    memory.add(Message {
        role: Role::Tool,
        content: "Tool result".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });
    memory.add(make_message("User 2"));

    let history = memory.chat_history(None);
    assert_eq!(history.len(), 3);
}

#[tokio::test]
async fn test_trim_strategy() {
    use kkr_core::memory::OptimizationStrategy;

    let strategy = TrimStrategy::new(3);

    let entries: Vec<_> = (0..10)
        .map(|i| MemoryEntry::new(make_message(&format!("Message {}", i))))
        .collect();

    let optimized = strategy.optimize(entries).await.unwrap();
    assert_eq!(optimized.len(), 3);
    assert_eq!(optimized[0].message.content, "Message 7");
    assert_eq!(optimized[2].message.content, "Message 9");
}

#[test]
fn test_memory_clear() {
    let mut memory = Memory::new();

    memory.add(make_message("Test"));

    assert_eq!(memory.len(), 1);
    memory.clear();
    assert_eq!(memory.len(), 0);
    assert!(memory.is_empty());
}

#[tokio::test]
async fn test_storage_eviction() {
    let mut storage = InMemoryStorage::with_max_entries(3);

    for i in 0..5 {
        let entry = MemoryEntry::new(make_message(&format!("msg-{}", i)));
        storage.store(entry).await.unwrap();
    }

    assert_eq!(storage.count().await.unwrap(), 3);

    let all = storage.retrieve_all(None).await.unwrap();
    assert_eq!(all[0].message.content, "msg-2");
    assert_eq!(all[2].message.content, "msg-4");
}

#[tokio::test]
async fn test_storage_limit() {
    let mut storage = InMemoryStorage::new();

    for i in 0..10 {
        let entry = MemoryEntry::new(make_message(&format!("msg-{}", i)));
        storage.store(entry).await.unwrap();
    }

    let limited = storage.retrieve_all(Some(3)).await.unwrap();
    assert_eq!(limited.len(), 3);
    assert_eq!(limited[0].message.content, "msg-7");
    assert_eq!(limited[2].message.content, "msg-9");
}

#[tokio::test]
async fn test_storage_delete_by_session() {
    let mut storage = InMemoryStorage::new();

    storage
        .store(MemoryEntry::new(make_message("a")).with_session("s1"))
        .await
        .unwrap();
    storage
        .store(MemoryEntry::new(make_message("b")).with_session("s2"))
        .await
        .unwrap();
    storage
        .store(MemoryEntry::new(make_message("c")).with_session("s1"))
        .await
        .unwrap();

    let deleted = storage.delete_by_session("s1").await.unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(storage.count().await.unwrap(), 1);
}

#[test]
fn test_memory_entry_creation() {
    let entry = MemoryEntry::new(make_message("Hello world"))
        .with_user("user-1")
        .with_session("session-1");

    assert_eq!(entry.message.content, "Hello world");
    assert_eq!(entry.user_id, Some("user-1".to_string()));
    assert_eq!(entry.session_id, Some("session-1".to_string()));
    assert!(entry.token_count > 0);
}

#[test]
fn test_memory_get_by_role() {
    let mut memory = Memory::new();

    memory.add(make_message("User msg 1"));
    memory.add(Message {
        role: Role::Assistant,
        content: "Assistant msg".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });
    memory.add(make_message("User msg 2"));

    let user_messages = memory.get_by_role(Role::User);
    assert_eq!(user_messages.len(), 2);

    let assistant_messages = memory.get_by_role(Role::Assistant);
    assert_eq!(assistant_messages.len(), 1);
}
