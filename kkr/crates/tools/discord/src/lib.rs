use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

pub struct DiscordWebhookTool {
    client: Client,
    webhook_url: Option<String>,
}

impl Default for DiscordWebhookTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordWebhookTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
        }
    }

    pub fn with_webhook_url(mut self, url: impl Into<String>) -> Self {
        self.webhook_url = Some(url.into());
        self
    }
}

#[derive(Debug, Serialize)]
struct WebhookMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<Embed>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<EmbedField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<EmbedMedia>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedField {
    name: String,
    value: String,
    #[serde(default)]
    inline: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedFooter {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedMedia {
    url: String,
}

#[derive(Debug, Deserialize)]
struct WebhookParams {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    embeds: Option<Vec<Embed>>,
    #[serde(default)]
    webhook_url: Option<String>,
}

#[async_trait]
impl Tool for DiscordWebhookTool {
    fn name(&self) -> &str {
        "discord_webhook"
    }

    fn description(&self) -> &str {
        "Send a message to Discord via webhook. Supports rich embeds with titles, colors, and fields."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "content": {
                    "type": "string",
                    "description": "Message content (max 2000 chars)"
                },
                "username": {
                    "type": "string",
                    "description": "Override webhook username"
                },
                "avatar_url": {
                    "type": "string",
                    "description": "Override webhook avatar URL"
                },
                "webhook_url": {
                    "type": "string",
                    "description": "Webhook URL (overrides default)"
                },
                "embeds": {
                    "type": "array",
                    "description": "Rich embed objects",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "url": {"type": "string"},
                            "color": {"type": "integer", "description": "Decimal color value"},
                            "fields": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"},
                                        "value": {"type": "string"},
                                        "inline": {"type": "boolean"}
                                    }
                                }
                            }
                        }
                    }
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["discord", "webhook", "message", "communication", "chat"])
            .with_read_only(false)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: WebhookParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let webhook_url = params
            .webhook_url
            .as_ref()
            .or(self.webhook_url.as_ref())
            .ok_or_else(|| kkr_core::Error::tool("No webhook URL provided".to_string()))?;

        if params.content.is_none() && params.embeds.is_none() {
            return Err(kkr_core::Error::tool("Either content or embeds must be provided".to_string()));
        }

        let message = WebhookMessage {
            content: params.content,
            username: params.username,
            avatar_url: params.avatar_url,
            embeds: params.embeds,
        };

        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!("Discord API error ({}): {}", status, body)));
        }

        Ok(json!({
            "success": true,
            "message": "Message sent to Discord"
        }))
    }
}

pub struct DiscordBotTool {
    client: Client,
    bot_token: Option<String>,
}

impl Default for DiscordBotTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordBotTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            bot_token: env::var("DISCORD_BOT_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.bot_token = Some(token.into());
        self
    }

    fn get_auth_header(&self) -> Result<String> {
        let env_token = env::var("DISCORD_BOT_TOKEN").ok();
        let token = self
            .bot_token
            .as_ref()
            .or(env_token.as_ref())
            .ok_or_else(|| kkr_core::Error::tool("DISCORD_BOT_TOKEN not set".to_string()))?;
        Ok(format!("Bot {}", token))
    }
}

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    channel_id: String,
    content: String,
    #[serde(default)]
    embeds: Option<Vec<Embed>>,
}

#[async_trait]
impl Tool for DiscordBotTool {
    fn name(&self) -> &str {
        "discord_send_message"
    }

    fn description(&self) -> &str {
        "Send a message to a Discord channel using bot token. Requires bot to be in the server."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "channel_id": {
                    "type": "string",
                    "description": "Discord channel ID"
                },
                "content": {
                    "type": "string",
                    "description": "Message content"
                },
                "embeds": {
                    "type": "array",
                    "description": "Rich embed objects"
                }
            }),
            required: vec!["channel_id".to_string(), "content".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["discord", "bot", "message", "channel", "communication"])
            .with_read_only(false)
            .with_priority(70)
            .with_requires(vec!["DISCORD_BOT_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: SendMessageParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let auth = self.get_auth_header()?;

        let mut body = json!({
            "content": params.content
        });

        if let Some(embeds) = params.embeds {
            body["embeds"] = json!(embeds);
        }

        let url = format!("{}/channels/{}/messages", DISCORD_API_BASE, params.channel_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Discord API error: {}", body)));
        }

        Ok(json!({
            "success": true,
            "message_id": body.get("id"),
            "channel_id": params.channel_id
        }))
    }
}

pub struct DiscordGetChannelsTool {
    client: Client,
    bot_token: Option<String>,
}

impl Default for DiscordGetChannelsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordGetChannelsTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            bot_token: env::var("DISCORD_BOT_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.bot_token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct GetChannelsParams {
    guild_id: String,
}

#[async_trait]
impl Tool for DiscordGetChannelsTool {
    fn name(&self) -> &str {
        "discord_get_channels"
    }

    fn description(&self) -> &str {
        "Get all channels in a Discord server (guild)."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "guild_id": {
                    "type": "string",
                    "description": "Discord server (guild) ID"
                }
            }),
            required: vec!["guild_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["discord", "channels", "guild", "list"])
            .with_read_only(true)
            .with_priority(60)
            .with_requires(vec!["DISCORD_BOT_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GetChannelsParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let env_token = env::var("DISCORD_BOT_TOKEN").ok();
        let token = self
            .bot_token
            .as_ref()
            .or(env_token.as_ref())
            .ok_or_else(|| kkr_core::Error::tool("DISCORD_BOT_TOKEN not set".to_string()))?;

        let url = format!("{}/guilds/{}/channels", DISCORD_API_BASE, params.guild_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Discord API error: {}", body)));
        }

        let channels: Vec<Value> = body
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|ch| {
                        json!({
                            "id": ch.get("id"),
                            "name": ch.get("name"),
                            "type": ch.get("type"),
                            "position": ch.get("position")
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "guild_id": params.guild_id,
            "channels": channels,
            "count": channels.len()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
