use roska_processor::*;
use roska_descriptor::*;
use roska_descriptor::opcode::Opcode;

fn sample_module() -> ModuleDescriptor {
    let mut m = ModuleDescriptor::new("api");

    let mut f1 = FileDescriptor::new("src/routes.rs", 200);
    f1.functions.push(
        FuncDescriptor::new("handle", "(Request) -> Response")
            .with_detail(FuncDetail::new().with_calls(vec!["validate".into(), "process".into()]))
    );

    let mut f2 = FileDescriptor::new("src/auth.rs", 150);
    f2.functions.push(FuncDescriptor::new("validate", "(Token) -> bool"));

    let mut f3 = FileDescriptor::new("src/process.rs", 300);
    f3.functions.push(
        FuncDescriptor::new("process", "(Data) -> Output")
            .with_detail(FuncDetail::new().with_calls(vec!["transform".into()]))
    );

    m.add_file(f1);
    m.add_file(f2);
    m.add_file(f3);
    m
}

#[test]
fn test_render_at_depth() {
    let mut file = FileDescriptor::new("src/lib.rs", 100);
    file.purpose = Some("Main library".into());
    file.tags = vec!["core".into()];
    file.functions.push(FuncDescriptor::new("init", "() -> Result<()>"));

    let overview = render_at_depth(&file, Depth::Overview);
    assert!(overview.contains("purpose: Main library"));
    assert!(!overview.contains("init")); // functions not shown at depth 0

    let structure = render_at_depth(&file, Depth::Structure);
    assert!(structure.contains("init"));
}

#[test]
fn test_extract_function() {
    let mut file = FileDescriptor::new("test.rs", 50);
    file.functions.push(
        FuncDescriptor::new("process", "(Vec<u8>) -> Result")
            .with_async(true)
            .with_ctx("Main pipeline")
            .with_detail(
                FuncDetail::new()
                    .with_calls(vec!["parse".into(), "validate".into()])
                    .with_body(vec![
                        Opcode::call("parse", vec!["data"], "r0"),
                        Opcode::ret("Ok(r0)"),
                    ]),
            ),
    );

    let overview = extract_function(&file, "process", Depth::Structure).unwrap();
    assert!(overview.contains("process"));
    assert!(!overview.contains("calls")); // not at depth 1

    let detail = extract_function(&file, "process", Depth::Detail).unwrap();
    assert!(detail.contains("calls: [parse, validate]"));

    let body = extract_function(&file, "process", Depth::Body).unwrap();
    assert!(body.contains("CALL parse"));
}

#[test]
fn test_callers_of() {
    let module = sample_module();
    let callers = callers_of(&module, "validate");
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].1.name, "handle");
}

#[test]
fn test_find_async_functions() {
    let mut m = ModuleDescriptor::new("test");
    let mut f = FileDescriptor::new("test.rs", 50);
    f.functions.push(FuncDescriptor::new("sync_fn", "()"));
    f.functions.push(FuncDescriptor::new("async_fn", "()").with_async(true));
    m.add_file(f);

    let async_fns = find_functions(&m, |f| f.is_async);
    assert_eq!(async_fns.len(), 1);
    assert_eq!(async_fns[0].1.name, "async_fn");
}

#[test]
fn test_extract_module() {
    let module = sample_module();

    // Extract only files with "auth" in path
    let auth = extract_module(&module, "auth-only", |f| {
        f.file.to_string_lossy().contains("auth")
    });

    assert_eq!(auth.name, "auth-only");
    assert_eq!(auth.file_count(), 1);
    assert!(matches!(auth.source, ModuleSource::Extracted { .. }));
}

#[test]
fn test_merge_modules() {
    let mut a = ModuleDescriptor::new("a");
    a.add_file(FileDescriptor::new("src/foo.rs", 10));
    a.add_file(FileDescriptor::new("src/bar.rs", 20));

    let mut b = ModuleDescriptor::new("b");
    b.add_file(FileDescriptor::new("src/bar.rs", 20)); // duplicate
    b.add_file(FileDescriptor::new("src/baz.rs", 30));

    let merged = merge_modules("merged", &a, &b);
    assert_eq!(merged.file_count(), 3); // foo + bar + baz (no dupes)
    assert!(matches!(merged.source, ModuleSource::Composed { .. }));
}

#[test]
fn test_intersect_modules() {
    let mut a = ModuleDescriptor::new("a");
    a.add_file(FileDescriptor::new("src/foo.rs", 10));
    a.add_file(FileDescriptor::new("src/shared.rs", 20));

    let mut b = ModuleDescriptor::new("b");
    b.add_file(FileDescriptor::new("src/shared.rs", 20));
    b.add_file(FileDescriptor::new("src/bar.rs", 30));

    let inter = intersect_modules("common", &a, &b);
    assert_eq!(inter.file_count(), 1);
    assert_eq!(inter.files[0].file.to_string_lossy(), "src/shared.rs");
}

#[test]
fn test_subtract_modules() {
    let mut a = ModuleDescriptor::new("a");
    a.add_file(FileDescriptor::new("src/foo.rs", 10));
    a.add_file(FileDescriptor::new("src/shared.rs", 20));

    let mut b = ModuleDescriptor::new("b");
    b.add_file(FileDescriptor::new("src/shared.rs", 20));

    let diff = subtract_modules("only-a", &a, &b);
    assert_eq!(diff.file_count(), 1);
    assert_eq!(diff.files[0].file.to_string_lossy(), "src/foo.rs");
}

#[test]
fn test_render_workspace() {
    let mut ws = WorkspaceDescriptor::new("my-project", "/tmp/project");
    ws.purpose = Some("Test workspace".into());

    let mut krate = CrateDescriptor::new("core", "crates/core");
    let mut module = ModuleDescriptor::new("src");
    module.add_file(FileDescriptor::new("src/lib.rs", 100));
    krate.add_module(module);
    ws.add_crate(krate);

    let overview = render_workspace_at_depth(&ws, Depth::Overview);
    assert!(overview.contains("my-project"));
    assert!(overview.contains("crates: 1"));

    let structure = render_workspace_at_depth(&ws, Depth::Structure);
    assert!(structure.contains("core"));
}
