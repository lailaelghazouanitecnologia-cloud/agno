use entity_safety::*;
use std::path::PathBuf;

#[test]
fn test_default_safety_config() {
    let config = SafetyConfig::default();
    assert!(config.fs_policy.allow_writes);
    assert!(!config.fs_policy.allow_deletes);
    assert!(!config.net_policy.allow_network);
    assert_eq!(config.approval_policy, ApprovalPolicy::WriteOnly);
    assert_eq!(config.rate_limits.tool_calls_per_minute, 60);
}

#[test]
fn test_default_cmd_policy_blocks_dangerous() {
    let config = SafetyConfig::default();
    assert!(!config.cmd_policy.blocked_commands.is_empty());
    assert!(config.cmd_policy.blocked_commands.iter().any(|c| c.contains("rm -rf /")));
}

#[test]
fn test_file_read_allowed() {
    let config = SafetyConfig::default();
    let guard = SafetyGuard::new(config);
    match guard.check_file_read(&PathBuf::from("/home/user/project/src/main.rs")) {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn test_file_read_blocked_pattern() {
    let config = SafetyConfig::default();
    let guard = SafetyGuard::new(config);
    match guard.check_file_read(&PathBuf::from("/home/user/project/.env")) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("blocked pattern"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_read_blocked_credentials() {
    let config = SafetyConfig::default();
    let guard = SafetyGuard::new(config);
    match guard.check_file_read(&PathBuf::from("/home/user/credentials.json")) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("blocked pattern"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_read_path_traversal() {
    let config = SafetyConfig::default();
    let guard = SafetyGuard::new(config);
    match guard.check_file_read(&PathBuf::from("/home/user/../etc/passwd")) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("traversal"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_read_allowed_root() {
    let config = SafetyConfig {
        fs_policy: FsPolicy {
            allowed_roots: vec![PathBuf::from("/home/user/project")],
            ..FsPolicy::default()
        },
        ..SafetyConfig::default()
    };
    let guard = SafetyGuard::new(config);

    // Allowed: under root
    match guard.check_file_read(&PathBuf::from("/home/user/project/src/lib.rs")) {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }

    // Denied: outside root
    match guard.check_file_read(&PathBuf::from("/etc/passwd")) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("not under any allowed root"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_write_disabled() {
    let config = SafetyConfig {
        fs_policy: FsPolicy {
            allow_writes: false,
            ..FsPolicy::default()
        },
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_file_write(&PathBuf::from("/tmp/test.txt"), 100) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("writes are disabled"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_write_size_limit() {
    let config = SafetyConfig {
        fs_policy: FsPolicy {
            max_write_size: 1024,
            ..FsPolicy::default()
        },
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_file_write(&PathBuf::from("/tmp/big.bin"), 2048) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("exceeds max"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_file_write_needs_approval_default() {
    let config = SafetyConfig::default(); // WriteOnly policy
    let mut guard = SafetyGuard::new(config);
    match guard.check_file_write(&PathBuf::from("/tmp/test.txt"), 100) {
        SafetyVerdict::NeedsApproval(_) => {}
        other => panic!("expected NeedsApproval, got {:?}", other),
    }
}

#[test]
fn test_command_blocked() {
    let config = SafetyConfig::default();
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("rm -rf /") {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("blocked"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_command_blocked_mkfs() {
    let config = SafetyConfig::default();
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("mkfs.ext4 /dev/sda1") {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("blocked"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_command_allowed_safe() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::DangerousOnly,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("cargo test") {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn test_command_dangerous_needs_approval() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::DangerousOnly,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("git push origin main") {
        SafetyVerdict::NeedsApproval(_) => {}
        other => panic!("expected NeedsApproval, got {:?}", other),
    }
}

#[test]
fn test_command_allowlist() {
    let config = SafetyConfig {
        cmd_policy: CmdPolicy {
            allowed_prefixes: vec!["cargo".to_string(), "git".to_string()],
            ..CmdPolicy::default()
        },
        approval_policy: ApprovalPolicy::Never,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);

    // Allowed
    match guard.check_command("cargo test") {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }

    // Not in allowlist
    match guard.check_command("python script.py") {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("not in allowlist"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn test_approval_policy_never() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::Never,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("echo hello") {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn test_approval_policy_always() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::Always,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_command("echo hello") {
        SafetyVerdict::NeedsApproval(_) => {}
        other => panic!("expected NeedsApproval, got {:?}", other),
    }
}

#[test]
fn test_tool_call_read_only_no_approval() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::WriteOnly,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_tool_call("file_read", true) {
        SafetyVerdict::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn test_tool_call_write_needs_approval() {
    let config = SafetyConfig {
        approval_policy: ApprovalPolicy::WriteOnly,
        ..SafetyConfig::default()
    };
    let mut guard = SafetyGuard::new(config);
    match guard.check_tool_call("file_write", false) {
        SafetyVerdict::NeedsApproval(_) => {}
        other => panic!("expected NeedsApproval, got {:?}", other),
    }
}

#[test]
fn test_record_tracking() {
    let config = SafetyConfig::default();
    let mut guard = SafetyGuard::new(config);
    guard.record_tool_call();
    guard.record_write();
    guard.record_command();
    // Should not panic
}

#[test]
fn test_safety_config_builder() {
    let config = SafetyConfigBuilder::new()
        .with_working_dir("/home/user/project")
        .allow_writes(true)
        .allow_deletes(false)
        .block_path("/home/user/project/.git")
        .approval_policy(ApprovalPolicy::DangerousOnly)
        .allow_network(false)
        .max_timeout(60)
        .build();

    assert_eq!(config.fs_policy.allowed_roots.len(), 1);
    assert!(config.fs_policy.allow_writes);
    assert!(!config.fs_policy.allow_deletes);
    assert_eq!(config.fs_policy.blocked_paths.len(), 1);
    assert_eq!(config.approval_policy, ApprovalPolicy::DangerousOnly);
    assert!(!config.net_policy.allow_network);
    assert_eq!(config.cmd_policy.max_timeout_secs, 60);
}

#[test]
fn test_blocked_path() {
    let config = SafetyConfig {
        fs_policy: FsPolicy {
            blocked_paths: vec![PathBuf::from("/secret")],
            ..FsPolicy::default()
        },
        ..SafetyConfig::default()
    };
    let guard = SafetyGuard::new(config);
    match guard.check_file_read(&PathBuf::from("/secret/data.txt")) {
        SafetyVerdict::Deny(reason) => {
            assert!(reason.contains("blocked directory"));
        }
        other => panic!("expected Deny, got {:?}", other),
    }
}
