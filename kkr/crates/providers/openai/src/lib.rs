use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Result, Role, ToolCall};

#[derive(Debug, Clone, Copy, Default)]
pub enum OpenAIModel {
    #[default]
    Gpt4o,
    Gpt4oMini,
    Gpt4Turbo,
    Gpt4,
    Gpt35Turbo,
    O1Preview,
    O1Mini,
}

impl OpenAIModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIModel::Gpt4o => "gpt-4o",
            OpenAIModel::Gpt4oMini => "gpt-4o-mini",
            OpenAIModel::Gpt4Turbo => "gpt-4-turbo",
            OpenAIModel::Gpt4 => "gpt-4",
            OpenAIModel::Gpt35Turbo => "gpt-3.5-turbo",
            OpenAIModel::O1Preview => "o1-preview",
            OpenAIModel::O1Mini => "o1-mini",
        }
    }

    pub fn supports_tools(&self) -> bool {
        !matches!(self, OpenAIModel::O1Preview | OpenAIModel::O1Mini)
    }
}

#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: OpenAIModel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub base_url: String,
}

impl OpenAIConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: OpenAIModel::default(),
            max_tokens: None,
            temperature: None,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| kkr_core::Error::Config("OPENAI_API_KEY not set".into()))?;
        let mut config = Self::new(api_key);
        if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
            config.base_url = base_url;
        }
        Ok(config)
    }

    pub fn model(mut self, model: OpenAIModel) -> Self {
        self.model = model;
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

pub struct OpenAI {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAI {
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OpenAIConfig::from_env()?))
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<ApiMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };

                ApiMessage {
                    role: role.to_string(),
                    content: Some(msg.content.clone()),
                    name: msg.name.clone(),
                    tool_calls: msg.tool_calls.as_ref().map(|tcs| {
                        tcs.iter()
                            .map(|tc| ApiToolCall {
                                id: tc.id.clone(),
                                r#type: "function".to_string(),
                                function: ApiFunction {
                                    name: tc.name.clone(),
                                    arguments: serde_json::to_string(&tc.arguments)
                                        .unwrap_or_default(),
                                },
                            })
                            .collect()
                    }),
                    tool_call_id: msg.tool_call_id.clone(),
                }
            })
            .collect()
    }

    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<ApiTool> {
        tools
            .iter()
            .map(|t| {
                let parameters = serde_json::json!({
                    "type": t.parameters.schema_type,
                    "properties": t.parameters.properties,
                    "required": t.parameters.required
                });
                ApiTool {
                    r#type: "function".to_string(),
                    function: ApiToolFunction {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters,
                    },
                }
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OpenAI {
    fn name(&self) -> &str {
        "openai"
    }

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let api_messages = self.convert_messages(&messages);

        let mut request = ApiRequest {
            model: self.config.model.as_str().to_string(),
            messages: api_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            tools: None,
            tool_choice: None,
        };

        if self.config.model.supports_tools() {
            if let Some(ref t) = tools {
                if !t.is_empty() {
                    request.tools = Some(self.convert_tools(t));
                    request.tool_choice = Some("auto".to_string());
                }
            }
        }

        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Provider(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Provider(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Provider(format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body).map_err(|e| {
            kkr_core::Error::Provider(format!("Failed to parse response: {} - {}", e, body))
        })?;

        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| kkr_core::Error::Provider("No choices in response".into()))?;

        let tool_calls = choice.message.tool_calls.map(|tcs| {
            tcs.into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null),
                })
                .collect()
        });

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        };

        let usage = api_response.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ProviderResponse {
            message: Message {
                role: Role::Assistant,
                content: choice.message.content.unwrap_or_default(),
                name: None,
                tool_calls,
                tool_call_id: None,
            },
            finish_reason,
            usage,
        })
    }
}

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiToolCall {
    id: String,
    r#type: String,
    function: ApiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    r#type: String,
    function: ApiToolFunction,
}

#[derive(Debug, Serialize)]
struct ApiToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
struct ApiChoice {
    message: ApiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_str() {
        assert_eq!(OpenAIModel::Gpt4o.as_str(), "gpt-4o");
        assert_eq!(OpenAIModel::Gpt4oMini.as_str(), "gpt-4o-mini");
    }

    #[test]
    fn test_model_supports_tools() {
        assert!(OpenAIModel::Gpt4o.supports_tools());
        assert!(!OpenAIModel::O1Preview.supports_tools());
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAIConfig::new("test-key")
            .model(OpenAIModel::Gpt4Turbo)
            .max_tokens(8192)
            .temperature(0.7);

        assert_eq!(config.max_tokens, Some(8192));
        assert_eq!(config.temperature, Some(0.7));
    }
}
