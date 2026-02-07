use common_error::ErrorKind;
use entity_run::RunStatus;
use entity_team::*;

// ── TeamMember tests ──

#[test]
fn test_team_member_new() {
    let member = TeamMember::new("agent-1", "Descriptor");
    assert_eq!(member.agent_id, "agent-1");
    assert_eq!(member.name, "Descriptor");
    assert!(member.role.is_none());
    assert!(member.description.is_none());
}

#[test]
fn test_team_member_with_role() {
    let member = TeamMember::new("agent-2", "Planner").with_role("planning");
    assert_eq!(member.role, Some("planning".to_string()));
}

#[test]
fn test_team_member_with_description() {
    let member =
        TeamMember::new("agent-3", "Coder").with_description("Writes production code");
    assert_eq!(
        member.description,
        Some("Writes production code".to_string())
    );
}

#[test]
fn test_team_member_full_builder() {
    let member = TeamMember::new("agent-4", "Reviewer")
        .with_role("review")
        .with_description("Reviews pull requests");

    assert_eq!(member.agent_id, "agent-4");
    assert_eq!(member.name, "Reviewer");
    assert_eq!(member.role, Some("review".to_string()));
    assert_eq!(
        member.description,
        Some("Reviews pull requests".to_string())
    );
}

// ── TeamConfig tests ──

#[test]
fn test_team_config_default() {
    let config = TeamConfig::default();
    assert_eq!(config.mode, TeamMode::Route);
    assert!(!config.respond_directly);
    assert!(!config.delegate_to_all);
    assert_eq!(config.max_rounds, 3);
}

// ── TeamMode tests ──

#[test]
fn test_team_mode_serialize_deserialize() {
    let modes = vec![TeamMode::Route, TeamMode::Coordinate, TeamMode::Collaborate];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: TeamMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }
}

#[test]
fn test_team_mode_json_values() {
    assert_eq!(serde_json::to_string(&TeamMode::Route).unwrap(), "\"route\"");
    assert_eq!(
        serde_json::to_string(&TeamMode::Coordinate).unwrap(),
        "\"coordinate\""
    );
    assert_eq!(
        serde_json::to_string(&TeamMode::Collaborate).unwrap(),
        "\"collaborate\""
    );
}

// ── Team tests ──

#[test]
fn test_team_new() {
    let team = Team::new("my-team");
    assert_eq!(team.name, "my-team");
    assert!(!team.id.is_empty());
    assert_eq!(team.config.mode, TeamMode::Route);
    assert_eq!(team.member_count(), 0);
    assert!(team.leader_model.is_none());
    assert!(team.description.is_none());
}

#[test]
fn test_team_with_mode() {
    let team = Team::new("t").with_mode(TeamMode::Collaborate);
    assert_eq!(team.config.mode, TeamMode::Collaborate);
}

#[test]
fn test_team_add_member() {
    let team = Team::new("t")
        .add_member(TeamMember::new("a1", "Alpha"))
        .add_member(TeamMember::new("a2", "Beta"));

    assert_eq!(team.member_count(), 2);
    assert_eq!(team.members[0].name, "Alpha");
    assert_eq!(team.members[1].name, "Beta");
}

#[test]
fn test_team_with_leader() {
    let team = Team::new("t").with_leader("gpt-4o");
    assert_eq!(team.leader_model, Some("gpt-4o".to_string()));
}

#[test]
fn test_team_with_description() {
    let team = Team::new("t").with_description("A coding team");
    assert_eq!(team.description, Some("A coding team".to_string()));
}

#[test]
fn test_team_builder_chain() {
    let team = Team::new("code-team")
        .with_mode(TeamMode::Coordinate)
        .with_leader("claude-3")
        .with_description("Coordinates coding tasks")
        .add_member(TeamMember::new("a1", "Descriptor").with_role("describe"))
        .add_member(TeamMember::new("a2", "Planner").with_role("plan"))
        .add_member(TeamMember::new("a3", "Coder").with_role("code"));

    assert_eq!(team.name, "code-team");
    assert_eq!(team.config.mode, TeamMode::Coordinate);
    assert_eq!(team.leader_model, Some("claude-3".to_string()));
    assert_eq!(
        team.description,
        Some("Coordinates coding tasks".to_string())
    );
    assert_eq!(team.member_count(), 3);
}

