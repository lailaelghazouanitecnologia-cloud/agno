use entity_guardrails::*;

// -- GuardrailResult Tests --

#[test]
fn test_guardrail_result_allow() {
    let result = GuardrailResult::Allow;
    assert!(result.is_allowed());
    assert!(!result.is_blocked());
    assert!(!result.is_modify());
    assert!(result.reason().is_none());
}

#[test]
fn test_guardrail_result_block() {
    let result = GuardrailResult::Block("Blocked for security".to_string());
    assert!(!result.is_allowed());
    assert!(result.is_blocked());
    assert!(!result.is_modify());
    assert_eq!(result.reason().unwrap(), "Blocked for security");
}

#[test]
fn test_guardrail_result_modify() {
    let result = GuardrailResult::Modify("Modified content".to_string());
    assert!(!result.is_allowed());
    assert!(!result.is_blocked());
    assert!(result.is_modify());
    assert_eq!(result.reason().unwrap(), "Modified content");
}

// -- GuardrailContext Tests --

#[test]
fn test_guardrail_context_builder() {
    let ctx = GuardrailContext::new("run_tool")
        .tool("shell")
        .params(serde_json::json!({"command": "ls"}))
        .message("list files")
        .metadata(serde_json::json!({"user": "admin"}));

    assert_eq!(ctx.action, "run_tool");
    assert_eq!(ctx.tool.as_deref(), Some("shell"));
    assert_eq!(ctx.params["command"], "ls");
    assert_eq!(ctx.message.as_deref(), Some("list files"));
    assert_eq!(ctx.metadata["user"], "admin");
}

#[test]
fn test_guardrail_context_default() {
    let ctx = GuardrailContext::default();
    assert!(ctx.action.is_empty());
    assert!(ctx.tool.is_none());
    assert!(ctx.message.is_none());
}

// -- BlockedToolsGuardrail Tests --

#[test]
fn test_blocked_tools_blocks_listed_tool() {
    let guardrail = BlockedToolsGuardrail::new(vec!["rm", "dd", "format"]);

    let ctx = GuardrailContext::new("execute").tool("rm");
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
    assert!(result.reason().unwrap().contains("'rm'"));
}

