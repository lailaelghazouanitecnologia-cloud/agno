use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use kkr_core::agent::{FinishReason, Provider, ProviderResponse, Usage};
use kkr_core::tool::ToolDefinition;
use kkr_core::{Message, Result, Role, ToolCall};

#[derive(Debug, Clone, Copy, Default)]
pub enum GeminiModel {
    #[default]
    Gemini25Flash,
    Gemini25Pro,
    Gemini25FlashLite,
    Gemini20Flash,
    Gemini20FlashLite,
    Gemini3Pro,
    Gemini3Flash,
    Gemini15Pro,
    Gemini15Flash,
}

impl GeminiModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GeminiModel::Gemini25Flash => "gemini-2.5-flash",
            GeminiModel::Gemini25Pro => "gemini-2.5-pro",
            GeminiModel::Gemini25FlashLite => "gemini-2.5-flash-lite",
            GeminiModel::Gemini20Flash => "gemini-2.0-flash",
            GeminiModel::Gemini20FlashLite => "gemini-2.0-flash-lite",
            GeminiModel::Gemini3Pro => "gemini-3-pro",
            GeminiModel::Gemini3Flash => "gemini-3-flash",
            GeminiModel::Gemini15Pro => "gemini-1.5-pro",
            GeminiModel::Gemini15Flash => "gemini-1.5-flash",
        }
    }

    pub fn is_reasoning_model(&self) -> bool {
        matches!(
            self,
            GeminiModel::Gemini25Pro | GeminiModel::Gemini3Pro | GeminiModel::Gemini3Flash
        )
    }

    pub fn max_output_tokens(&self) -> u32 {
        match self {
            GeminiModel::Gemini25Pro => 65536,
            GeminiModel::Gemini25Flash | GeminiModel::Gemini25FlashLite => 8192,
            GeminiModel::Gemini3Pro | GeminiModel::Gemini3Flash => 65536,
            _ => 8192,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ThinkingLevel {
    #[default]
    None,
    Low,
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> Option<&'static str> {
        match self {
            ThinkingLevel::None => None,
            ThinkingLevel::Low => Some("low"),
            ThinkingLevel::Medium => Some("medium"),
            ThinkingLevel::High => Some("high"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub api_key: String,
    pub model: GeminiModel,
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub thinking_level: ThinkingLevel,
}

impl GoogleConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: GeminiModel::default(),
            max_output_tokens: None,
            temperature: None,
            thinking_level: ThinkingLevel::None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("GOOGLE_API_KEY")
            .or_else(|_| env::var("GEMINI_API_KEY"))
            .map_err(|_| kkr_core::error::config("GOOGLE_API_KEY not set"))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: GeminiModel) -> Self {
        self.model = model;
        self
    }

    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    #[deprecated(since = "0.2.0", note = "Use max_output_tokens instead")]
    pub fn max_tokens(self, tokens: u32) -> Self {
        self.max_output_tokens(tokens)
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn thinking_level(mut self, level: ThinkingLevel) -> Self {
        self.thinking_level = level;
        self
    }
}

pub struct Google {
    config: GoogleConfig,
    client: Client,
}

impl Google {
    const BASE_URL: &'static str = "https://generativelanguage.googleapis.com/v1beta";

    pub fn new(config: GoogleConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GoogleConfig::from_env()?))
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<ApiContent>) {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system_instruction = Some(msg.content.clone());
                }
                Role::User => {
                    contents.push(ApiContent {
                        role: "user".to_string(),
                        parts: vec![ApiPart::Text {
                            text: msg.content.clone(),
                        }],
                    });
                }
                Role::Assistant => {
                    let mut parts = Vec::new();

                    if !msg.content.is_empty() {
                        parts.push(ApiPart::Text {
                            text: msg.content.clone(),
                        });
                    }

                    if let Some(ref tool_calls) = msg.tool_calls {
                        for tc in tool_calls {
                            parts.push(ApiPart::FunctionCall {
                                function_call: ApiFunctionCall {
                                    name: tc.name.clone(),
                                    args: tc.arguments.clone(),
                                },
                            });
                        }
                    }

                    contents.push(ApiContent {
                        role: "model".to_string(),
                        parts,
                    });
                }
                Role::Tool => {
                    contents.push(ApiContent {
                        role: "user".to_string(),
                        parts: vec![ApiPart::FunctionResponse {
                            function_response: ApiFunctionResponse {
                                name: msg.name.clone().unwrap_or_default(),
                                response: serde_json::from_str(&msg.content)
                                    .unwrap_or(serde_json::json!({"result": msg.content})),
                            },
                        }],
                    });
                }
            }
        }

        (system_instruction, contents)
    }

    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<ApiTool> {
        let declarations: Vec<ApiFunctionDeclaration> = tools
            .iter()
            .map(|t| ApiFunctionDeclaration {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: serde_json::json!({
                    "type": t.parameters.schema_type,
                    "properties": t.parameters.properties,
                    "required": t.parameters.required
                }),
            })
            .collect();

        vec![ApiTool {
            function_declarations: declarations,
        }]
    }
}

