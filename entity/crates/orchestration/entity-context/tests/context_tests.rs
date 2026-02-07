use entity_context::*;
use roska_descriptor::Depth;
use std::path::PathBuf;

#[test]
fn test_context_window_new() {
    let ctx = ContextWindow::new(128_000);
    assert_eq!(ctx.max_tokens, 128_000);
    assert_eq!(ctx.used_tokens, 0);
    assert_eq!(ctx.files.len(), 0);
    assert_eq!(ctx.available_tokens(), 128_000);
}

#[test]
fn test_context_window_with_system_tokens() {
    let mut ctx = ContextWindow::new(10_000);
    ctx.system_tokens = 2_000;
    ctx.history_tokens = 3_000;
    assert_eq!(ctx.available_tokens(), 5_000);
}

#[test]
fn test_add_file_within_budget() {
    let mut ctx = ContextWindow::new(1_000);
    let file = ContextFile {
        path: PathBuf::from("src/main.rs"),
        depth: Depth::Structure,
        tokens: 200,
        relevance: 0.8,
        reason: "test".to_string(),
    };
    assert!(ctx.add_file(file));
    assert_eq!(ctx.used_tokens, 200);
    assert_eq!(ctx.files.len(), 1);
    assert_eq!(ctx.available_tokens(), 800);
}

#[test]
fn test_add_file_over_budget() {
    let mut ctx = ContextWindow::new(100);
    let file = ContextFile {
        path: PathBuf::from("src/big.rs"),
        depth: Depth::Body,
        tokens: 200,
        relevance: 0.9,
        reason: "too big".to_string(),
    };
    assert!(!ctx.add_file(file));
    assert_eq!(ctx.used_tokens, 0);
    assert_eq!(ctx.files.len(), 0);
}

#[test]
fn test_remove_file() {
    let mut ctx = ContextWindow::new(1_000);
    let path = PathBuf::from("src/lib.rs");
    ctx.add_file(ContextFile {
        path: path.clone(),
        depth: Depth::Overview,
        tokens: 100,
        relevance: 0.5,
        reason: "test".to_string(),
    });
    assert_eq!(ctx.used_tokens, 100);
    assert!(ctx.remove_file(&path));
    assert_eq!(ctx.used_tokens, 0);
    assert_eq!(ctx.files.len(), 0);
}

#[test]
fn test_remove_nonexistent_file() {
    let mut ctx = ContextWindow::new(1_000);
    assert!(!ctx.remove_file(&PathBuf::from("nonexistent.rs")));
}

#[test]
fn test_upgrade_depth() {
    let mut ctx = ContextWindow::new(1_000);
    let path = PathBuf::from("src/mod.rs");
    ctx.add_file(ContextFile {
        path: path.clone(),
        depth: Depth::Overview,
        tokens: 50,
        relevance: 0.7,
        reason: "test".to_string(),
    });
    // Upgrade from 50 to 200 tokens
    assert!(ctx.upgrade_depth(&path, Depth::Detail, 200));
    assert_eq!(ctx.used_tokens, 200);
    assert_eq!(ctx.files[0].depth, Depth::Detail);
    assert_eq!(ctx.files[0].tokens, 200);
}

#[test]
fn test_upgrade_depth_over_budget() {
    let mut ctx = ContextWindow::new(300);
    let path = PathBuf::from("src/mod.rs");
    ctx.add_file(ContextFile {
        path: path.clone(),
        depth: Depth::Overview,
        tokens: 100,
        relevance: 0.7,
        reason: "test".to_string(),
    });
    // Try to upgrade to 500 tokens (only 200 available)
    assert!(!ctx.upgrade_depth(&path, Depth::Body, 500));
    // Original state preserved
    assert_eq!(ctx.used_tokens, 100);
    assert_eq!(ctx.files[0].depth, Depth::Overview);
}

#[test]
fn test_utilization() {
    let mut ctx = ContextWindow::new(1_000);
    ctx.system_tokens = 200;
    ctx.history_tokens = 300;
    ctx.add_file(ContextFile {
        path: PathBuf::from("a.rs"),
        depth: Depth::Structure,
        tokens: 100,
        relevance: 0.5,
        reason: "test".to_string(),
    });
    // (200 + 300 + 100) / 1000 = 0.6
    assert!((ctx.utilization() - 0.6).abs() < 0.001);
}

#[test]
fn test_utilization_zero_max() {
    let ctx = ContextWindow::new(0);
    assert_eq!(ctx.utilization(), 0.0);
}

#[test]
fn test_summary() {
    let mut ctx = ContextWindow::new(10_000);
    ctx.system_tokens = 1_000;
    ctx.history_tokens = 2_000;
    ctx.add_file(ContextFile {
        path: PathBuf::from("a.rs"),
        depth: Depth::Overview,
        tokens: 500,
        relevance: 0.5,
        reason: "test".to_string(),
    });
    let summary = ctx.summary();
    assert!(summary.contains("3500/10000"));
    assert!(summary.contains("1 files"));
}

#[test]
fn test_descriptor_cache_basic() {
    let cache = DescriptorCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_descriptor_cache_default() {
    let cache = DescriptorCache::default();
    assert!(cache.is_empty());
}

#[test]
fn test_selection_strategy_default() {
    let strategy = SelectionStrategy::default();
    assert_eq!(strategy.max_files, 20);
    assert!((strategy.min_relevance - 0.1).abs() < 0.001);
    assert_eq!(strategy.default_depth, Depth::Structure);
    assert!(strategy.prefer_recent);
}

#[test]
fn test_context_file_serialization() {
    let file = ContextFile {
        path: PathBuf::from("src/lib.rs"),
        depth: Depth::Detail,
        tokens: 350,
        relevance: 0.75,
        reason: "keyword match".to_string(),
    };

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: ContextFile = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.path, file.path);
    assert_eq!(deserialized.tokens, 350);
    assert!((deserialized.relevance - 0.75).abs() < 0.001);
}

#[test]
fn test_multiple_files_budget() {
    let mut ctx = ContextWindow::new(500);
    for i in 0..5 {
        let added = ctx.add_file(ContextFile {
            path: PathBuf::from(format!("file_{}.rs", i)),
            depth: Depth::Overview,
            tokens: 100,
            relevance: 0.5,
            reason: "batch".to_string(),
        });
        assert!(added);
    }
    assert_eq!(ctx.files.len(), 5);
    assert_eq!(ctx.used_tokens, 500);
    assert_eq!(ctx.available_tokens(), 0);

    // 6th file should fail
    let added = ctx.add_file(ContextFile {
        path: PathBuf::from("file_5.rs"),
        depth: Depth::Overview,
        tokens: 1,
        relevance: 0.5,
        reason: "overflow".to_string(),
    });
    assert!(!added);
}
