use kkr_inner::{Role, DELIBERATION_SEQUENCE};

#[test]
fn role_labels() {
    assert_eq!(Role::Analyst.label(), "analyst");
    assert_eq!(Role::Architect.label(), "architect");
    assert_eq!(Role::Critic.label(), "critic");
    assert_eq!(Role::Coder.label(), "coder");
    assert_eq!(Role::Reviewer.label(), "reviewer");
}

#[test]
fn deliberation_sequence_has_revision() {
    // Architect appears twice: initial + revision after critic
    let architect_count = DELIBERATION_SEQUENCE
        .iter()
        .filter(|r| **r == Role::Architect)
        .count();
    assert_eq!(architect_count, 2);
}

#[test]
fn system_prompts_not_empty() {
    for role in &[Role::Analyst, Role::Architect, Role::Critic, Role::Coder, Role::Reviewer] {
        assert!(!role.system_prompt().is_empty());
    }
}
