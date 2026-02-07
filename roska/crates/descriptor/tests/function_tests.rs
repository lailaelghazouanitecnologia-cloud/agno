use roska_descriptor::*;
use roska_descriptor::function::Visibility;

#[test]
fn test_func_descriptor() {
    let func = FuncDescriptor::new("process_data", "(Vec<u8>) -> Result<Output>")
        .with_async(true)
        .with_vis(Visibility::Pub)
        .with_lines(45, 89)
        .with_ctx("Orquesta parse -> validate -> transform")
        .with_detail(
            FuncDetail::new()
                .with_calls(vec!["parse_header".into(), "validate".into(), "transform".into()])
                .with_body(vec![
                    Opcode::call("parse_header", vec!["data"], "r0"),
                    Opcode::try_op("r0"),
                    Opcode::call("validate", vec!["r0"], "r1"),
                    Opcode::ret("Ok(r2)"),
                ]),
        );

    assert_eq!(func.name, "process_data");
    assert!(func.is_async);
    assert_eq!(func.calls(), &["parse_header", "validate", "transform"]);
    assert_eq!(func.opcode_count(), 4);
}

#[test]
fn test_func_yaml_roundtrip() {
    let func = FuncDescriptor::new("handler", "(Request) -> Response")
        .with_async(true)
        .with_vis(Visibility::Pub);

    let yaml = serde_yaml::to_string(&func).unwrap();
    let parsed: FuncDescriptor = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(parsed.name, "handler");
    assert!(parsed.is_async);
    assert_eq!(parsed.vis, Visibility::Pub);
}
