use roska_descriptor::*;
use roska_descriptor::function::Visibility;

#[test]
fn test_struct_descriptor() {
    let t = TypeDescriptor::new_struct("Output", vec![
        FieldDef::new("data", "Vec<u8>"),
        FieldDef::new("metadata", "HashMap<String,String>"),
    ])
    .with_vis(Visibility::Pub)
    .with_derives(vec!["Debug".into(), "Clone".into()])
    .with_ctx("Resultado del pipeline");

    assert_eq!(t.name, "Output");
    assert_eq!(t.kind, TypeKind::Struct);
    assert_eq!(t.field_count(), 2);
    assert_eq!(t.derives.len(), 2);
}

#[test]
fn test_enum_descriptor() {
    let t = TypeDescriptor::new_enum("ProcessError", vec![
        VariantDef::new("ParseError").with_data("String").with_ctx("Header malformado"),
        VariantDef::new("ValidationError").with_data("String"),
    ]);

    assert_eq!(t.kind, TypeKind::Enum);
    assert_eq!(t.field_count(), 2);
    assert_eq!(t.variants[0].ctx.as_deref(), Some("Header malformado"));
}

#[test]
fn test_trait_descriptor() {
    let t = TypeDescriptor::new_trait("Handler", vec![
        "handle".into(), "validate".into(),
    ]);

    assert_eq!(t.kind, TypeKind::Trait);
    assert_eq!(t.methods.len(), 2);
}
