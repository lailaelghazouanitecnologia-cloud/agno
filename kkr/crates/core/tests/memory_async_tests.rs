//! Tests for Memory async operations and SharedMemory.

use kkr_core::memory::{Memory, MemoryConfig, SharedMemory};
use kkr_core::{Message, Role};

fn make_user_message(content: &str) -> Message {
    Message {
        role: Role::User,
        content: content.to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

fn make_assistant_message(content: &str) -> Message {
    Message {
        role: Role::Assistant,
        content: content.to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

// ---------------------------------------------------------------------------
// add_async works
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_add_async_stores_message() {
    let mut memory = Memory::new();

    let result = memory.add_async(make_user_message("Hello async")).await;
    assert!(result.is_ok());

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello async");
}

#[tokio::test]
async fn test_add_async_preserves_role() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("User msg")).await.unwrap();
    memory.add_async(make_assistant_message("Assistant msg")).await.unwrap();

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[1].role, Role::Assistant);
}

#[tokio::test]
async fn test_add_async_returns_ok() {
    let mut memory = Memory::new();

    let result = memory.add_async(make_user_message("test")).await;
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// messages_async returns messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_messages_async_returns_empty_initially() {
    let memory = Memory::new();

    let messages = memory.messages_async().await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_messages_async_returns_all_stored_messages() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("First")).await.unwrap();
    memory.add_async(make_assistant_message("Second")).await.unwrap();
    memory.add_async(make_user_message("Third")).await.unwrap();

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].content, "First");
    assert_eq!(messages[1].content, "Second");
    assert_eq!(messages[2].content, "Third");
}

#[tokio::test]
async fn test_messages_async_preserves_order() {
    let mut memory = Memory::new();

    for i in 0..10 {
        memory
            .add_async(make_user_message(&format!("Message {}", i)))
            .await
            .unwrap();
    }

    let messages = memory.messages_async().await.unwrap();
    for (i, msg) in messages.iter().enumerate() {
        assert_eq!(msg.content, format!("Message {}", i));
    }
}

// ---------------------------------------------------------------------------
// clear_async clears everything
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_clear_async_removes_all_messages() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("One")).await.unwrap();
    memory.add_async(make_user_message("Two")).await.unwrap();
    memory.add_async(make_user_message("Three")).await.unwrap();

    let count_before = memory.len_async().await.unwrap();
    assert_eq!(count_before, 3);

    memory.clear_async().await.unwrap();

    let count_after = memory.len_async().await.unwrap();
    assert_eq!(count_after, 0);

    let messages = memory.messages_async().await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_clear_async_on_empty_memory_succeeds() {
    let mut memory = Memory::new();

    let result = memory.clear_async().await;
    assert!(result.is_ok());

    let messages = memory.messages_async().await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_clear_async_also_clears_state() {
    let mut memory = Memory::new();

    memory.set_state("key", "value");
    assert!(memory.state().contains_key("key"));

    memory.clear_async().await.unwrap();

    assert!(memory.state().is_empty());
}

#[tokio::test]
async fn test_clear_async_allows_new_messages_after() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("Before clear")).await.unwrap();
    memory.clear_async().await.unwrap();
    memory.add_async(make_user_message("After clear")).await.unwrap();

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "After clear");
}

// ---------------------------------------------------------------------------
// Multiple add_async calls accumulate messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multiple_add_async_accumulates() {
    let mut memory = Memory::new();

    for i in 0..5 {
        memory
            .add_async(make_user_message(&format!("Msg {}", i)))
            .await
            .unwrap();
    }

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages.len(), 5);
}

#[tokio::test]
async fn test_accumulation_with_different_roles() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("User Q1")).await.unwrap();
    memory.add_async(make_assistant_message("Bot A1")).await.unwrap();
    memory.add_async(make_user_message("User Q2")).await.unwrap();
    memory.add_async(make_assistant_message("Bot A2")).await.unwrap();

    let messages = memory.messages_async().await.unwrap();
    assert_eq!(messages.len(), 4);

    let user_msgs: Vec<_> = messages.iter().filter(|m| m.role == Role::User).collect();
    let asst_msgs: Vec<_> = messages.iter().filter(|m| m.role == Role::Assistant).collect();
    assert_eq!(user_msgs.len(), 2);
    assert_eq!(asst_msgs.len(), 2);
}

#[tokio::test]
async fn test_len_async_tracks_count() {
    let mut memory = Memory::new();

    assert_eq!(memory.len_async().await.unwrap(), 0);

    memory.add_async(make_user_message("One")).await.unwrap();
    assert_eq!(memory.len_async().await.unwrap(), 1);

    memory.add_async(make_user_message("Two")).await.unwrap();
    assert_eq!(memory.len_async().await.unwrap(), 2);

    memory.add_async(make_user_message("Three")).await.unwrap();
    assert_eq!(memory.len_async().await.unwrap(), 3);
}

// ---------------------------------------------------------------------------
// last_n_async
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_last_n_async_returns_last_n_messages() {
    let mut memory = Memory::new();

    for i in 0..10 {
        memory
            .add_async(make_user_message(&format!("Msg {}", i)))
            .await
            .unwrap();
    }

    let last_3 = memory.last_n_async(3).await.unwrap();
    assert_eq!(last_3.len(), 3);
    assert_eq!(last_3[0].content, "Msg 7");
    assert_eq!(last_3[1].content, "Msg 8");
    assert_eq!(last_3[2].content, "Msg 9");
}

