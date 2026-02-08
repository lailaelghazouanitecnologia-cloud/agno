use kkr_tool_discord::*;

use kkr_core::tool::{Tool, ToolCategory};

#[test]
fn test_webhook_tool_metadata() {
    let tool = DiscordWebhookTool::new();
    assert_eq!(tool.metadata().category, ToolCategory::Network);
    assert!(!tool.metadata().read_only);
}

#[test]
fn test_bot_tool_metadata() {
    let tool = DiscordBotTool::new();
    assert!(tool.metadata().requires.contains(&"DISCORD_BOT_TOKEN".to_string()));
}
