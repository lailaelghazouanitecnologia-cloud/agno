use roska_descriptor::*;
use roska_descriptor::change::Impact;

#[test]
fn test_changeset() {
    let mut cs = ChangeSet::new("src/processor.rs");

    cs.add(
        ChangeKind::Rename { from: "process_data".into(), to: "process_binary".into() },
        "func.process_data",
        Impact::High,
    );
    cs.add(
        ChangeKind::ParamAdded {
            param: Param::new("config", "&Config"),
            pos: 1,
        },
        "func.process_binary",
        Impact::High,
    );
    cs.add(
        ChangeKind::CallAdded { func: "validate_config".into() },
        "func.process_binary.body",
        Impact::Low,
    );

    assert_eq!(cs.changes.len(), 3);
    assert_eq!(cs.breaking_count(), 2);
    assert!(cs.changes[0].is_rename());
    assert!(cs.changes[1].is_signature_change());
    assert!(cs.changes[2].is_body_change());
}

#[test]
fn test_propagation() {
    let mut cs = ChangeSet::new("src/processor.rs");
    cs.add(
        ChangeKind::Rename { from: "process_data".into(), to: "process_binary".into() },
        "func.process_data",
        Impact::High,
    );
    cs.add_propagation(Propagation {
        file: "src/api/routes.rs".into(),
        target: "func.handle_process".into(),
        required: ChangeKind::Rename {
            from: "process_data".into(),
            to: "process_binary".into(),
        },
        reason: "Calls renamed function".into(),
    });

    assert_eq!(cs.affected_files().len(), 1);
}

#[test]
fn test_default_impact() {
    assert_eq!(
        ChangeKind::Rename { from: "a".into(), to: "b".into() }.default_impact(),
        Impact::High
    );
    assert_eq!(
        ChangeKind::CallAdded { func: "f".into() }.default_impact(),
        Impact::Low
    );
    assert_eq!(
        ChangeKind::AsyncChanged { now_async: true }.default_impact(),
        Impact::Medium
    );
}

#[test]
fn test_changeset_yaml() {
    let mut cs = ChangeSet::new("test.rs");
    cs.add(
        ChangeKind::FunctionAdded { name: "new_func".into() },
        "file",
        Impact::Low,
    );

    let yaml = serde_yaml::to_string(&cs).unwrap();
    let parsed: ChangeSet = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.changes.len(), 1);
}