#[test]
fn test_team_get_member_found() {
    let team = Team::new("t")
        .add_member(TeamMember::new("a1", "Agent One"))
        .add_member(TeamMember::new("a2", "Agent Two"));

    let member = team.get_member("a2").unwrap();
    assert_eq!(member.name, "Agent Two");
}

#[test]
fn test_team_get_member_not_found() {
    let team = Team::new("t").add_member(TeamMember::new("a1", "Agent One"));
    assert!(team.get_member("nonexistent").is_none());
}

#[test]
fn test_team_member_count_empty() {
    let team = Team::new("empty");
    assert_eq!(team.member_count(), 0);
}

// ── Team validation tests ──

#[test]
fn test_team_validate_success() {
    let team = Team::new("valid-team")
        .add_member(TeamMember::new("a1", "Alpha"))
        .add_member(TeamMember::new("a2", "Beta"));

    assert!(team.validate().is_ok());
}

#[test]
fn test_team_validate_empty_name() {
    let team = Team::new("");
    let err = team.validate().unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidValue);
    assert!(err.message.contains("name"));
}

#[test]
fn test_team_validate_zero_max_rounds() {
    let mut team = Team::new("t");
    team.config.max_rounds = 0;

    let err = team.validate().unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidValue);
    assert!(err.message.contains("max_rounds"));
}

#[test]
fn test_team_validate_duplicate_agent_ids() {
    let team = Team::new("t")
        .add_member(TeamMember::new("dup", "First"))
        .add_member(TeamMember::new("dup", "Second"));

    let err = team.validate().unwrap_err();
    assert_eq!(err.kind, ErrorKind::AlreadyExists);
    assert!(err.message.contains("dup"));
}

#[test]
fn test_team_validate_no_members_is_ok() {
    let team = Team::new("lonely");
    assert!(team.validate().is_ok());
}

// ── TeamRunOutput tests ──

#[test]
fn test_team_run_output_new() {
    let output = TeamRunOutput::new("team-1");
    assert_eq!(output.team_id, "team-1");
    assert!(!output.run_id.is_empty());
    assert_eq!(output.status, RunStatus::Running);
    assert!(output.member_outputs.is_empty());
    assert!(output.final_output.is_none());
}

#[test]
fn test_team_run_output_add_member_output() {
    let mut output = TeamRunOutput::new("team-1");
    output.add_member_output("agent-1", "result from agent-1");
    output.add_member_output("agent-2", "result from agent-2");

    assert_eq!(output.member_outputs.len(), 2);
    assert_eq!(output.member_outputs["agent-1"], "result from agent-1");
    assert_eq!(output.member_outputs["agent-2"], "result from agent-2");
}

#[test]
fn test_team_run_output_overwrites_same_member() {
    let mut output = TeamRunOutput::new("team-1");
    output.add_member_output("agent-1", "first");
    output.add_member_output("agent-1", "second");

    assert_eq!(output.member_outputs.len(), 1);
    assert_eq!(output.member_outputs["agent-1"], "second");
}

#[test]
fn test_team_run_output_complete() {
    let output = TeamRunOutput::new("team-1").complete("final answer");
    assert!(output.is_success());
    assert_eq!(output.final_output, Some("final answer".to_string()));
    assert_eq!(output.status, RunStatus::Completed);
}

#[test]
fn test_team_run_output_fail() {
    let output = TeamRunOutput::new("team-1").fail();
    assert!(!output.is_success());
    assert_eq!(output.status, RunStatus::Failed);
}

// ── Serialization roundtrip tests ──

#[test]
fn test_team_serialization_roundtrip() {
    let team = Team::new("serialize-me")
        .with_mode(TeamMode::Coordinate)
        .add_member(TeamMember::new("a", "A").with_role("coder"));

    let json = serde_json::to_string(&team).unwrap();
    let parsed: Team = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "serialize-me");
    assert_eq!(parsed.config.mode, TeamMode::Coordinate);
    assert_eq!(parsed.member_count(), 1);
    assert_eq!(parsed.members[0].role, Some("coder".to_string()));
}

#[test]
fn test_team_run_output_serialization_roundtrip() {
    let mut output = TeamRunOutput::new("team-x");
    output.add_member_output("a1", "hello");
    let output = output.complete("done");

    let json = serde_json::to_string(&output).unwrap();
    let parsed: TeamRunOutput = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.team_id, "team-x");
    assert_eq!(parsed.status, RunStatus::Completed);
    assert_eq!(parsed.member_outputs["a1"], "hello");
    assert_eq!(parsed.final_output, Some("done".to_string()));
}