#[async_trait]
impl Provider for Google {
    fn name(&self) -> &str {
        "google"
    }

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let (system_instruction, contents) = self.convert_messages(&messages);

        let max_tokens = self
            .config
            .max_output_tokens
            .unwrap_or_else(|| self.config.model.max_output_tokens());

        let thinking_config = if self.config.model.is_reasoning_model() {
            self.config
                .thinking_level
                .as_str()
                .map(|level| ApiThinkingConfig {
                    thinking_budget: Some(level.to_string()),
                })
        } else {
            None
        };

        let generation_config = ApiGenerationConfig {
            max_output_tokens: Some(max_tokens),
            temperature: self.config.temperature,
            thinking_config,
        };

        let mut request = ApiRequest {
            contents,
            system_instruction: system_instruction.map(|s| ApiSystemInstruction {
                parts: vec![ApiPart::Text { text: s }],
            }),
            generation_config: Some(generation_config),
            tools: None,
        };

        if let Some(ref t) = tools {
            if !t.is_empty() {
                request.tools = Some(self.convert_tools(t));
            }
        }

        let url = format!(
            "{}/models/{}:generateContent",
            Self::BASE_URL,
            self.config.model.as_str()
        );

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::error::provider("google",format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::error::provider("google",format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::error::provider("google",format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body).map_err(|e| {
            kkr_core::error::provider("google",format!("Failed to parse response: {} - {}", e, body))
        })?;

        let candidate = api_response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| kkr_core::error::provider("google", "No candidates in response"))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for part in candidate.content.parts {
            match part {
                ApiPart::Text { text } => {
                    content.push_str(&text);
                }
                ApiPart::FunctionCall { function_call } => {
                    tool_calls.push(ToolCall {
                        id: format!("call_{}", tool_calls.len()),
                        name: function_call.name,
                        arguments: function_call.args,
                    });
                }
                _ => {}
            }
        }

        let finish_reason = match candidate.finish_reason.as_deref() {
            Some("STOP") => FinishReason::Stop,
            Some("MAX_TOKENS") => FinishReason::Length,
            Some("SAFETY") => FinishReason::ContentFilter,
            Some("RECITATION") => FinishReason::ContentFilter,
            _ if !tool_calls.is_empty() => FinishReason::ToolCalls,
            _ => FinishReason::Stop,
        };

        let usage = api_response.usage_metadata.map(|u| Usage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count.unwrap_or(0),
            total_tokens: u.total_token_count,
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

#[derive(Debug, Serialize)]
struct ApiRequest {
    contents: Vec<ApiContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    system_instruction: Option<ApiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    generation_config: Option<ApiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Serialize)]
struct ApiContent {
    role: String,
    parts: Vec<ApiPart>,
}

#[derive(Debug, Serialize)]
struct ApiSystemInstruction {
    parts: Vec<ApiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ApiPart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: ApiFunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: ApiFunctionResponse,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ApiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingConfig")]
    thinking_config: Option<ApiThinkingConfig>,
}

#[derive(Debug, Serialize)]
struct ApiThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingBudget")]
    thinking_budget: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<ApiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct ApiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    candidates: Vec<ApiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<ApiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct ApiCandidate {
    content: ApiCandidateContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiCandidateContent {
    parts: Vec<ApiPart>,
}

#[derive(Debug, Deserialize)]
struct ApiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_str() {
        assert_eq!(GeminiModel::Gemini25Flash.as_str(), "gemini-2.5-flash");
        assert_eq!(GeminiModel::Gemini25Pro.as_str(), "gemini-2.5-pro");
        assert_eq!(GeminiModel::Gemini3Pro.as_str(), "gemini-3-pro");
    }

    #[test]
    fn test_reasoning_models() {
        assert!(GeminiModel::Gemini25Pro.is_reasoning_model());
        assert!(GeminiModel::Gemini3Pro.is_reasoning_model());
        assert!(!GeminiModel::Gemini20Flash.is_reasoning_model());
    }

    #[test]
    fn test_config_builder() {
        let config = GoogleConfig::new("test-key")
            .model(GeminiModel::Gemini25Flash)
            .max_output_tokens(8192)
            .temperature(0.5)
            .thinking_level(ThinkingLevel::High);

        assert_eq!(config.max_output_tokens, Some(8192));
        assert_eq!(config.temperature, Some(0.5));
    }
}
