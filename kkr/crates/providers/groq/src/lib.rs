use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Result, Role, ToolCall};

#[derive(Debug, Clone, Copy, Default)]
pub enum GroqModel {
    #[default]
    Llama33_70B,
    Llama31_8B,
    Llama4Scout,
    Llama4Maverick,
    DeepSeekR1Llama70B,
    DeepSeekR1Qwen32B,
    QwenQwQ32B,
    Qwen3_32B,
}

impl GroqModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroqModel::Llama33_70B => "llama-3.3-70b-versatile",
            GroqModel::Llama31_8B => "llama-3.1-8b-instant",
            GroqModel::Llama4Scout => "meta-llama/llama-4-scout-17b-16e-instruct",
            GroqModel::Llama4Maverick => "meta-llama/llama-4-maverick-17b-128e-instruct",
            GroqModel::DeepSeekR1Llama70B => "deepseek-r1-distill-llama-70b",
            GroqModel::DeepSeekR1Qwen32B => "deepseek-r1-distill-qwen-32b",
            GroqModel::QwenQwQ32B => "qwen-qwq-32b",
            GroqModel::Qwen3_32B => "qwen3-32b",
        }
    }

    pub fn is_reasoning_model(&self) -> bool {
        matches!(
            self,
            GroqModel::DeepSeekR1Llama70B
                | GroqModel::DeepSeekR1Qwen32B
                | GroqModel::QwenQwQ32B
        )
    }

    pub fn supports_vision(&self) -> bool {
        matches!(self, GroqModel::Llama4Scout | GroqModel::Llama4Maverick)
    }
}

#[derive(Debug, Clone)]
pub struct GroqConfig {
    pub api_key: String,
    pub model: GroqModel,
    pub max_completion_tokens: u32,
    pub temperature: Option<f32>,
    pub include_reasoning: bool,
}

impl GroqConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: GroqModel::default(),
            max_completion_tokens: 4096,
            temperature: None,
            include_reasoning: false,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("GROQ_API_KEY")
            .map_err(|_| kkr_core::error::config("GROQ_API_KEY not set"))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: GroqModel) -> Self {
        self.model = model;
        self
    }

    pub fn max_completion_tokens(mut self, tokens: u32) -> Self {
        self.max_completion_tokens = tokens;
        self
    }

    #[deprecated(since = "0.2.0", note = "Use max_completion_tokens instead")]
    pub fn max_tokens(self, max_tokens: u32) -> Self {
        self.max_completion_tokens(max_tokens)
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn include_reasoning(mut self, include: bool) -> Self {
        self.include_reasoning = include;
        self
    }
}

pub struct Groq {
    config: GroqConfig,
    client: Client,
}

impl Groq {
    const API_URL: &'static str = "https://api.groq.com/openai/v1/chat/completions";

    pub fn new(config: GroqConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GroqConfig::from_env()?))
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
impl Provider for Groq {
    fn name(&self) -> &str {
        "groq"
    }

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let api_messages = self.convert_messages(&messages);

        let include_reasoning = if self.config.model.is_reasoning_model() {
            Some(self.config.include_reasoning)
        } else {
            None
        };

        let mut request = ApiRequest {
            model: self.config.model.as_str().to_string(),
            messages: api_messages,
            max_completion_tokens: Some(self.config.max_completion_tokens),
            temperature: self.config.temperature,
            tools: None,
            tool_choice: None,
            include_reasoning,
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
            .map_err(|e| kkr_core::error::provider("groq",format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::error::provider("groq",format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::error::provider("groq",format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body).map_err(|e| {
            kkr_core::error::provider("groq",format!("Failed to parse response: {} - {}", e, body))
        })?;

        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| kkr_core::error::provider("groq", "No choices in response"))?;

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

        let mut content = choice.message.content.unwrap_or_default();
        if let Some(reasoning) = choice.message.reasoning {
            if !reasoning.is_empty() {
                content = format!("<reasoning>\n{}\n</reasoning>\n\n{}", reasoning, content);
            }
        }

        Ok(ProviderResponse {
            message: Message {
                role: Role::Assistant,
                content,
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
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_reasoning: Option<bool>,
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
    reasoning: Option<String>,
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
        assert_eq!(GroqModel::Llama33_70B.as_str(), "llama-3.3-70b-versatile");
        assert_eq!(
            GroqModel::DeepSeekR1Llama70B.as_str(),
            "deepseek-r1-distill-llama-70b"
        );
    }

    #[test]
    fn test_reasoning_models() {
        assert!(GroqModel::DeepSeekR1Llama70B.is_reasoning_model());
        assert!(GroqModel::QwenQwQ32B.is_reasoning_model());
        assert!(!GroqModel::Llama33_70B.is_reasoning_model());
    }

    #[test]
    fn test_vision_models() {
        assert!(GroqModel::Llama4Scout.supports_vision());
        assert!(!GroqModel::Llama33_70B.supports_vision());
    }

    #[test]
    fn test_config_builder() {
        let config = GroqConfig::new("test-key")
            .model(GroqModel::DeepSeekR1Llama70B)
            .max_completion_tokens(8192)
            .temperature(0.5)
            .include_reasoning(true);

        assert_eq!(config.max_completion_tokens, 8192);
        assert_eq!(config.temperature, Some(0.5));
        assert!(config.include_reasoning);
    }
}
