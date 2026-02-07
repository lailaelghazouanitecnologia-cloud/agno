use kkr_tendencies::*;

// -- TendencyStrength Tests --

#[test]
fn test_tendency_strength_ordering() {
    assert!(TendencyStrength::Weak < TendencyStrength::Moderate);
    assert!(TendencyStrength::Moderate < TendencyStrength::Strong);
    assert!(TendencyStrength::Strong < TendencyStrength::VeryStrong);
    assert!(TendencyStrength::VeryStrong < TendencyStrength::Required);
}

#[test]
fn test_tendency_strength_weights() {
    assert_eq!(TendencyStrength::Weak.as_weight(), 0.25);
    assert_eq!(TendencyStrength::Moderate.as_weight(), 0.5);
    assert_eq!(TendencyStrength::Strong.as_weight(), 0.75);
    assert_eq!(TendencyStrength::VeryStrong.as_weight(), 0.9);
    assert_eq!(TendencyStrength::Required.as_weight(), 1.0);
}

#[test]
fn test_tendency_strength_is_required() {
    assert!(TendencyStrength::Required.is_required());
    assert!(!TendencyStrength::Strong.is_required());
    assert!(!TendencyStrength::Weak.is_required());
}

#[test]
fn test_tendency_strength_is_strong_or_above() {
    assert!(!TendencyStrength::Weak.is_strong_or_above());
    assert!(!TendencyStrength::Moderate.is_strong_or_above());
    assert!(TendencyStrength::Strong.is_strong_or_above());
    assert!(TendencyStrength::VeryStrong.is_strong_or_above());
    assert!(TendencyStrength::Required.is_strong_or_above());
}

#[test]
fn test_tendency_strength_default() {
    assert_eq!(TendencyStrength::default(), TendencyStrength::Moderate);
}

// -- Tendency Tests --

#[test]
fn test_tendency_creation() {
    let t = Tendency::new("test", "A test tendency");
    assert_eq!(t.name, "test");
    assert_eq!(t.description, "A test tendency");
    assert_eq!(t.strength, TendencyStrength::Moderate); // default
    assert!(t.category.is_none());
    assert!(t.conditions.is_empty());
    assert!(t.encourages.is_empty());
}

#[test]
fn test_tendency_builder() {
    let t = Tendency::new("code_quality", "Write clean code")
        .strength(TendencyStrength::Strong)
        .category("code_style")
        .when(TendencyCondition::Always)
        .encourage("write_clean_code")
        .encourage("add_tests");

    assert_eq!(t.name, "code_quality");
    assert_eq!(t.strength, TendencyStrength::Strong);
    assert_eq!(t.category.as_deref(), Some("code_style"));
    assert_eq!(t.conditions.len(), 1);
    assert_eq!(t.encourages.len(), 2);
}

// -- TendencyCondition Tests --

#[test]
fn test_condition_always() {
    let cond = TendencyCondition::Always;
    let ctx = TendencyContext::new();
    assert!(cond.matches(&ctx));
}

#[test]
fn test_condition_file_type_match() {
    let cond = TendencyCondition::FileType(vec!["rs".to_string(), "py".to_string()]);

    let ctx_rs = TendencyContext::new().file_type("rs");
    assert!(cond.matches(&ctx_rs));

    let ctx_py = TendencyContext::new().file_type("py");
    assert!(cond.matches(&ctx_py));

    let ctx_js = TendencyContext::new().file_type("js");
    assert!(!cond.matches(&ctx_js));
}

#[test]
fn test_condition_file_type_no_context() {
    let cond = TendencyCondition::FileType(vec!["rs".to_string()]);
    let ctx = TendencyContext::new();
    assert!(!cond.matches(&ctx));
}

#[test]
fn test_condition_tool() {
    let cond = TendencyCondition::Tool(vec!["shell".to_string(), "git".to_string()]);

    let ctx_shell = TendencyContext::new().tool("shell");
    assert!(cond.matches(&ctx_shell));

    let ctx_http = TendencyContext::new().tool("http");
    assert!(!cond.matches(&ctx_http));
}

#[test]
fn test_condition_project_type() {
    let cond = TendencyCondition::ProjectType(vec!["rust".to_string()]);

    let ctx_rust = TendencyContext::new().project_type("rust");
    assert!(cond.matches(&ctx_rust));

    let ctx_python = TendencyContext::new().project_type("python");
    assert!(!cond.matches(&ctx_python));
}

#[test]
fn test_condition_content_contains() {
    let cond = TendencyCondition::ContentContains("unsafe".to_string());

    let ctx_match = TendencyContext::new().content("This code uses unsafe blocks");
    assert!(cond.matches(&ctx_match));

    let ctx_no = TendencyContext::new().content("This code is safe");
    assert!(!cond.matches(&ctx_no));
}

#[test]
fn test_condition_custom() {
    let cond = TendencyCondition::Custom {
        key: "env".to_string(),
        value: "production".to_string(),
    };

    let ctx_prod = TendencyContext::new().meta("env", "production");
    assert!(cond.matches(&ctx_prod));

    let ctx_dev = TendencyContext::new().meta("env", "development");
    assert!(!cond.matches(&ctx_dev));
}

