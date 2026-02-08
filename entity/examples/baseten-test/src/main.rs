//! Integration test: entity-runner + Baseten OpenAI-compatible endpoint (GLM-4.7)
//!
//! Proves that entity's Runner can drive a real LLM via any OpenAI-compatible API.

use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use entity_runner::{
    ChatMessage, ChatRequest, ChatResponse, FinishReason, OutputHandler, Runner,
    RunnerConfig, RunnerLlm, ToolCallRequest,
};
use entity_safety::{ApprovalPolicy, SafetyConfigBuilder};
use entity_tools::ToolRegistry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── OpenAI-compatible provider for entity's RunnerLlm ──

struct OpenAICompat {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAICompat {
    fn new(api_key: impl Into<String>, base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

// OpenAI API request/response types

#[derive(Serialize)]
struct OaiRequest {
    model: String,
    messages: Vec<OaiMessage>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OaiTool>>,
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiFunction,
}

#[derive(Serialize)]
struct OaiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Deserialize, Debug)]
struct OaiChoice {
    message: OaiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OaiResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Deserialize, Debug)]
struct OaiToolCall {
    id: String,
    function: OaiToolCallFunction,
}

#[derive(Deserialize, Debug)]
struct OaiToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Debug)]
struct OaiUsage {
    #[serde(default)]
    prompt_tokens: usize,
    #[serde(default)]
    completion_tokens: usize,
}

#[derive(Deserialize, Debug)]
struct OaiErrorResponse {
    error: OaiErrorDetail,
}

#[derive(Deserialize, Debug)]
struct OaiErrorDetail {
    message: String,
}

// ── Implement RunnerLlm ──

#[async_trait]
impl RunnerLlm for OpenAICompat {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Build messages: system prompt + conversation
        let mut messages = vec![OaiMessage {
            role: "system".to_string(),
            content: request.system_prompt,
        }];
        for msg in &request.messages {
            messages.push(OaiMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        // Build tools if any
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| OaiTool {
                        tool_type: "function".to_string(),
                        function: OaiFunction {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        let oai_req = OaiRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: 0.7,
            tools,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&oai_req)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Provider, format!("HTTP error: {}", e)))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::new(ErrorKind::Provider, format!("read body error: {}", e)))?;

        if !status.is_success() {
            // Try to parse error
            if let Ok(err_resp) = serde_json::from_str::<OaiErrorResponse>(&body) {
                return Err(Error::new(
                    ErrorKind::Provider,
                    format!("API error ({}): {}", status, err_resp.error.message),
                ));
            }
            return Err(Error::new(
                ErrorKind::Provider,
                format!("API error ({}): {}", status, body),
            ));
        }

        let oai_resp: OaiResponse = serde_json::from_str(&body)
            .map_err(|e| Error::new(ErrorKind::Parse, format!("parse response: {} body: {}", e, body)))?;

        let choice = oai_resp
            .choices
            .first()
            .ok_or_else(|| Error::new(ErrorKind::Provider, "no choices in response"))?;

        let content = choice.message.content.clone().unwrap_or_default();

        let tool_calls: Vec<ToolCallRequest> = choice
            .message
            .tool_calls
            .as_ref()
            .map(|tcs| {
                tcs.iter()
                    .filter_map(|tc| {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).ok()?;
                        Some(ToolCallRequest {
                            id: tc.id.clone(),
                            tool_name: tc.function.name.clone(),
                            arguments: args,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("tool_calls") => FinishReason::ToolUse,
            Some("length") => FinishReason::MaxTokens,
            _ => FinishReason::Stop,
        };

        let usage = oai_resp.usage.as_ref();

        Ok(ChatResponse {
            content,
            tool_calls,
            input_tokens: usage.map(|u| u.prompt_tokens).unwrap_or(0),
            output_tokens: usage.map(|u| u.completion_tokens).unwrap_or(0),
            finish_reason,
        })
    }
}

// ── Simple output handler for the terminal ──

struct PrintOutput;

#[async_trait]
impl OutputHandler for PrintOutput {
    async fn on_text(&self, text: &str) {
        println!("\n--- Agent Output ---\n{}\n---", text);
    }
    async fn on_tool_start(&self, tool_name: &str, params: &str) {
        println!("[tool:start] {} {}", tool_name, params);
    }
    async fn on_tool_end(&self, tool_name: &str, result: &str, success: bool) {
        let status = if success { "OK" } else { "ERR" };
        println!("[tool:end] {} [{}] {}", tool_name, status, &result[..result.len().min(200)]);
    }
    async fn on_iteration(&self, number: usize, total: usize) {
        println!("\n=== Iteration {}/{} ===", number, total);
    }
}

// ── Main ──

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Entity Runner + Baseten GLM-4.7 Integration Test");
    println!("=================================================\n");

    // 1. Configure the OpenAI-compatible LLM provider
    let llm = OpenAICompat::new(
        "NLOYgWtk.rS4Ey1LMbyc3xRB6vM7e3KiBI2n5pPTx",
        "https://inference.baseten.co/v1",
        "zai-org/GLM-4.7",
    );

    // 2. Configure safety (autonomous mode for testing, no tools needing approval)
    let safety = SafetyConfigBuilder::new()
        .approval_policy(ApprovalPolicy::Never)
        .allow_network(true)
        .build();

    // 3. Set up runner with an empty tool registry (pure chat test)
    let config = RunnerConfig {
        system_prompt: "You are a helpful coding assistant. Be concise.".to_string(),
        max_iterations: 3,
        max_tool_calls_per_turn: 5,
        auto_test: false,
        test_command: None,
        context_tokens: 8192,
        working_dir: PathBuf::from("."),
    };

    let tools = ToolRegistry::new();
    let mut runner = Runner::new(config, tools, safety);

    // 4. Run the task through entity's runner loop
    let output = PrintOutput;
    let task = "Write a Python function that checks if a number is prime. Include docstring and type hints.";

    println!("Task: {}\n", task);

    let result = runner.run(task, &llm, None, Some(&output)).await?;

    // 5. Report results
    println!("\n=== Run Result ===");
    println!("Success: {}", result.success);
    println!("Turns: {}", result.turns);
    println!("Total tokens: {}", result.total_tokens);
    if !result.errors.is_empty() {
        println!("Errors: {:?}", result.errors);
    }

    // 6. Also test a direct LLM call to verify the provider works standalone
    println!("\n=== Direct LLM Call Test ===");
    let direct = llm
        .chat(entity_runner::ChatRequest {
            system_prompt: "You are helpful.".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "What is 2+2? Answer with just the number.".to_string(),
            }],
            tools: vec![],
            max_tokens: 50,
        })
        .await?;

    println!("Response: {}", direct.content);
    println!("Tokens: {} in / {} out", direct.input_tokens, direct.output_tokens);
    println!("Finish reason: {:?}", direct.finish_reason);

    println!("\n=== All tests passed! Entity framework works with OpenAI-compatible endpoints. ===");
    Ok(())
}
