use entity_cli::*;
use entity_runner::RunResult;
use std::path::PathBuf;

#[test]
fn test_cli_config_default() {
    let config = CliConfig::default();
    assert!(config.color);
    assert!(!config.verbose);
    assert!(config.show_usage);
    assert_eq!(config.max_width, 120);
}

#[test]
fn test_terminal_output_creation() {
    let config = CliConfig::default();
    let _output = TerminalOutput::new(config);
    // Should not panic
}

#[test]
fn test_terminal_approval_creation() {
    let config = CliConfig::default();
    let _approval = TerminalApproval::new(config);
}

#[test]
fn test_spinner_creation_and_stop() {
    let mut spinner = Spinner::new("Loading...");
    spinner.stop_with_message("done");
}

#[test]
fn test_spinner_drop() {
    // Spinner should handle drop gracefully even if not stopped
    let _spinner = Spinner::new("Working...");
    // drop happens here
}

#[test]
fn test_display_result_success() {
    let result = RunResult {
        success: true,
        output: "Completed".to_string(),
        turns: 3,
        total_tokens: 5000,
        files_modified: vec![PathBuf::from("src/lib.rs")],
        errors: Vec::new(),
    };
    let config = CliConfig { color: false, ..CliConfig::default() };
    // Should not panic
    display_result(&result, &config);
}

#[test]
fn test_display_result_failure() {
    let result = RunResult {
        success: false,
        output: String::new(),
        turns: 1,
        total_tokens: 100,
        files_modified: Vec::new(),
        errors: vec!["timeout".to_string()],
    };
    let config = CliConfig { color: false, verbose: true, ..CliConfig::default() };
    display_result(&result, &config);
}

#[test]
fn test_display_result_with_colors() {
    let result = RunResult {
        success: true,
        output: "Done".to_string(),
        turns: 1,
        total_tokens: 50,
        files_modified: Vec::new(),
        errors: Vec::new(),
    };
    let config = CliConfig::default(); // color = true
    display_result(&result, &config);
}

#[test]
fn test_display_session_info() {
    let session = entity_session::Session::new(entity_session::SessionConfig::default());
    let config = CliConfig { color: false, ..CliConfig::default() };
    display_session_info(&session, &config);
}

#[test]
fn test_display_session_info_colored() {
    let session = entity_session::Session::new(entity_session::SessionConfig::default());
    let config = CliConfig::default();
    display_session_info(&session, &config);
}
