//! Memory tests

use kkr_core::memory::{Memory, MemoryEntry, TrimStrategy};
use kkr_core::{Message, Role};

#[test]
fn test_memory_add_and_retrieve() {
    let mut memory = Memory::new();

    memory.add(Message {
        role: Role::User,
        content: "Hello".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

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

    memory.add(Message {
        role: Role::User,
        content: "Test message".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

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
    memory.add(Message {
        role: Role::User,
        content: "User 1".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });
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
    memory.add(Message {
        role: Role::User,
        content: "User 2".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    let history = memory.chat_history(None);
    assert_eq!(history.len(), 3); // Only user and assistant
}

#[tokio::test]
async fn test_trim_strategy() {
    use kkr_core::memory::OptimizationStrategy;

    let strategy = TrimStrategy::new(3);

    let entries: Vec<_> = (0..10)
        .map(|i| {
            MemoryEntry::new(Message {
                role: Role::User,
                content: format!("Message {}", i),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            })
        })
        .collect();

    let optimized = strategy.optimize(entries).await.unwrap();
    assert_eq!(optimized.len(), 3);
    assert_eq!(optimized[0].message.content, "Message 7");
    assert_eq!(optimized[2].message.content, "Message 9");
}

#[test]
fn test_memory_clear() {
    let mut memory = Memory::new();

    memory.add(Message {
        role: Role::User,
        content: "Test".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    assert_eq!(memory.len(), 1);
    memory.clear();
    assert_eq!(memory.len(), 0);
    assert!(memory.is_empty());
}