#[tokio::test]
async fn test_last_n_async_with_n_larger_than_stored() {
    let mut memory = Memory::new();

    memory.add_async(make_user_message("Only one")).await.unwrap();

    let result = memory.last_n_async(100).await.unwrap();
    assert_eq!(result.len(), 1);
}

// ---------------------------------------------------------------------------
// token_count_async
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_token_count_async_empty() {
    let memory = Memory::new();
    let count = memory.token_count_async().await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_token_count_async_increases_with_messages() {
    let mut memory = Memory::new();

    let count_before = memory.token_count_async().await.unwrap();

    memory.add_async(make_user_message("Some text to count tokens for")).await.unwrap();

    let count_after = memory.token_count_async().await.unwrap();
    assert!(count_after > count_before);
}

// ---------------------------------------------------------------------------
// Memory with session -- async
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_async_with_session_scoped_messages() {
    let mut mem1 = Memory::new().with_session("session-A");
    let mut mem2 = Memory::new().with_session("session-B");

    mem1.add_async(make_user_message("From session A")).await.unwrap();
    mem2.add_async(make_user_message("From session B")).await.unwrap();

    let msgs1 = mem1.messages_async().await.unwrap();
    let msgs2 = mem2.messages_async().await.unwrap();

    assert_eq!(msgs1.len(), 1);
    assert_eq!(msgs1[0].content, "From session A");

    assert_eq!(msgs2.len(), 1);
    assert_eq!(msgs2[0].content, "From session B");
}

// ---------------------------------------------------------------------------
// needs_optimization_async
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_needs_optimization_async_default_false() {
    let memory = Memory::new();
    assert!(!memory.needs_optimization_async().await);
}

#[tokio::test]
async fn test_needs_optimization_async_after_many_messages() {
    // Use a very small max_tokens to trigger optimization based on token count.
    // The storage itself enforces max_messages, so we rely on the token budget.
    let config = MemoryConfig {
        max_messages: 100,
        max_tokens: Some(5), // tiny token budget
        enable_optimization: true,
        optimization_target: 1,
    };
    let mut memory = Memory::with_config(config);

    // Add enough text to exceed 5 tokens (estimate: 1 token per 4 chars)
    memory
        .add_async(make_user_message("This is a long message that should definitely exceed a five-token budget"))
        .await
        .unwrap();

    assert!(memory.needs_optimization_async().await);
}

#[tokio::test]
async fn test_needs_optimization_async_disabled() {
    let config = MemoryConfig {
        max_messages: 1,
        max_tokens: None,
        enable_optimization: false,
        optimization_target: 0,
    };
    let mut memory = Memory::with_config(config);

    for i in 0..10 {
        memory
            .add_async(make_user_message(&format!("msg {}", i)))
            .await
            .unwrap();
    }

    // Even though we have many messages, optimization is disabled
    assert!(!memory.needs_optimization_async().await);
}

// ---------------------------------------------------------------------------
// SharedMemory async operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_shared_memory_new_is_empty() {
    let shared = SharedMemory::new();
    assert!(shared.is_empty().await);
    assert_eq!(shared.len().await, 0);
}

#[tokio::test]
async fn test_shared_memory_add_and_retrieve() {
    let shared = SharedMemory::new();

    shared.add(make_user_message("Shared msg")).await.unwrap();

    let messages = shared.messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Shared msg");
}

#[tokio::test]
async fn test_shared_memory_multiple_adds() {
    let shared = SharedMemory::new();

    shared.add(make_user_message("One")).await.unwrap();
    shared.add(make_assistant_message("Two")).await.unwrap();
    shared.add(make_user_message("Three")).await.unwrap();

    assert_eq!(shared.len().await, 3);
    assert!(!shared.is_empty().await);
}

#[tokio::test]
async fn test_shared_memory_clear() {
    let shared = SharedMemory::new();

    shared.add(make_user_message("Hello")).await.unwrap();
    shared.add(make_user_message("World")).await.unwrap();
    assert_eq!(shared.len().await, 2);

    shared.clear().await.unwrap();
    assert!(shared.is_empty().await);
    assert_eq!(shared.len().await, 0);
}

#[tokio::test]
async fn test_shared_memory_from_memory() {
    let mut memory = Memory::new();
    memory.add_async(make_user_message("Pre-existing")).await.unwrap();

    let shared = SharedMemory::from_memory(memory);

    let messages = shared.messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Pre-existing");
}

#[tokio::test]
async fn test_shared_memory_clone_shares_data() {
    let shared = SharedMemory::new();
    let clone = shared.clone();

    shared.add(make_user_message("From original")).await.unwrap();

    // Clone should see the same data
    let messages = clone.messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "From original");
}

#[tokio::test]
async fn test_shared_memory_concurrent_adds() {
    let shared = SharedMemory::new();

    let mut handles = vec![];
    for i in 0..10 {
        let shared_clone = shared.clone();
        let handle = tokio::spawn(async move {
            shared_clone
                .add(make_user_message(&format!("Concurrent {}", i)))
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(shared.len().await, 10);

    let messages = shared.messages().await.unwrap();
    assert_eq!(messages.len(), 10);
}

#[tokio::test]
async fn test_shared_memory_clear_then_add() {
    let shared = SharedMemory::new();

    shared.add(make_user_message("Before")).await.unwrap();
    shared.clear().await.unwrap();
    shared.add(make_user_message("After")).await.unwrap();

    let messages = shared.messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "After");
}

// ---------------------------------------------------------------------------
// SharedMemory default
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_shared_memory_default() {
    let shared = SharedMemory::default();
    assert!(shared.is_empty().await);
}
