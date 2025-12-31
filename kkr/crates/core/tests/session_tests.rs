//! Session tests

use kkr_core::session::{
    AgentSession, InMemorySessionStorage, RunOutput, SessionStorage, WorkflowRunOutput,
    WorkflowSession,
};
use kkr_core::Status;

#[test]
fn test_agent_session() {
    let mut session = AgentSession::new("test-session")
        .with_agent("agent-1")
        .with_user("user-1");

    let run = RunOutput::new("run-1")
        .with_agent("agent-1")
        .with_input("Hello")
        .complete("Hi there!");

    session.upsert_run(run);

    assert_eq!(session.runs.len(), 1);
    assert_eq!(
        session.get_run("run-1").unwrap().content,
        Some("Hi there!".to_string())
    );
}

#[test]
fn test_session_state() {
    let mut session = AgentSession::new("test-session");

    session.set_state("counter", 42);
    session.set_state("name", "test");

    assert_eq!(session.get_state::<i32>("counter"), Some(42));
    assert_eq!(
        session.get_state::<String>("name"),
        Some("test".to_string())
    );
}

#[test]
fn test_workflow_session() {
    let mut session =
        WorkflowSession::new("wf-session").with_workflow("workflow-1", Some("Test Workflow".to_string()));

    let mut run = WorkflowRunOutput::new("run-1");
    run.input = Some("Process data".to_string());
    run.content = Some("Data processed".to_string());
    run.status = Status::Completed;

    session.upsert_run(run);

    let history = session.get_history(None);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], ("Process data", "Data processed"));
}

#[test]
fn test_workflow_history_context() {
    let mut session = WorkflowSession::new("wf-session");

    let mut run1 = WorkflowRunOutput::new("run-1");
    run1.input = Some("First input".to_string());
    run1.content = Some("First response".to_string());
    run1.status = Status::Completed;
    session.upsert_run(run1);

    let mut run2 = WorkflowRunOutput::new("run-2");
    run2.input = Some("Second input".to_string());
    run2.content = Some("Second response".to_string());
    run2.status = Status::Completed;
    session.upsert_run(run2);

    let context = session.get_history_context(None).unwrap();
    assert!(context.contains("<workflow_history_context>"));
    assert!(context.contains("First input"));
    assert!(context.contains("Second response"));
}

#[test]
fn test_run_output_lifecycle() {
    let run = RunOutput::new("run-1")
        .with_agent("agent-1")
        .with_input("Hello");

    assert_eq!(run.status, Status::Running);
    assert!(run.completed_at.is_none());

    let completed_run = run.complete("Done!");
    assert_eq!(completed_run.status, Status::Completed);
    assert!(completed_run.completed_at.is_some());
}

#[test]
fn test_run_output_failure() {
    let run = RunOutput::new("run-1").with_input("Bad input");

    let failed_run = run.fail("Something went wrong");
    assert_eq!(failed_run.status, Status::Failed);
    assert_eq!(failed_run.error, Some("Something went wrong".to_string()));
}

#[tokio::test]
async fn test_in_memory_storage() {
    let mut storage = InMemorySessionStorage::new();

    let session = AgentSession::new("session-1").with_user("user-1");
    storage.store_agent_session(&session).await.unwrap();

    assert_eq!(storage.agent_count(), 1);

    let retrieved = storage.get_agent_session("session-1").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().session_id, "session-1");

    let user_sessions = storage.list_sessions_by_user("user-1").await.unwrap();
    assert_eq!(user_sessions.len(), 1);
}

#[tokio::test]
async fn test_workflow_storage() {
    let mut storage = InMemorySessionStorage::new();

    let session = WorkflowSession::new("wf-session-1")
        .with_user("user-1")
        .with_workflow("wf-1", Some("Test".to_string()));

    storage.store_workflow_session(&session).await.unwrap();

    assert_eq!(storage.workflow_count(), 1);

    let retrieved = storage.get_workflow_session("wf-session-1").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().workflow_id, Some("wf-1".to_string()));
}

#[tokio::test]
async fn test_storage_delete() {
    let mut storage = InMemorySessionStorage::new();

    let session = AgentSession::new("session-1");
    storage.store_agent_session(&session).await.unwrap();

    assert!(storage.delete_agent_session("session-1").await.unwrap());
    assert!(!storage.delete_agent_session("session-1").await.unwrap());

    let retrieved = storage.get_agent_session("session-1").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_storage_clear() {
    let mut storage = InMemorySessionStorage::new();

    storage
        .store_agent_session(&AgentSession::new("s1"))
        .await
        .unwrap();
    storage
        .store_agent_session(&AgentSession::new("s2"))
        .await
        .unwrap();
    storage
        .store_workflow_session(&WorkflowSession::new("wf1"))
        .await
        .unwrap();

    assert_eq!(storage.agent_count(), 2);
    assert_eq!(storage.workflow_count(), 1);

    storage.clear();

    assert_eq!(storage.agent_count(), 0);
    assert_eq!(storage.workflow_count(), 0);
}
