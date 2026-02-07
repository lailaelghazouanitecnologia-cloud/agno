use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Result, Role, ToolCall};

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
    pub num_predict: Option<u32>,
    pub temperature: Option<f32>,
    pub num_ctx: Option<u32>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            num_predict: None,
            temperature: None,
            num_ctx: None,
        }
    }
}

impl OllamaConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());

        Ok(Self {
            base_url,
            model,
            ..Default::default()
        })
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn num_predict(mut self, n: u32) -> Self {
        self.num_predict = Some(n);
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn num_ctx(mut self, n: u32) -> Self {
        self.num_ctx = Some(n);
        self
    }
}

pub struct Ollama {
    config: OllamaConfig,
    client: Client,
}

impl Ollama {
    pub fn new(config: OllamaConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OllamaConfig::from_env()?))
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
                    content: msg.content.clone(),
                    tool_calls: msg.tool_calls.as_ref().map(|tcs| {
                        tcs.iter()
                            .map(|tc| ApiToolCall {
                                function: ApiFunction {
                                    name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                },
                            })
                            .collect()
                    }),
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
impl Provider for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let api_messages = self.convert_messages(&messages);

        let mut options = ApiOptions {
            num_predict: self.config.num_predict,
            temperature: self.config.temperature,
            num_ctx: self.config.num_ctx,
        };

        if options.num_predict.is_none() {
            options.num_predict = Some(4096);
        }

        let mut request = ApiRequest {
            model: self.config.model.clone(),
            messages: api_messages,
            stream: false,
            options: Some(options),
            tools: None,
        };

        if let Some(ref t) = tools {
            if !t.is_empty() {
                request.tools = Some(self.convert_tools(t));
            }
        }

        let url = format!("{}/api/chat", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::error::provider("ollama",format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::error::provider("ollama",format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::error::provider("ollama",format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body).map_err(|e| {
            kkr_core::error::provider("ollama",format!("Failed to parse response: {} - {}", e, body))
        })?;

        let tool_calls = api_response.message.tool_calls.map(|tcs| {
            tcs.into_iter()
                .enumerate()
                .map(|(i, tc)| ToolCall {
                    id: format!("call_{}", i),
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                })
                .collect()
        });

        let has_tool_calls = tool_calls.as_ref().map(|v: &Vec<_>| !v.is_empty()).unwrap_or(false);

        let finish_reason = if api_response.done {
            if has_tool_calls {
                FinishReason::ToolCalls
            } else {
                FinishReason::Stop
            }
        } else {
            FinishReason::Length
        };

        let usage = Some(Usage {
            prompt_tokens: api_response.prompt_eval_count.unwrap_or(0),
            completion_tokens: api_response.eval_count.unwrap_or(0),
            total_tokens: api_response.prompt_eval_count.unwrap_or(0)
                + api_response.eval_count.unwrap_or(0),
        });

        Ok(ProviderResponse {
            message: Message {
                role: Role::Assistant,
                content: api_response.message.content,
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
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ApiOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Serialize)]
struct ApiOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiToolCall {
    function: ApiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunction {
    name: String,
    arguments: serde_json::Value,
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
    message: ApiResponseMessage,
    done: bool,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ApiResponseMessage {
    content: String,
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "llama3.2");
    }

    #[test]
    fn test_config_builder() {
        let config = OllamaConfig::new("codellama")
            .base_url("http://192.168.1.100:11434")
            .num_predict(2048)
            .temperature(0.7)
            .num_ctx(8192);

        assert_eq!(config.model, "codellama");
        assert_eq!(config.num_predict, Some(2048));
        assert_eq!(config.num_ctx, Some(8192));
    }
}
