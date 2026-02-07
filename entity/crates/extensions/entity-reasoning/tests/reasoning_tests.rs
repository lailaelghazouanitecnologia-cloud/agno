use entity_reasoning::*;

#[test]
fn test_reasoning_strategy_variants() {
    let strategies = vec![
        ReasoningStrategy::Default,
        ReasoningStrategy::DeepThink,
        ReasoningStrategy::TreeOfThought,
        ReasoningStrategy::StepByStep,
        ReasoningStrategy::Reflection,
    ];
    for strategy in strategies {
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: ReasoningStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, strategy);
    }
}

#[test]
fn test_reasoning_config_default() {
    let config = ReasoningConfig::default();
    assert_eq!(config.strategy, ReasoningStrategy::Default);
    assert_eq!(config.max_steps, 10);
    assert!(!config.show_thinking);
    assert!(config.thinking_budget_tokens.is_none());
}

#[test]
fn test_reasoning_config_builder() {
    let config = ReasoningConfig::new()
        .with_strategy(ReasoningStrategy::DeepThink)
        .with_max_steps(20)
        .with_thinking_budget(10000)
        .with_show_thinking(true);

    assert_eq!(config.strategy, ReasoningStrategy::DeepThink);
    assert_eq!(config.max_steps, 20);
    assert_eq!(config.thinking_budget_tokens, Some(10000));
    assert!(config.show_thinking);
}

#[test]
fn test_thinking_step_new() {
    let step = ThinkingStep::new(1, "The problem requires sorting.");
    assert_eq!(step.step_number, 1);
    assert_eq!(step.thought, "The problem requires sorting.");
    assert!(step.conclusion.is_none());
    assert_eq!(step.confidence, 0.0);
}

#[test]
fn test_thinking_step_builder() {
    let step = ThinkingStep::new(2, "Analyzing options")
        .with_conclusion("Option B is best")
        .with_confidence(0.85);

    assert_eq!(step.conclusion, Some("Option B is best".to_string()));
    assert!((step.confidence - 0.85).abs() < f32::EPSILON);
}

#[test]
fn test_thinking_step_confidence_clamping() {
    let over = ThinkingStep::new(1, "t").with_confidence(1.5);
    assert!((over.confidence - 1.0).abs() < f32::EPSILON);

    let under = ThinkingStep::new(1, "t").with_confidence(-0.5);
    assert!((under.confidence - 0.0).abs() < f32::EPSILON);
}

#[test]
fn test_reasoning_status_variants() {
    let statuses = vec![
        ReasoningStatus::InProgress,
        ReasoningStatus::Complete,
        ReasoningStatus::MaxStepsReached,
        ReasoningStatus::Aborted,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: ReasoningStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }
}

#[test]
fn test_reasoning_chain_new() {
    let config = ReasoningConfig::new().with_max_steps(5);
    let chain = ReasoningChain::new(config);

    assert!(!chain.id.is_empty());
    assert_eq!(chain.config.max_steps, 5);
    assert!(chain.steps.is_empty());
    assert_eq!(chain.total_thinking_tokens, 0);
    assert_eq!(chain.status, ReasoningStatus::InProgress);
    assert!(!chain.is_complete());
}

#[test]
fn test_reasoning_chain_add_steps() {
    let config = ReasoningConfig::new().with_max_steps(10);
    let mut chain = ReasoningChain::new(config);

    chain.add_step(ThinkingStep::new(1, "First thought"));
    chain.add_step(ThinkingStep::new(2, "Second thought"));

    assert_eq!(chain.step_count(), 2);
    assert_eq!(chain.status, ReasoningStatus::InProgress);
}

#[test]
fn test_reasoning_chain_max_steps_reached() {
    let config = ReasoningConfig::new().with_max_steps(2);
    let mut chain = ReasoningChain::new(config);

    chain.add_step(ThinkingStep::new(1, "One"));
    assert_eq!(chain.status, ReasoningStatus::InProgress);

    chain.add_step(ThinkingStep::new(2, "Two"));
    assert_eq!(chain.status, ReasoningStatus::MaxStepsReached);
    assert!(chain.is_complete());
}

#[test]
fn test_reasoning_chain_current_step() {
    let config = ReasoningConfig::new();
    let mut chain = ReasoningChain::new(config);

    assert!(chain.current_step().is_none());

    chain.add_step(ThinkingStep::new(1, "First"));
    chain.add_step(ThinkingStep::new(2, "Second"));

    let current = chain.current_step().unwrap();
    assert_eq!(current.step_number, 2);
    assert_eq!(current.thought, "Second");
}

#[test]
fn test_reasoning_chain_complete() {
    let config = ReasoningConfig::new();
    let mut chain = ReasoningChain::new(config);
    chain.add_step(ThinkingStep::new(1, "Done thinking"));

    chain.complete();
    assert_eq!(chain.status, ReasoningStatus::Complete);
    assert!(chain.is_complete());
}

#[test]
fn test_reasoning_chain_abort() {
    let config = ReasoningConfig::new();
    let mut chain = ReasoningChain::new(config);

    chain.abort();
    assert_eq!(chain.status, ReasoningStatus::Aborted);
    assert!(chain.is_complete());
}

#[test]
fn test_reasoning_chain_add_thinking_tokens() {
    let config = ReasoningConfig::new();
    let mut chain = ReasoningChain::new(config);

    chain.add_thinking_tokens(500);
    chain.add_thinking_tokens(300);
    assert_eq!(chain.total_thinking_tokens, 800);
}

#[test]
fn test_reasoning_chain_to_reasoning_steps() {
    let config = ReasoningConfig::new();
    let mut chain = ReasoningChain::new(config);

    chain.add_step(ThinkingStep::new(1, "Analyze the problem"));
    chain.add_step(
        ThinkingStep::new(2, "Consider alternatives")
            .with_conclusion("Option A wins"),
    );

    let steps = chain.to_reasoning_steps();
    assert_eq!(steps.len(), 2);

    assert_eq!(steps[0].step_number, 1);
    assert_eq!(steps[0].content, "Analyze the problem");

    assert_eq!(steps[1].step_number, 2);
    assert!(steps[1].content.contains("Consider alternatives"));
    assert!(steps[1].content.contains("Conclusion: Option A wins"));
}

#[test]
fn test_reasoning_chain_serialization() {
    let config = ReasoningConfig::new().with_strategy(ReasoningStrategy::StepByStep);
    let mut chain = ReasoningChain::new(config);
    chain.add_step(ThinkingStep::new(1, "Step one").with_confidence(0.9));

    let json = serde_json::to_string(&chain).unwrap();
    let parsed: ReasoningChain = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.config.strategy, ReasoningStrategy::StepByStep);
    assert_eq!(parsed.step_count(), 1);
    assert!((parsed.steps[0].confidence - 0.9).abs() < f32::EPSILON);
}

#[test]
fn test_thinking_step_serialization() {
    let step = ThinkingStep::new(3, "Deep analysis")
        .with_conclusion("Result found")
        .with_confidence(0.95);

    let json = serde_json::to_string(&step).unwrap();
    let parsed: ThinkingStep = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.step_number, 3);
    assert_eq!(parsed.thought, "Deep analysis");
    assert_eq!(parsed.conclusion, Some("Result found".to_string()));
}
