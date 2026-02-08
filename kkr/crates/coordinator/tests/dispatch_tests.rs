use kkr_coordinator::{AgentHandle, AgentPool};
use kkr_project::AgentRole;

#[test]
fn test_agent_pool() {
    let mut pool = AgentPool::new();
    pool.register(AgentHandle::worker("worker-1"));
    pool.register(AgentHandle::worker("worker-2"));
    pool.register(AgentHandle::validator("validator-1"));

    assert_eq!(pool.available_count(AgentRole::Worker), 2);
    assert_eq!(pool.available_count(AgentRole::Validator), 1);

    pool.mark_busy("worker-1");
    assert_eq!(pool.available_count(AgentRole::Worker), 1);

    pool.mark_available("worker-1");
    assert_eq!(pool.available_count(AgentRole::Worker), 2);
}

#[test]
fn test_get_available_agent() {
    let mut pool = AgentPool::new();
    pool.register(AgentHandle::worker("w1"));
    pool.register(AgentHandle::validator("v1"));

    let worker = pool.get_available(AgentRole::Worker);
    assert!(worker.is_some());
    assert_eq!(worker.unwrap().name, "w1");

    let validator = pool.get_available(AgentRole::Validator);
    assert!(validator.is_some());
}
