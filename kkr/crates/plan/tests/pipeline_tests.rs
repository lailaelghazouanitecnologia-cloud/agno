use kkr_plan::{Condition, FileAction, Pipeline, PipelinePhase, Plan, Step};

#[test]
fn pipeline_c_compiler() {
    let mut pipeline = Pipeline::new(
        "Build C Compiler",
        "Create a complete C11 compiler targeting x86_64",
    )
    .with_description("Multi-phase compiler construction")
    .with_creator("architect");

    // Phase 1: Research
    let mut research_plan = Plan::new("Analyze requirements", "Study C11 spec");
    research_plan.add_step(
        Step::new("read-spec", "Extract grammar rules from C11 standard").with_depth(0),
    );
    research_plan.add_step(
        Step::new("study-ref", "Analyze tcc and chibicc reference compilers")
            .with_depth(0)
            .after("read-spec".to_string()),
    );

    pipeline.add_phase(
        PipelinePhase::new("research", "Analyze C11 spec and reference compilers")
            .with_plan(research_plan)
            .with_gate(Condition::custom(
                "List of features to implement identified",
            )),
    );

    // Phase 2: Lexer
    let mut lexer_plan = Plan::new("Implement lexer", "C11 tokenizer");
    lexer_plan.add_step(
        Step::new("token-types", "Define Token enum with all C11 token variants")
            .with_file("src/token.rs", FileAction::Create)
            .with_depth(2),
    );
    lexer_plan.add_step(
        Step::new("scanner", "Implement character scanner with peek/advance")
            .with_file("src/lexer.rs", FileAction::Create)
            .with_depth(3)
            .after("token-types".to_string()),
    );
    lexer_plan.add_step(
        Step::new("lexer-tests", "Test all token types including edge cases")
            .with_file("tests/lexer_test.rs", FileAction::Create)
            .with_depth(3)
            .after("scanner".to_string())
            .with_postcondition(Condition::command_succeeds("cargo test")),
    );

    pipeline.add_phase(
        PipelinePhase::new("lexer", "Implement C11 tokenizer")
            .with_plan(lexer_plan)
            .after("research".to_string())
            .with_gate(Condition::command_succeeds("cargo test -- lexer")),
    );

    // Phase 3: Parser
    pipeline.add_phase(
        PipelinePhase::new("parser", "Implement recursive descent parser")
            .after("lexer".to_string())
            .with_gate(Condition::command_succeeds("cargo test -- parser")),
    );

    assert_eq!(pipeline.phases.len(), 3);
    assert_eq!(pipeline.progress(), 0.0);

    // Only research is ready (no deps)
    let ready = pipeline.ready_phases();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].name, "research");

    // Complete research
    pipeline.phases[0].mark_completed();

    // Now lexer is ready
    let ready = pipeline.ready_phases();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].name, "lexer");

    // Parser has deferred plan
    assert!(pipeline.phases[2].plan.is_none());
}

#[test]
fn pipeline_progress() {
    let mut pipeline = Pipeline::new("test", "task");
    pipeline.add_phase(PipelinePhase::new("a", "Phase A"));
    pipeline.add_phase(PipelinePhase::new("b", "Phase B").after("a".to_string()));

    assert_eq!(pipeline.progress(), 0.0);
    pipeline.phases[0].mark_completed();
    assert_eq!(pipeline.progress(), 0.5);
    pipeline.phases[1].mark_completed();
    assert_eq!(pipeline.progress(), 1.0);
    assert!(pipeline.is_done());
}

#[test]
fn render_output() {
    let mut pipeline =
        Pipeline::new("Build X", "Create X from scratch").with_description("Three phase build");
    pipeline.add_phase(PipelinePhase::new("setup", "Initialize project"));
    pipeline.add_phase(
        PipelinePhase::new("build", "Implement core")
            .after("setup".to_string())
            .with_gate(Condition::command_succeeds("cargo check")),
    );

    let out = pipeline.render();
    assert!(out.contains("Build X"));
    assert!(out.contains("setup"));
    assert!(out.contains("build"));
    assert!(out.contains("Gate:"));
    assert!(out.contains("cargo check"));
}
