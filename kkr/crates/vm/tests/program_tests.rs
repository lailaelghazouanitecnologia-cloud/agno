use kkr_vm::{ProgramBuilder, Program, Register, Opcode};

#[test]
fn test_program_builder() {
    let prog = ProgramBuilder::new("test-plan")
        .store(serde_json::json!({"task": "analyze"}), Register(1))
        .call_tool("ast_parse", Register(1), Register(2))
        .call_llm("Generate implementation", "developer", Register(3))
        .call_agent("worker-1", Register(3), Register(4))
        .barrier()
        .call_tool("refactor_rename", Register(4), Register(5))
        .ret(Register(5))
        .build();

    assert_eq!(prog.name, "test-plan");
    assert_eq!(prog.len(), 7);
    assert_eq!(prog.register_count, 6); // @0..@5
}

#[test]
fn test_program_disassemble() {
    let prog = ProgramBuilder::new("simple")
        .label("start")
        .store(serde_json::json!("hello"), Register::ACC)
        .call_llm("prompt", "architect", Register(1))
        .ret(Register(1))
        .build();

    let dis = prog.disassemble();
    assert!(dis.contains("Program: simple"));
    assert!(dis.contains("CALL_LLM"));
    assert!(dis.contains("RET"));
    assert!(dis.contains(".start:"));
}

#[test]
fn test_program_labels() {
    let mut prog = Program::new("labeled");
    prog.label("start");
    prog.emit(Opcode::Nop);
    prog.emit(Opcode::Nop);
    prog.label("end");
    prog.emit(Opcode::Return { src: Register::ACC });

    assert_eq!(prog.find_label("start"), Some(0));
    assert_eq!(prog.find_label("end"), Some(2));
    assert_eq!(prog.find_label("missing"), None);
}