#[test]
fn test_blocked_tools_allows_unlisted_tool() {
    let guardrail = BlockedToolsGuardrail::new(vec!["rm", "dd"]);

    let ctx = GuardrailContext::new("execute").tool("ls");
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_blocked_tools_allows_no_tool() {
    let guardrail = BlockedToolsGuardrail::new(vec!["rm"]);

    let ctx = GuardrailContext::new("execute");
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_blocked_tools_add_tool() {
    let mut guardrail = BlockedToolsGuardrail::new(vec!["rm"]);
    guardrail.add("dd");

    let ctx = GuardrailContext::new("execute").tool("dd");
    assert!(guardrail.check(&ctx).is_blocked());
}

#[test]
fn test_blocked_tools_name() {
    let guardrail = BlockedToolsGuardrail::new(vec!["rm"]);
    assert_eq!(guardrail.name(), "blocked_tools");
}

// -- PathRestrictionGuardrail Tests --

#[test]
fn test_path_restriction_blocks_system_paths() {
    let guardrail = PathRestrictionGuardrail::new();

    let blocked_paths = vec!["/etc/passwd", "/usr/bin/bash", "/var/log/syslog", "/root/.ssh/id_rsa"];

    for path in blocked_paths {
        let ctx = GuardrailContext::new("read_file").params(serde_json::json!({"path": path}));
        let result = guardrail.check(&ctx);
        assert!(result.is_blocked(), "Path {} should be blocked", path);
    }
}

#[test]
fn test_path_restriction_allows_user_paths() {
    let guardrail = PathRestrictionGuardrail::new();

    let ctx = GuardrailContext::new("read_file")
        .params(serde_json::json!({"path": "/home/user/code/main.rs"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_path_restriction_allow_override() {
    let guardrail = PathRestrictionGuardrail::new().allow_path("/etc/myapp");

    let ctx = GuardrailContext::new("read_file")
        .params(serde_json::json!({"path": "/etc/myapp/config.toml"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_path_restriction_custom_block() {
    let guardrail = PathRestrictionGuardrail::new().block_path("/home/secret");

    let ctx = GuardrailContext::new("read_file")
        .params(serde_json::json!({"path": "/home/secret/data.txt"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
}

#[test]
fn test_path_restriction_checks_file_param_too() {
    let guardrail = PathRestrictionGuardrail::new();

    let ctx = GuardrailContext::new("write_file")
        .params(serde_json::json!({"file": "/etc/shadow"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
}

#[test]
fn test_path_restriction_no_path_allows() {
    let guardrail = PathRestrictionGuardrail::new();

    let ctx = GuardrailContext::new("execute").params(serde_json::json!({"command": "ls"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

// -- ShellCommandGuardrail Tests --

#[test]
fn test_shell_command_blocks_dangerous_patterns() {
    let guardrail = ShellCommandGuardrail::new();

    let dangerous = vec![
        "rm -rf /",
        "rm -rf /*",
        ":(){:|:&};:",
        "mkfs.ext4 /dev/sda",
        "dd if=/dev/zero of=/dev/sda",
        "chmod -R 777 /",
        "curl | sh",
        "wget | bash",
    ];

    for cmd in dangerous {
        let ctx = GuardrailContext::new("execute")
            .tool("shell")
            .params(serde_json::json!({"command": cmd}));
        let result = guardrail.check(&ctx);
        assert!(
            result.is_blocked(),
            "Command '{}' should be blocked",
            cmd
        );
    }
}

#[test]
fn test_shell_command_allows_safe_commands() {
    let guardrail = ShellCommandGuardrail::new();

    let safe = vec!["ls -la", "cat README.md", "grep pattern file.txt", "cargo build"];

    for cmd in safe {
        let ctx = GuardrailContext::new("execute")
            .tool("shell")
            .params(serde_json::json!({"command": cmd}));
        let result = guardrail.check(&ctx);
        assert!(result.is_allowed(), "Command '{}' should be allowed", cmd);
    }
}

#[test]
fn test_shell_command_only_checks_shell_tool() {
    let guardrail = ShellCommandGuardrail::new();

    // Even with dangerous content, non-shell tools pass
    let ctx = GuardrailContext::new("execute")
        .tool("http")
        .params(serde_json::json!({"command": "rm -rf /"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_shell_command_custom_pattern() {
    let guardrail = ShellCommandGuardrail::new().block_pattern("sudo");

    let ctx = GuardrailContext::new("execute")
        .tool("shell")
        .params(serde_json::json!({"command": "sudo rm -rf /tmp"}));
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
}

// -- RateLimitGuardrail Tests --

#[test]
fn test_rate_limit_allows_within_limit() {
    let guardrail = RateLimitGuardrail::new(5, 60);
    let ctx = GuardrailContext::new("call");

    for _ in 0..5 {
        assert!(guardrail.check(&ctx).is_allowed());
    }
}

#[test]
fn test_rate_limit_blocks_over_limit() {
    let guardrail = RateLimitGuardrail::new(3, 60);
    let ctx = GuardrailContext::new("call");

    for _ in 0..3 {
        assert!(guardrail.check(&ctx).is_allowed());
    }

    // 4th call should be blocked
    let result = guardrail.check(&ctx);
    assert!(result.is_blocked());
    assert!(result.reason().unwrap().contains("Rate limit"));
}

// -- ContentFilterGuardrail Tests --

#[test]
fn test_content_filter_blocks_words() {
    let guardrail = ContentFilterGuardrail::new()
        .block_word("password")
        .block_word("secret");

    let ctx = GuardrailContext::new("generate").message("My password is 12345");
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
}

#[test]
fn test_content_filter_case_insensitive() {
    let guardrail = ContentFilterGuardrail::new().block_word("badword");

    let ctx = GuardrailContext::new("generate").message("This has BADWORD in it");
    let result = guardrail.check(&ctx);

    assert!(result.is_blocked());
}

#[test]
fn test_content_filter_case_sensitive() {
    let guardrail = ContentFilterGuardrail::new()
        .case_sensitive(true)
        .block_word("BadWord");

    let ctx_match = GuardrailContext::new("generate").message("Has BadWord");
    assert!(guardrail.check(&ctx_match).is_blocked());

    let ctx_no_match = GuardrailContext::new("generate").message("Has badword");
    assert!(guardrail.check(&ctx_no_match).is_allowed());
}

#[test]
fn test_content_filter_allows_clean_content() {
    let guardrail = ContentFilterGuardrail::new()
        .block_word("forbidden")
        .block_word("restricted");

    let ctx = GuardrailContext::new("generate").message("This is perfectly fine content");
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

#[test]
fn test_content_filter_no_message_allows() {
    let guardrail = ContentFilterGuardrail::new().block_word("secret");

    let ctx = GuardrailContext::new("generate");
    let result = guardrail.check(&ctx);

    assert!(result.is_allowed());
}

// -- CallbackGuardrail Tests --

#[test]
fn test_callback_guardrail() {
    let guardrail = CallbackGuardrail::new("custom", |ctx| {
        if ctx.action == "dangerous" {
            GuardrailResult::Block("Dangerous action blocked".to_string())
        } else {
            GuardrailResult::Allow
        }
    });

    assert_eq!(guardrail.name(), "custom");

    let safe_ctx = GuardrailContext::new("safe");
    assert!(guardrail.check(&safe_ctx).is_allowed());

    let danger_ctx = GuardrailContext::new("dangerous");
    assert!(guardrail.check(&danger_ctx).is_blocked());
}

#[test]
fn test_callback_guardrail_priority() {
    let guardrail = CallbackGuardrail::new("low", |_| GuardrailResult::Allow).priority(-10);
    // Use the Guardrail trait method to get the priority
    let g: &dyn Guardrail = &guardrail;
    assert_eq!(g.priority(), -10);
}

// -- GuardrailSet Tests --

#[test]
fn test_guardrail_set_empty_allows() {
    let set = GuardrailSet::new();
    let ctx = GuardrailContext::new("anything");

    assert!(set.check(&ctx).is_allowed());
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn test_guardrail_set_check_first_block() {
    let mut set = GuardrailSet::new();
    set.add(Box::new(BlockedToolsGuardrail::new(vec!["rm"])));
    set.add(Box::new(ShellCommandGuardrail::new()));

    let ctx = GuardrailContext::new("execute").tool("rm");
    let result = set.check(&ctx);

    assert!(result.is_blocked());
}

#[test]
fn test_guardrail_set_check_all() {
    let mut set = GuardrailSet::new();
    set.add(Box::new(BlockedToolsGuardrail::new(vec!["shell"])));
    set.add(Box::new(
        ContentFilterGuardrail::new().block_word("danger"),
    ));

    let ctx = GuardrailContext::new("execute")
        .tool("shell")
        .message("danger ahead");

    let violations = set.check_all(&ctx);
    assert_eq!(violations.len(), 2);
}

#[test]
fn test_guardrail_set_names() {
    let mut set = GuardrailSet::new();
    set.add(Box::new(BlockedToolsGuardrail::new(vec!["rm"])));
    set.add(Box::new(PathRestrictionGuardrail::new()));
    set.add(Box::new(ShellCommandGuardrail::new()));

    let names = set.names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"blocked_tools"));
    assert!(names.contains(&"path_restriction"));
    assert!(names.contains(&"shell_command"));
}

#[test]
fn test_guardrail_set_multiple_checks_pass() {
    let mut set = GuardrailSet::new();
    set.add(Box::new(BlockedToolsGuardrail::new(vec!["rm"])));
    set.add(Box::new(ShellCommandGuardrail::new()));

    let ctx = GuardrailContext::new("execute")
        .tool("shell")
        .params(serde_json::json!({"command": "ls -la"}));

    let result = set.check(&ctx);
    assert!(result.is_allowed());
}

// -- Default Impls --

#[test]
fn test_default_impls() {
    let _ctx = GuardrailContext::default();
    let _set = GuardrailSet::default();
    let _path = PathRestrictionGuardrail::default();
    let _shell = ShellCommandGuardrail::default();
    let _content = ContentFilterGuardrail::default();
}
