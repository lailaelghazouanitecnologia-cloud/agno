use roska_descriptor::*;

#[test]
fn test_opcode_constructors() {
    let ops = vec![
        Opcode::call("parse_header", vec!["data"], "r0"),
        Opcode::try_op("r0"),
        Opcode::call("validate", vec!["r0"], "r1"),
        Opcode::br_false("r1", ":err"),
        Opcode::call("transform", vec!["r0", "data"], "r2"),
        Opcode::ret("Ok(r2)"),
        Opcode::label(":err"),
        Opcode::ret("Err(ValidationError)"),
    ];

    assert_eq!(ops.len(), 8);
    assert!(ops[0].is_call());
    assert!(ops[3].is_branch());
    assert_eq!(ops[0].called_function(), Some("parse_header"));
    assert_eq!(ops[0].mnemonic(), "CALL");
}

#[test]
fn test_opcode_display() {
    let op = Opcode::call("validate", vec!["header"], "r1");
    let s = format!("{}", op);
    assert_eq!(s, "CALL validate (header) -> r1");
}

#[test]
fn test_opcode_yaml_roundtrip() {
    let ops = vec![
        Opcode::call("parse", vec!["data"], "r0"),
        Opcode::try_op("r0"),
        Opcode::br_false("r0", ":err"),
        Opcode::label(":err"),
        Opcode::ret("Err"),
    ];

    let yaml = serde_yaml::to_string(&ops).unwrap();
    let parsed: Vec<Opcode> = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(ops, parsed);
}

#[test]
fn test_mut_call() {
    let op = Opcode::mut_call("CACHE", "insert", vec!["key", "val"], None);
    assert!(op.is_call());
    assert_eq!(op.called_function(), Some("insert"));
    let s = format!("{}", op);
    assert!(s.contains("MUT_CALL CACHE.insert"));
}
