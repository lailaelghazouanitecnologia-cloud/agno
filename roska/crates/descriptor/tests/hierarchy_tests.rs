use roska_descriptor::*;

#[test]
fn test_workspace_hierarchy() {
    let mut ws = WorkspaceDescriptor::new("my-project", "/home/user/project");
    ws.purpose = Some("Web API framework".into());

    let mut krate = CrateDescriptor::new("api", "crates/api");
    krate.deps = vec!["tokio".into(), "serde".into()];

    let mut module = ModuleDescriptor::new("routes");
    module.purpose = Some("HTTP route handlers".into());
    module.add_file(FileDescriptor::new("src/routes/users.rs", 150));
    module.add_file(FileDescriptor::new("src/routes/items.rs", 200));

    krate.add_module(module);
    ws.add_crate(krate);

    assert_eq!(ws.file_count(), 2);
    assert_eq!(ws.crates[0].name, "api");
    assert_eq!(ws.crates[0].modules[0].name, "routes");
    assert_eq!(ws.crates[0].modules[0].files.len(), 2);
}

#[test]
fn test_module_from_files() {
    let files = vec![
        FileDescriptor::new("src/auth.rs", 100),
        FileDescriptor::new("src/session.rs", 80),
        FileDescriptor::new("src/token.rs", 60),
    ];
    let module = ModuleDescriptor::from_files("auth", files);

    assert_eq!(module.name, "auth");
    assert_eq!(module.file_count(), 3);
    assert!(matches!(module.source, ModuleSource::Manual { .. }));
}

#[test]
fn test_nested_modules() {
    let mut parent = ModuleDescriptor::new("api");
    parent.add_file(FileDescriptor::new("src/api/mod.rs", 20));

    let mut child = ModuleDescriptor::new("v1");
    child.add_file(FileDescriptor::new("src/api/v1/routes.rs", 300));
    child.add_file(FileDescriptor::new("src/api/v1/handlers.rs", 500));

    parent.add_submodule(child);

    assert_eq!(parent.file_count(), 3); // 1 + 2
}

#[test]
fn test_yaml_roundtrip() {
    let mut module = ModuleDescriptor::new("test");
    module.purpose = Some("Test module".into());
    module.tags = vec!["testing".into()];
    module.add_file(FileDescriptor::new("test.rs", 50));

    let yaml = serde_yaml::to_string(&module).unwrap();
    let parsed: ModuleDescriptor = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(parsed.name, "test");
    assert_eq!(parsed.purpose, Some("Test module".into()));
    assert_eq!(parsed.files.len(), 1);
}
