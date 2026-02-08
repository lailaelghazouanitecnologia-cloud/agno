use kkr_vm::{Register, Opcode};

#[test]
fn test_register_display() {
    assert_eq!(format!("{}", Register::ACC), "@0");
    assert_eq!(format!("{}", Register(3)), "@3");
}

#[test]
fn test_opcode_mnemonic() {
    let op = Opcode::CallLlm {
        prompt: "test".into(),
        model_tag: "architect".into(),
        dest: Register::ACC,
    };
    assert_eq!(op.mnemonic(), "CALL_LLM");
}

#[test]
fn test_opcode_dest_register() {
    let op = Opcode::CallTool {
        tool_name: "errordb".into(),
        params: Register(1),
        dest: Register(2),
    };
    assert_eq!(op.dest_register(), Some(Register(2)));
    assert_eq!(Opcode::Barrier.dest_register(), None);
}

#[test]
fn test_disassemble() {
    let op = Opcode::CallAgent {
        agent_id: "worker-1".into(),
        context: Register(1),
        dest: Register(2),
    };
    assert_eq!(op.disassemble(), "CALL_AGENT worker-1 (@1) -> @2");
}
