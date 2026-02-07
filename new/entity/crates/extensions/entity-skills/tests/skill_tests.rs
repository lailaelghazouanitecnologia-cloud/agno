use entity_skills::*;

#[test]
fn test_skill_new() {
    let skill = Skill::new("rust-coding", "Write Rust code", "Follow Rust best practices...");
    assert_eq!(skill.name, "rust-coding");
    assert_eq!(skill.description, "Write Rust code");
    assert_eq!(skill.instructions, "Follow Rust best practices...");
    assert!(skill.source_path.is_none());
    assert!(skill.scripts.is_empty());
    assert!(skill.references.is_empty());
    assert!(skill.metadata.is_empty());
    assert!(skill.allowed_tools.is_empty());
    assert!(skill.tags.is_empty());
    assert!(skill.version.is_none());
}

#[test]
fn test_skill_builder() {
    let skill = Skill::new("test-skill", "desc", "instructions")
        .with_script("scripts/run.sh")
        .with_script("scripts/test.sh")
        .with_reference("refs/api.md")
        .with_tag("coding")
        .with_tag("testing")
        .with_tool("read_file")
        .with_tool("write_file")
        .with_version("1.0.0")
        .with_source_path("/skills/test-skill")
        .with_metadata("author", serde_json::json!("dev"));

    assert_eq!(skill.scripts.len(), 2);
    assert_eq!(skill.references.len(), 1);
    assert_eq!(skill.tags, vec!["coding", "testing"]);
    assert_eq!(skill.allowed_tools, vec!["read_file", "write_file"]);
    assert_eq!(skill.version, Some("1.0.0".to_string()));
    assert_eq!(skill.source_path, Some("/skills/test-skill".to_string()));
    assert_eq!(skill.metadata["author"], serde_json::json!("dev"));
}

#[test]
fn test_skill_to_prompt_basic() {
    let skill = Skill::new("analysis", "Code analysis", "Analyze code for issues.");
    let prompt = skill.to_prompt();

    assert!(prompt.contains("## Skill: analysis"));
    assert!(prompt.contains("Code analysis"));
    assert!(prompt.contains("### Instructions"));
    assert!(prompt.contains("Analyze code for issues."));
    // Should NOT contain tool/reference/script sections when empty
    assert!(!prompt.contains("### Allowed Tools"));
    assert!(!prompt.contains("### References"));
    assert!(!prompt.contains("### Scripts"));
}

#[test]
fn test_skill_to_prompt_with_extras() {
    let skill = Skill::new("full", "Full skill", "Do everything.")
        .with_tool("bash")
        .with_tool("read_file")
        .with_reference("docs/guide.md")
        .with_script("run.sh");

    let prompt = skill.to_prompt();

    assert!(prompt.contains("### Allowed Tools"));
    assert!(prompt.contains("- bash"));
    assert!(prompt.contains("- read_file"));
    assert!(prompt.contains("### References"));
    assert!(prompt.contains("- docs/guide.md"));
    assert!(prompt.contains("### Scripts"));
    assert!(prompt.contains("- run.sh"));
}

#[test]
fn test_skill_registry_new() {
    let reg = SkillRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

#[test]
fn test_skill_registry_register_and_get() {
    let mut reg = SkillRegistry::new();
    reg.register(Skill::new("alpha", "A skill", "Do A"));
    reg.register(Skill::new("beta", "B skill", "Do B"));

    assert_eq!(reg.len(), 2);
    assert!(!reg.is_empty());

    let alpha = reg.get("alpha").unwrap();
    assert_eq!(alpha.description, "A skill");

    assert!(reg.get("gamma").is_none());
}

#[test]
fn test_skill_registry_list() {
    let mut reg = SkillRegistry::new();
    reg.register(Skill::new("x", "X", "X"));
    reg.register(Skill::new("y", "Y", "Y"));

    let names = reg.list();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
}

#[test]
fn test_skill_registry_by_tag() {
    let mut reg = SkillRegistry::new();
    reg.register(Skill::new("a", "A", "A").with_tag("coding").with_tag("rust"));
    reg.register(Skill::new("b", "B", "B").with_tag("coding"));
    reg.register(Skill::new("c", "C", "C").with_tag("testing"));

    let coding_skills = reg.by_tag("coding");
    assert_eq!(coding_skills.len(), 2);

    let rust_skills = reg.by_tag("rust");
    assert_eq!(rust_skills.len(), 1);
    assert_eq!(rust_skills[0].name, "a");

    let empty = reg.by_tag("nonexistent");
    assert!(empty.is_empty());
}

#[test]
fn test_skill_registry_remove() {
    let mut reg = SkillRegistry::new();
    reg.register(Skill::new("removable", "R", "R"));
    assert_eq!(reg.len(), 1);

    let removed = reg.remove("removable");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "removable");
    assert!(reg.is_empty());

    // Removing non-existent returns None
    assert!(reg.remove("nope").is_none());
}

#[test]
fn test_skill_registry_overwrite() {
    let mut reg = SkillRegistry::new();
    reg.register(Skill::new("s", "version 1", "v1"));
    reg.register(Skill::new("s", "version 2", "v2"));

    assert_eq!(reg.len(), 1);
    assert_eq!(reg.get("s").unwrap().description, "version 2");
}

#[test]
fn test_skill_serialization() {
    let skill = Skill::new("serde-test", "Serializable", "Instructions here")
        .with_tag("test")
        .with_tool("bash");

    let json = serde_json::to_string(&skill).unwrap();
    let parsed: Skill = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "serde-test");
    assert_eq!(parsed.tags, vec!["test"]);
    assert_eq!(parsed.allowed_tools, vec!["bash"]);
}

#[tokio::test]
async fn test_skill_loader_missing_dir() {
    let result = SkillLoader::load_from_dir("/nonexistent/path").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_skill_loader_load_from_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let skill_dir = tmp.path().join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "# Rust Expert\nWrite idiomatic Rust code.\nUse proper error handling.",
    )
    .unwrap();

    let scripts_dir = skill_dir.join("scripts");
    std::fs::create_dir_all(&scripts_dir).unwrap();
    std::fs::write(scripts_dir.join("build.sh"), "#!/bin/bash\ncargo build").unwrap();

    let refs_dir = skill_dir.join("references");
    std::fs::create_dir_all(&refs_dir).unwrap();
    std::fs::write(refs_dir.join("style.md"), "# Style Guide").unwrap();

    let skill = SkillLoader::load_from_dir(skill_dir.to_str().unwrap()).await.unwrap();

    assert_eq!(skill.name, "Rust Expert");
    assert!(skill.instructions.contains("Write idiomatic Rust code."));
    assert_eq!(skill.scripts.len(), 1);
    assert_eq!(skill.references.len(), 1);
    assert!(skill.source_path.is_some());
}

#[tokio::test]
async fn test_skill_loader_load_all() {
    let tmp = tempfile::tempdir().unwrap();

    // Create two skill directories
    for name in &["skill-a", "skill-b"] {
        let dir = tmp.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("# {}\nInstructions for {}.", name, name),
        )
        .unwrap();
    }

    // Create a non-skill directory (no SKILL.md)
    std::fs::create_dir_all(tmp.path().join("not-a-skill")).unwrap();

    let skills = SkillLoader::load_all(tmp.path().to_str().unwrap()).await.unwrap();
    assert_eq!(skills.len(), 2);
}
