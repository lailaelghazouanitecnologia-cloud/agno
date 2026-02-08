use kkr_inner::{CostTier, Phase, PhaseResult};

#[test]
fn standard_order() {
    let order = Phase::standard_order();
    assert_eq!(order.len(), 5);
    assert_eq!(order[0], Phase::Perception);
    assert_eq!(order[4], Phase::Reflection);
}

#[test]
fn cost_tiers() {
    assert_eq!(Phase::Perception.cost_tier(), CostTier::Cheap);
    assert_eq!(Phase::Deliberation.cost_tier(), CostTier::Expensive);
    assert_eq!(Phase::Execution.cost_tier(), CostTier::Variable);
}

#[test]
fn phase_result_halt() {
    let r = PhaseResult::new(Phase::Simulation, "risks found").halt();
    assert!(!r.continue_to_next);
}
