//! Roska Differ — detects structured changes between descriptor versions.
//!
//! Compares two FileDescriptors and produces a ChangeSet with typed changes.
//! Each change has a kind (rename, param_added, etc.) and an impact level.

use roska_descriptor::change::*;
use roska_descriptor::file::*;
use roska_descriptor::function::*;
use roska_descriptor::types::*;
use std::collections::HashSet;

/// Compare two FileDescriptors and produce a ChangeSet.
pub fn diff_files(old: &FileDescriptor, new: &FileDescriptor) -> ChangeSet {
    let mut cs = ChangeSet::new(&new.file);

    // ── Imports ──
    diff_imports(&old.imports, &new.imports, &mut cs);

    // ── Data section ──
    diff_data(&old.data, &new.data, &mut cs);

    // ── Types ──
    diff_types(&old.types, &new.types, &mut cs);

    // ── Functions ──
    diff_functions(&old.functions, &new.functions, &mut cs);

    // ── Exports ──
    diff_exports(&old.exports, &new.exports, &mut cs);

    cs
}

/// Compare two ModuleDescriptors.
pub fn diff_modules(
    old: &roska_descriptor::ModuleDescriptor,
    new: &roska_descriptor::ModuleDescriptor,
) -> Vec<ChangeSet> {
    let mut changesets = Vec::new();

    let old_files: HashSet<String> = old.files.iter()
        .map(|f| f.file.to_string_lossy().to_string())
        .collect();
    let new_files: HashSet<String> = new.files.iter()
        .map(|f| f.file.to_string_lossy().to_string())
        .collect();

    // Files in both → diff them
    for new_file in &new.files {
        let path = new_file.file.to_string_lossy().to_string();
        if let Some(old_file) = old.files.iter().find(|f| f.file.to_string_lossy() == path) {
            let cs = diff_files(old_file, new_file);
            if !cs.is_empty() {
                changesets.push(cs);
            }
        } else {
            // New file added
            let mut cs = ChangeSet::new(&new_file.file);
            for func in &new_file.functions {
                cs.add(
                    ChangeKind::FunctionAdded { name: func.name.clone() },
                    "file",
                    Impact::Low,
                );
            }
            for t in &new_file.types {
                cs.add(
                    ChangeKind::TypeAdded { name: t.name.clone() },
                    "file",
                    Impact::Low,
                );
            }
            if !cs.is_empty() {
                changesets.push(cs);
            }
        }
    }

    // Files removed
    for path in &old_files {
        if !new_files.contains(path) {
            let old_file = old.files.iter().find(|f| f.file.to_string_lossy() == *path).unwrap();
            let mut cs = ChangeSet::new(path);
            for func in &old_file.functions {
                cs.add(
                    ChangeKind::FunctionRemoved { name: func.name.clone() },
                    "file",
                    Impact::High,
                );
            }
            changesets.push(cs);
        }
    }

    changesets
}

// ── Internal diff functions ──

fn diff_imports(old: &[ImportEntry], new: &[ImportEntry], cs: &mut ChangeSet) {
    let old_syms: HashSet<&str> = old.iter().map(|i| i.sym.as_str()).collect();
    let new_syms: HashSet<&str> = new.iter().map(|i| i.sym.as_str()).collect();

    for sym in new_syms.difference(&old_syms) {
        cs.add(ChangeKind::ImportAdded { sym: sym.to_string() }, "imports", Impact::Low);
    }
    for sym in old_syms.difference(&new_syms) {
        cs.add(ChangeKind::ImportRemoved { sym: sym.to_string() }, "imports", Impact::Low);
    }
}

fn diff_data(old: &[DataEntry], new: &[DataEntry], cs: &mut ChangeSet) {
    let old_names: HashSet<&str> = old.iter().map(|d| d.name.as_str()).collect();
    let new_names: HashSet<&str> = new.iter().map(|d| d.name.as_str()).collect();

    for name in new_names.difference(&old_names) {
        cs.add(ChangeKind::DataAdded { name: name.to_string() }, "data", Impact::Low);
    }
    for name in old_names.difference(&new_names) {
        cs.add(ChangeKind::DataRemoved { name: name.to_string() }, "data", Impact::Low);
    }

    // Changed values
    for old_d in old {
        if let Some(new_d) = new.iter().find(|d| d.name == old_d.name) {
            if old_d.val != new_d.val {
                cs.add(
                    ChangeKind::ConstantChanged {
                        name: old_d.name.clone(),
                        from: old_d.val.clone().unwrap_or_default(),
                        to: new_d.val.clone().unwrap_or_default(),
                    },
                    format!("data.{}", old_d.name),
                    Impact::Medium,
                );
            }
        }
    }
}

fn diff_types(old: &[TypeDescriptor], new: &[TypeDescriptor], cs: &mut ChangeSet) {
    let old_names: HashSet<&str> = old.iter().map(|t| t.name.as_str()).collect();
    let new_names: HashSet<&str> = new.iter().map(|t| t.name.as_str()).collect();

    for name in new_names.difference(&old_names) {
        cs.add(ChangeKind::TypeAdded { name: name.to_string() }, "types", Impact::Low);
    }
    for name in old_names.difference(&new_names) {
        cs.add(ChangeKind::TypeRemoved { name: name.to_string() }, "types", Impact::High);
    }

    // Diff matching types
    for old_t in old {
        if let Some(new_t) = new.iter().find(|t| t.name == old_t.name) {
            diff_type_detail(old_t, new_t, cs);
        }
    }
}

