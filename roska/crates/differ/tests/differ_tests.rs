use roska_differ::*;
use roska_descriptor::file::*;
use roska_descriptor::function::*;
use roska_descriptor::types::*;
use roska_descriptor::change::*;

fn make_file(funcs: Vec<FuncDescriptor>, types: Vec<TypeDescriptor>) -> FileDescriptor {
    let mut file = FileDescriptor::new("src/test.rs", 100);
    file.functions = funcs;
    file.types = types;
    file
}

#[test]
fn test_detect_function_added_removed() {
    let old = make_file(
        vec![FuncDescriptor::new("foo", "() -> i32")],
        vec![],
    );
    let new = make_file(
        vec![FuncDescriptor::new("bar", "() -> i32")],
        vec![],
    );

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::FunctionRemoved { name } if name == "foo")));
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::FunctionAdded { name } if name == "bar")));
}

#[test]
fn test_detect_signature_change() {
    let old = make_file(
        vec![FuncDescriptor::new("process", "(Vec<u8>) -> Result<Output>")],
        vec![],
    );
    let new = make_file(
        vec![FuncDescriptor::new("process", "(Vec<u8>, &Config) -> Result<Output, Error>")],
        vec![],
    );

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::ReturnChanged { .. })));
}

#[test]
fn test_detect_async_change() {
    let old = make_file(
        vec![FuncDescriptor::new("handler", "() -> Response")],
        vec![],
    );
    let new = make_file(
        vec![FuncDescriptor::new("handler", "() -> Response").with_async(true)],
        vec![],
    );

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::AsyncChanged { now_async: true })));
}

#[test]
fn test_detect_field_changes() {
    let old = make_file(vec![], vec![
        TypeDescriptor::new_struct("User", vec![
            FieldDef::new("name", "String"),
            FieldDef::new("age", "u32"),
        ]),
    ]);
    let new = make_file(vec![], vec![
        TypeDescriptor::new_struct("User", vec![
            FieldDef::new("name", "String"),
            FieldDef::new("age", "u64"),       // retyped
            FieldDef::new("email", "String"),   // added
        ]),
    ]);

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::FieldRetyped { field, .. } if field == "age")));
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::FieldAdded { field, .. } if field == "email")));
}

#[test]
fn test_detect_variant_changes() {
    let old = make_file(vec![], vec![
        TypeDescriptor::new_enum("Error", vec![
            VariantDef::new("NotFound"),
            VariantDef::new("Unauthorized"),
        ]),
    ]);
    let new = make_file(vec![], vec![
        TypeDescriptor::new_enum("Error", vec![
            VariantDef::new("NotFound"),
            VariantDef::new("Internal"),        // replaced Unauthorized
        ]),
    ]);

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::VariantRemoved { variant, .. } if variant == "Unauthorized")));
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::VariantAdded { variant, .. } if variant == "Internal")));
}

#[test]
fn test_detect_import_changes() {
    let mut old = FileDescriptor::new("test.rs", 10);
    old.imports = vec![ImportEntry::new("std::io"), ImportEntry::new("serde::Serialize")];

    let mut new = FileDescriptor::new("test.rs", 10);
    new.imports = vec![ImportEntry::new("std::io"), ImportEntry::new("tokio::spawn")];

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::ImportRemoved { sym } if sym == "serde::Serialize")));
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::ImportAdded { sym } if sym == "tokio::spawn")));
}

#[test]
fn test_detect_call_changes() {
    let old = make_file(
        vec![FuncDescriptor::new("process", "()").with_detail(
            FuncDetail::new().with_calls(vec!["parse".into(), "validate".into()])
        )],
        vec![],
    );
    let new = make_file(
        vec![FuncDescriptor::new("process", "()").with_detail(
            FuncDetail::new().with_calls(vec!["parse".into(), "transform".into()])
        )],
        vec![],
    );

    let cs = diff_files(&old, &new);
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::CallRemoved { func } if func == "validate")));
    assert!(cs.changes.iter().any(|c| matches!(&c.kind, ChangeKind::CallAdded { func } if func == "transform")));
}

#[test]
fn test_no_changes() {
    let file = make_file(
        vec![FuncDescriptor::new("foo", "()")],
        vec![TypeDescriptor::new_struct("Bar", vec![])],
    );

    let cs = diff_files(&file, &file);
    assert!(cs.is_empty());
}
