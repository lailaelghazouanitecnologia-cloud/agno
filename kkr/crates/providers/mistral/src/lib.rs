use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Result, Role, ToolCall};

#[derive(Debug, Clone, Copy, Default)]
pub enum MistralModel {
    #[default]
    MistralLarge,
    MistralMedium,
    MistralSmall,
    Codestral,
    Pixtral,
    MistralNemo,
    Ministral3B,
    Ministral8B,
}

impl MistralModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MistralModel::MistralLarge => "mistral-large-latest",
            MistralModel::MistralMedium => "mistral-medium-latest",
            MistralModel::MistralSmall => "mistral-small-latest",
            MistralModel::Codestral => "codestral-latest",
            MistralModel::Pixtral => "pixtral-12b-2409",
            MistralModel::MistralNemo => "open-mistral-nemo",
            MistralModel::Ministral3B => "ministral-3b-latest",
            MistralModel::Ministral8B => "ministral-8b-latest",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MistralConfig {
    pub api_key: String,
    pub model: MistralModel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl MistralConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: MistralModel::default(),
            max_tokens: None,
            temperature: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("MISTRAL_API_KEY")
            .map_err(|_| kkr_core::Error::Config("MISTRAL_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: MistralModel) -> Self {
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
}

pub struct Mistral {
    config: MistralConfig,
    client: Client,
}

impl Mistral {
    const API_URL: &'static str = "https://api.mistral.ai/v1/chat/completions";

    pub fn new(config: MistralConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(MistralConfig::from_env()?))
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
impl Provider for Mistral {
    fn name(&self) -> &str {
        "mistral"
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

        if let Some(ref t) = tools {
            if !t.is_empty() {
                request.tools = Some(self.convert_tools(t));
                request.tool_choice = Some("auto".to_string());
            }
        }

        let response = self
            .client
            .post(Self::API_URL)
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
            Some("model_length") => FinishReason::Length,
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
        assert_eq!(MistralModel::MistralLarge.as_str(), "mistral-large-latest");
        assert_eq!(MistralModel::Codestral.as_str(), "codestral-latest");
    }

    #[test]
    fn test_config_builder() {
        let config = MistralConfig::new("test-key")
            .model(MistralModel::Codestral)
            .max_tokens(8192)
            .temperature(0.3);

        assert_eq!(config.max_tokens, Some(8192));
        assert_eq!(config.temperature, Some(0.3));
    }
}