// -- TendencyContext Tests --

#[test]
fn test_tendency_context_builder() {
    let ctx = TendencyContext::new()
        .tool("shell")
        .file_type("rs")
        .project_type("rust")
        .content("some code here")
        .meta("version", "1.0");

    assert_eq!(ctx.tool.as_deref(), Some("shell"));
    assert_eq!(ctx.file_type.as_deref(), Some("rs"));
    assert_eq!(ctx.project_type.as_deref(), Some("rust"));
    assert_eq!(ctx.content.as_deref(), Some("some code here"));
    assert_eq!(ctx.metadata.get("version").map(|s| s.as_str()), Some("1.0"));
}

#[test]
fn test_tendency_context_default() {
    let ctx = TendencyContext::default();
    assert!(ctx.tool.is_none());
    assert!(ctx.file_type.is_none());
    assert!(ctx.project_type.is_none());
    assert!(ctx.content.is_none());
    assert!(ctx.metadata.is_empty());
}

// -- Tendency::applies_to Tests --

#[test]
fn test_tendency_applies_to_no_conditions() {
    let t = Tendency::new("always", "Always applies");
    let ctx = TendencyContext::new();
    assert!(t.applies_to(&ctx)); // No conditions = always applies
}

#[test]
fn test_tendency_applies_to_matching_conditions() {
    let t = Tendency::new("rust_only", "For Rust files")
        .when(TendencyCondition::FileType(vec!["rs".to_string()]));

    let ctx_rs = TendencyContext::new().file_type("rs");
    assert!(t.applies_to(&ctx_rs));

    let ctx_py = TendencyContext::new().file_type("py");
    assert!(!t.applies_to(&ctx_py));
}

#[test]
fn test_tendency_applies_to_all_conditions_must_match() {
    let t = Tendency::new("specific", "Very specific")
        .when(TendencyCondition::FileType(vec!["rs".to_string()]))
        .when(TendencyCondition::Tool(vec!["shell".to_string()]));

    let ctx_both = TendencyContext::new().file_type("rs").tool("shell");
    assert!(t.applies_to(&ctx_both));

    let ctx_only_file = TendencyContext::new().file_type("rs");
    assert!(!t.applies_to(&ctx_only_file)); // Both conditions must match
}

// -- TendencySet Tests --

#[test]
fn test_tendency_set_new_is_empty() {
    let set = TendencySet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn test_tendency_set_add_and_len() {
    let mut set = TendencySet::new();
    set.add(Tendency::new("t1", "first"));
    set.add(Tendency::new("t2", "second"));

    assert_eq!(set.len(), 2);
    assert!(!set.is_empty());
}

#[test]
fn test_tendency_set_applicable() {
    let mut set = TendencySet::new();
    set.add(
        Tendency::new("rust", "Rust tendencies")
            .when(TendencyCondition::FileType(vec!["rs".to_string()])),
    );
    set.add(
        Tendency::new("python", "Python tendencies")
            .when(TendencyCondition::FileType(vec!["py".to_string()])),
    );
    set.add(Tendency::new("always", "Always applies"));

    let ctx = TendencyContext::new().file_type("rs");
    let applicable = set.applicable(&ctx);

    assert_eq!(applicable.len(), 2); // "rust" and "always"
    let names: Vec<&str> = applicable.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"rust"));
    assert!(names.contains(&"always"));
}

#[test]
fn test_tendency_set_encouraged_actions() {
    let mut set = TendencySet::new();
    set.add(
        Tendency::new("strong", "Strong")
            .strength(TendencyStrength::Strong)
            .encourage("action_a")
            .encourage("action_b"),
    );
    set.add(
        Tendency::new("weak", "Weak")
            .strength(TendencyStrength::Weak)
            .encourage("action_a") // Overlapping action, should take max weight
            .encourage("action_c"),
    );

    let ctx = TendencyContext::new();
    let actions = set.encouraged_actions(&ctx);

    // action_a should have weight 0.75 (max of Strong=0.75 and Weak=0.25)
    let action_a = actions.iter().find(|(name, _)| name == "action_a");
    assert!(action_a.is_some());
    assert_eq!(action_a.unwrap().1, 0.75);

    // Results sorted by weight descending
    assert!(actions[0].1 >= actions[1].1);
}

#[test]
fn test_tendency_set_to_prompt() {
    let mut set = TendencySet::new();
    set.add(
        Tendency::new("concise", "Write concise code")
            .strength(TendencyStrength::Strong),
    );
    set.add(
        Tendency::new("secure", "Never hardcode secrets")
            .strength(TendencyStrength::Required),
    );

    let ctx = TendencyContext::new();
    let prompt = set.to_prompt(&ctx);

    assert!(prompt.contains("Preferences to follow:"));
    assert!(prompt.contains("concise"));
    assert!(prompt.contains("secure"));
    assert!(prompt.contains("Never hardcode secrets"));
}

#[test]
fn test_tendency_set_to_prompt_empty() {
    let set = TendencySet::new();
    let ctx = TendencyContext::new();
    let prompt = set.to_prompt(&ctx);
    assert!(prompt.is_empty());
}