fn diff_type_detail(old: &TypeDescriptor, new: &TypeDescriptor, cs: &mut ChangeSet) {
    let target = format!("type.{}", old.name);

    // Fields (structs)
    let old_fields: HashSet<&str> = old.fields.iter().map(|f| f.name.as_str()).collect();
    let new_fields: HashSet<&str> = new.fields.iter().map(|f| f.name.as_str()).collect();

    for name in new_fields.difference(&old_fields) {
        let field = new.fields.iter().find(|f| f.name == *name).unwrap();
        cs.add(
            ChangeKind::FieldAdded {
                type_name: old.name.clone(),
                field: name.to_string(),
                field_type: field.field_type.clone(),
            },
            &target,
            Impact::Low,
        );
    }
    for name in old_fields.difference(&new_fields) {
        cs.add(
            ChangeKind::FieldRemoved {
                type_name: old.name.clone(),
                field: name.to_string(),
            },
            &target,
            Impact::High,
        );
    }

    // Field type changes
    for old_f in &old.fields {
        if let Some(new_f) = new.fields.iter().find(|f| f.name == old_f.name) {
            if old_f.field_type != new_f.field_type {
                cs.add(
                    ChangeKind::FieldRetyped {
                        type_name: old.name.clone(),
                        field: old_f.name.clone(),
                        from: old_f.field_type.clone(),
                        to: new_f.field_type.clone(),
                    },
                    &target,
                    Impact::High,
                );
            }
        }
    }

    // Variants (enums)
    let old_vars: HashSet<&str> = old.variants.iter().map(|v| v.name.as_str()).collect();
    let new_vars: HashSet<&str> = new.variants.iter().map(|v| v.name.as_str()).collect();

    for name in new_vars.difference(&old_vars) {
        cs.add(
            ChangeKind::VariantAdded { enum_name: old.name.clone(), variant: name.to_string() },
            &target,
            Impact::Low,
        );
    }
    for name in old_vars.difference(&new_vars) {
        cs.add(
            ChangeKind::VariantRemoved { enum_name: old.name.clone(), variant: name.to_string() },
            &target,
            Impact::High,
        );
    }

    // Derives
    let old_derives: HashSet<&str> = old.derives.iter().map(|d| d.as_str()).collect();
    let new_derives: HashSet<&str> = new.derives.iter().map(|d| d.as_str()).collect();

    for d in new_derives.difference(&old_derives) {
        cs.add(
            ChangeKind::DeriveAdded { type_name: old.name.clone(), derive: d.to_string() },
            &target,
            Impact::Low,
        );
    }
    for d in old_derives.difference(&new_derives) {
        cs.add(
            ChangeKind::DeriveRemoved { type_name: old.name.clone(), derive: d.to_string() },
            &target,
            Impact::Low,
        );
    }
}

fn diff_functions(old: &[FuncDescriptor], new: &[FuncDescriptor], cs: &mut ChangeSet) {
    let old_names: HashSet<&str> = old.iter().map(|f| f.name.as_str()).collect();
    let new_names: HashSet<&str> = new.iter().map(|f| f.name.as_str()).collect();

    for name in new_names.difference(&old_names) {
        cs.add(ChangeKind::FunctionAdded { name: name.to_string() }, "functions", Impact::Low);
    }
    for name in old_names.difference(&new_names) {
        cs.add(ChangeKind::FunctionRemoved { name: name.to_string() }, "functions", Impact::High);
    }

    // Diff matching functions
    for old_f in old {
        if let Some(new_f) = new.iter().find(|f| f.name == old_f.name) {
            diff_function_detail(old_f, new_f, cs);
        }
    }
}

fn diff_function_detail(old: &FuncDescriptor, new: &FuncDescriptor, cs: &mut ChangeSet) {
    let target = format!("func.{}", old.name);

    // Signature change
    if old.sig != new.sig {
        cs.add(
            ChangeKind::ReturnChanged { from: old.sig.clone(), to: new.sig.clone() },
            &target,
            Impact::High,
        );
    }

    // Async change
    if old.is_async != new.is_async {
        cs.add(
            ChangeKind::AsyncChanged { now_async: new.is_async },
            &target,
            Impact::Medium,
        );
    }

    // Visibility change
    if old.vis != new.vis {
        cs.add(
            ChangeKind::VisibilityChanged {
                from: format!("{:?}", old.vis),
                to: format!("{:?}", new.vis),
            },
            &target,
            Impact::Medium,
        );
    }

    // Calls change
    let old_calls: HashSet<&str> = old.calls().iter().map(|c| c.as_str()).collect();
    let new_calls: HashSet<&str> = new.calls().iter().map(|c| c.as_str()).collect();

    for c in new_calls.difference(&old_calls) {
        cs.add(
            ChangeKind::CallAdded { func: c.to_string() },
            format!("{}.body", target),
            Impact::Low,
        );
    }
    for c in old_calls.difference(&new_calls) {
        cs.add(
            ChangeKind::CallRemoved { func: c.to_string() },
            format!("{}.body", target),
            Impact::Low,
        );
    }
}

fn diff_exports(old: &[ExportEntry], new: &[ExportEntry], cs: &mut ChangeSet) {
    let old_names: HashSet<&str> = old.iter().map(|e| e.name.as_str()).collect();
    let new_names: HashSet<&str> = new.iter().map(|e| e.name.as_str()).collect();

    for name in new_names.difference(&old_names) {
        cs.add(ChangeKind::ExportAdded { name: name.to_string() }, "exports", Impact::Low);
    }
    for name in old_names.difference(&new_names) {
        cs.add(ChangeKind::ExportRemoved { name: name.to_string() }, "exports", Impact::High);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
