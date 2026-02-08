use kkr_tool_email::*;

use kkr_core::tool::{Tool, ToolCategory};

#[test]
fn test_smtp_config_gmail() {
    let config = SmtpConfig::gmail("user@gmail.com", "app-password")
        .from_email("user@gmail.com")
        .from_name("Test User");

    assert_eq!(config.host, "smtp.gmail.com");
    assert_eq!(config.port, 587);
}

#[test]
fn test_resend_tool_metadata() {
    let tool = ResendEmailTool::new("test@example.com");
    assert_eq!(tool.metadata().category, ToolCategory::Network);
}

#[test]
fn test_gmail_credentials_from_env() {
    let creds = GmailCredentials::new("client_id", "client_secret")
        .with_access_token("token")
        .with_refresh_token("refresh");

    assert_eq!(creds.client_id, "client_id");
    assert!(creds.access_token.is_some());
}

#[test]
fn test_gmail_read_tool_metadata() {
    let creds = GmailCredentials::new("id", "secret").with_access_token("token");
    let tool = GmailReadTool::new(GmailClient::new(creds));

    assert_eq!(tool.name(), "gmail_read");
    assert!(tool.metadata().read_only);
}