#[test]
fn test_tendency_set_to_prompt_no_applicable() {
    let mut set = TendencySet::new();
    set.add(
        Tendency::new("rust_only", "For Rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()])),
    );

    let ctx = TendencyContext::new().file_type("py");
    let prompt = set.to_prompt(&ctx);
    assert!(prompt.is_empty());
}

#[test]
fn test_tendency_set_by_category() {
    let mut set = TendencySet::new();
    set.add(Tendency::new("t1", "first").category("safety"));
    set.add(Tendency::new("t2", "second").category("style"));
    set.add(Tendency::new("t3", "third").category("safety"));

    let safety = set.by_category("safety");
    assert_eq!(safety.len(), 2);

    let style = set.by_category("style");
    assert_eq!(style.len(), 1);
}

#[test]
fn test_tendency_set_all() {
    let mut set = TendencySet::new();
    set.add(Tendency::new("a", "first"));
    set.add(Tendency::new("b", "second"));

    let all = set.all();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_tendency_set_merge() {
    let mut set1 = TendencySet::new();
    set1.add(Tendency::new("a", "first"));

    let mut set2 = TendencySet::new();
    set2.add(Tendency::new("b", "second"));
    set2.add(Tendency::new("c", "third"));

    set1.merge(set2);
    assert_eq!(set1.len(), 3);
}

// -- Pre-built Tendency Sets --

#[test]
fn test_code_style_tendencies() {
    let set = code_style_tendencies();
    assert!(!set.is_empty());

    let ctx = TendencyContext::new();
    let applicable = set.applicable(&ctx);
    assert!(!applicable.is_empty());

    // All should have category "code_style"
    for t in applicable {
        assert_eq!(t.category.as_deref(), Some("code_style"));
    }
}

#[test]
fn test_safety_tendencies() {
    let set = safety_tendencies();
    assert!(!set.is_empty());

    // Should include a Required one
    let required: Vec<_> = set
        .all()
        .iter()
        .filter(|t| t.strength.is_required())
        .collect();
    assert!(!required.is_empty());
}

#[test]
fn test_rust_tendencies_apply_only_to_rust() {
    let set = rust_tendencies();

    let ctx_rs = TendencyContext::new().file_type("rs");
    let applicable_rs = set.applicable(&ctx_rs);
    assert!(!applicable_rs.is_empty());

    let ctx_py = TendencyContext::new().file_type("py");
    let applicable_py = set.applicable(&ctx_py);
    assert!(applicable_py.is_empty());
}

#[test]
fn test_python_tendencies_apply_only_to_python() {
    let set = python_tendencies();

    let ctx_py = TendencyContext::new().file_type("py");
    let applicable_py = set.applicable(&ctx_py);
    assert!(!applicable_py.is_empty());

    let ctx_rs = TendencyContext::new().file_type("rs");
    let applicable_rs = set.applicable(&ctx_rs);
    assert!(applicable_rs.is_empty());
}

// -- CallbackTendency Tests --

#[test]
fn test_callback_tendency() {
    let cb = CallbackTendency::new("custom", |ctx| {
        ctx.tool.as_deref() == Some("shell")
    })
    .description("Custom tendency for shell")
    .strength(TendencyStrength::Strong)
    .encourage("be_careful");

    let ctx_shell = TendencyContext::new().tool("shell");
    assert!(cb.applies_to(&ctx_shell));

    let ctx_http = TendencyContext::new().tool("http");
    assert!(!cb.applies_to(&ctx_http));

    let tendency = cb.into_tendency();
    assert_eq!(tendency.name, "custom");
    assert_eq!(tendency.strength, TendencyStrength::Strong);
    assert_eq!(tendency.encourages, vec!["be_careful"]);
}

// -- Serialization Tests --

#[test]
fn test_tendency_strength_serialization() {
    let strength = TendencyStrength::Strong;
    let json = serde_json::to_string(&strength).unwrap();
    let deserialized: TendencyStrength = serde_json::from_str(&json).unwrap();
    assert_eq!(strength, deserialized);
}

#[test]
fn test_tendency_serialization() {
    let t = Tendency::new("test", "Test tendency")
        .strength(TendencyStrength::VeryStrong)
        .category("testing")
        .encourage("test_more");

    let json = serde_json::to_string(&t).unwrap();
    let deserialized: Tendency = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "test");
    assert_eq!(deserialized.strength, TendencyStrength::VeryStrong);
    assert_eq!(deserialized.category.as_deref(), Some("testing"));
}

#[test]
fn test_tendency_condition_serialization() {
    let conditions = vec![
        TendencyCondition::Always,
        TendencyCondition::FileType(vec!["rs".to_string()]),
        TendencyCondition::Tool(vec!["shell".to_string()]),
        TendencyCondition::Custom {
            key: "env".to_string(),
            value: "prod".to_string(),
        },
    ];

    for cond in conditions {
        let json = serde_json::to_string(&cond).unwrap();
        let _: TendencyCondition = serde_json::from_str(&json).unwrap();
    }
}
