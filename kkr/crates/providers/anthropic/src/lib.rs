//! Anthropic provider (Claude)
//!
//! Implementation of the Claude API for KKR.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Role, ToolCall, Result};

/// Available Claude models
#[derive(Debug, Clone, Copy, Default)]
pub enum ClaudeModel {
    #[default]
    Claude3Opus,
    Claude3Sonnet,
    Claude3Haiku,
    Claude35Sonnet,
    Claude35Haiku,
}

impl ClaudeModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClaudeModel::Claude3Opus => "claude-3-opus-20240229",
            ClaudeModel::Claude3Sonnet => "claude-3-sonnet-20240229",
            ClaudeModel::Claude3Haiku => "claude-3-haiku-20240307",
            ClaudeModel::Claude35Sonnet => "claude-3-5-sonnet-20241022",
            ClaudeModel::Claude35Haiku => "claude-3-5-haiku-20241022",
        }
    }
}

/// Anthropic provider configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: ClaudeModel,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: ClaudeModel::default(),
            max_tokens: 4096,
            temperature: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| kkr_core::error::config("ANTHROPIC_API_KEY not set"))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: ClaudeModel) -> Self {
        self.model = model;
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Anthropic provider
pub struct Anthropic {
    config: AnthropicConfig,
    client: Client,
}

impl Anthropic {
    const API_URL: &'static str = "https://api.anthropic.com/v1/messages";
    const API_VERSION: &'static str = "2023-06-01";

    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(AnthropicConfig::from_env()?))
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<ApiMessage>) {
        let mut system = None;
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system = Some(msg.content.clone());
                }
                Role::User => {
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: ApiContent::Text(msg.content.clone()),
                    });
                }
                Role::Assistant => {
                    if let Some(ref tool_calls) = msg.tool_calls {
                        // Convert tool calls to tool_use blocks
                        let blocks: Vec<ContentBlock> = tool_calls
                            .iter()
                            .map(|tc| ContentBlock::ToolUse {
                                id: tc.id.clone(),
                                name: tc.name.clone(),
                                input: tc.arguments.clone(),
                            })
                            .collect();

                        // Add text if present
                        let mut all_blocks = Vec::new();
                        if !msg.content.is_empty() {
                            all_blocks.push(ContentBlock::Text { text: msg.content.clone() });
                        }
                        all_blocks.extend(blocks);

                        api_messages.push(ApiMessage {
                            role: "assistant".to_string(),
                            content: ApiContent::Blocks(all_blocks),
                        });
                    } else {
                        api_messages.push(ApiMessage {
                            role: "assistant".to_string(),
                            content: ApiContent::Text(msg.content.clone()),
                        });
                    }
                }
                Role::Tool => {
                    // Tool results go as user messages with tool_result block
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: ApiContent::Blocks(vec![ContentBlock::ToolResult {
                            tool_use_id: msg.tool_call_id.clone().unwrap_or_default(),
                            content: msg.content.clone(),
                        }]),
                    });
                }
            }
        }

        (system, api_messages)
    }

    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<ApiTool> {
        tools
            .iter()
            .map(|t| {
                let input_schema = serde_json::json!({
                    "type": t.parameters.schema_type,
                    "properties": t.parameters.properties,
                    "required": t.parameters.required
                });
                ApiTool {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema,
                }
            })
            .collect()
    }
}

#[async_trait]
impl Provider for Anthropic {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let (system, api_messages) = self.convert_messages(&messages);

        let mut request = ApiRequest {
            model: self.config.model.as_str().to_string(),
            max_tokens: self.config.max_tokens,
            system,
            messages: api_messages,
            tools: None,
            temperature: self.config.temperature,
        };

        if let Some(ref t) = tools {
            if !t.is_empty() {
                request.tools = Some(self.convert_tools(t));
            }
        }

        let response = self
            .client
            .post(Self::API_URL)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", Self::API_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::error::provider("anthropic",format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::error::provider("anthropic",format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::error::provider("anthropic",format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::error::provider("anthropic",format!("Failed to parse response: {} - {}", e, body)))?;

        // Convert response to our format
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &api_response.content {
            match block {
                ContentBlock::Text { text } => {
                    content.push_str(text);
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: input.clone(),
                    });
                }
                _ => {}
            }
        }

        let finish_reason = match api_response.stop_reason.as_deref() {
            Some("end_turn") => FinishReason::Stop,
            Some("tool_use") => FinishReason::ToolCalls,
            Some("max_tokens") => FinishReason::Length,
            _ => FinishReason::Stop,
        };

        let usage = api_response.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        Ok(ProviderResponse {
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
                tool_call_id: None,
            },
            finish_reason,
            usage,
        })
    }
}

// API types

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: ApiContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_str() {
        assert_eq!(ClaudeModel::Claude35Sonnet.as_str(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_config_builder() {
        let config = AnthropicConfig::new("test-key")
            .model(ClaudeModel::Claude35Sonnet)
            .max_tokens(8192)
            .temperature(0.7);

        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.temperature, Some(0.7));
    }
}
