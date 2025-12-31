use kkr_core::new_id;
use kkr_core::team::{team, Team, TeamBuilder, TeamConfig, TeamMember, TeamMode, TeamResult};
use kkr_core::Output;

#[test]
fn test_team_config_creation() {
    let config = TeamConfig::new("TestTeam");

    assert_eq!(config.name, "TestTeam");
    assert_eq!(config.mode, TeamMode::Sequential);
    assert_eq!(config.max_rounds, 5);
    assert!(!config.share_memory);
}

#[test]
fn test_team_config_builder() {
    let config = TeamConfig::new("TestTeam")
        .with_mode(TeamMode::Parallel)
        .with_max_rounds(10)
        .with_shared_memory(true);

    assert_eq!(config.mode, TeamMode::Parallel);
    assert_eq!(config.max_rounds, 10);
    assert!(config.share_memory);
}

#[test]
fn test_team_mode_default() {
    let mode = TeamMode::default();
    assert_eq!(mode, TeamMode::Sequential);
}

#[test]
fn test_team_member_creation() {
    let id = new_id();
    let member = TeamMember::new(id, "Agent1");

    assert_eq!(member.id, id);
    assert_eq!(member.name, "Agent1");
    assert!(member.role.is_none());
}

#[test]
fn test_team_member_with_role() {
    let member = TeamMember::new(new_id(), "Agent1").with_role("researcher");

    assert_eq!(member.role.unwrap(), "researcher");
}

#[test]
fn test_team_result_success() {
    let team_id = new_id();
    let outputs = vec![
        Output::success(new_id(), "done1".to_string()),
        Output::success(new_id(), "done2".to_string()),
    ];

    let result = TeamResult::success(team_id, outputs, 2);

    assert!(result.is_success());
    assert!(!result.is_failure());
    assert_eq!(result.rounds_executed, 2);
    assert!(result.error.is_none());
    assert!(result.final_output().is_some());
}

#[test]
fn test_team_result_failure() {
    let result = TeamResult::failure(new_id(), "something went wrong");

    assert!(result.is_failure());
    assert!(!result.is_success());
    assert!(result.error.is_some());
    assert!(result.final_output().is_none());
}

#[test]
fn test_team_builder() {
    let id1 = new_id();
    let id2 = new_id();

    let team = TeamBuilder::new("MyTeam")
        .mode(TeamMode::RoundRobin)
        .max_rounds(3)
        .share_memory(true)
        .member(id1, "Agent1")
        .member_with_role(id2, "Agent2", "leader")
        .build();

    assert_eq!(team.name(), "MyTeam");
    assert_eq!(team.mode(), TeamMode::RoundRobin);
    assert_eq!(team.member_count(), 2);
}

#[test]
fn test_team_builder_function() {
    let team = team("QuickTeam").mode(TeamMode::Parallel).build();

    assert_eq!(team.name(), "QuickTeam");
    assert_eq!(team.mode(), TeamMode::Parallel);
}

#[test]
fn test_team_add_remove_member() {
    let mut team = Team::default();
    let id = new_id();

    team.add_member(TeamMember::new(id, "Agent1"));
    assert_eq!(team.member_count(), 1);

    let removed = team.remove_member(id);
    assert!(removed.is_some());
    assert_eq!(team.member_count(), 0);
}

#[test]
fn test_team_member_lookup() {
    let id = new_id();
    let mut team = Team::default();
    team.add_member(TeamMember::new(id, "Agent1").with_role("worker"));

    let member = team.member(id);
    assert!(member.is_some());
    assert_eq!(member.unwrap().name, "Agent1");
    assert_eq!(member.unwrap().role.as_ref().unwrap(), "worker");
}

#[test]
fn test_team_members_list() {
    let mut team = Team::default();
    team.add_member(TeamMember::new(new_id(), "Agent1"));
    team.add_member(TeamMember::new(new_id(), "Agent2"));
    team.add_member(TeamMember::new(new_id(), "Agent3"));

    let members = team.members();
    assert_eq!(members.len(), 3);
}

#[test]
fn test_team_context() {
    let mut team = Team::default();

    team.set_context("task", serde_json::json!({"goal": "analyze data"}));
    team.set_context("iteration", serde_json::json!(1));

    assert!(team.get_context("task").is_some());
    assert!(team.get_context("iteration").is_some());
    assert!(team.get_context("missing").is_none());

    team.clear_context();
    assert!(team.get_context("task").is_none());
}

#[test]
fn test_team_default() {
    let team = Team::default();

    assert_eq!(team.name(), "Team");
    assert_eq!(team.mode(), TeamMode::Sequential);
    assert_eq!(team.member_count(), 0);
}

#[test]
fn test_team_builder_default() {
    let builder = TeamBuilder::default();
    let team = builder.build();

    assert_eq!(team.name(), "Team");
}

#[test]
fn test_team_config_default() {
    let config = TeamConfig::default();

    assert_eq!(config.name, "Team");
    assert_eq!(config.mode, TeamMode::Sequential);
}

#[test]
fn test_team_remove_nonexistent() {
    let mut team = Team::default();
    let removed = team.remove_member(new_id());

    assert!(removed.is_none());
}
