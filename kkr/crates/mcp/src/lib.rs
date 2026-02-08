use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use kkr_core::Result;

const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

impl Default for RequestId {
    fn default() -> Self {
        RequestId::String(Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

impl McpContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
        }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "image".to_string(),
            text: None,
            data: Some(data.into()),
            mime_type: Some(mime_type.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<McpContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: McpContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RootsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

#[async_trait]
pub trait McpToolHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn call(&self, arguments: serde_json::Value) -> Result<ToolCallResult>;
}

#[async_trait]
pub trait McpResourceHandler: Send + Sync {
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn mime_type(&self) -> Option<&str>;
    async fn read(&self) -> Result<ResourceContent>;
}

#[async_trait]
pub trait McpPromptHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn arguments(&self) -> Option<Vec<McpPromptArgument>>;
    async fn get(&self, arguments: HashMap<String, String>) -> Result<Vec<PromptMessage>>;
}

pub struct McpServer {
    name: String,
    version: String,
    tools: RwLock<Vec<Arc<dyn McpToolHandler>>>,
    resources: RwLock<Vec<Arc<dyn McpResourceHandler>>>,
    prompts: RwLock<Vec<Arc<dyn McpPromptHandler>>>,
}

impl McpServer {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: RwLock::new(Vec::new()),
            resources: RwLock::new(Vec::new()),
            prompts: RwLock::new(Vec::new()),
        }
    }

    pub async fn add_tool(&self, handler: Arc<dyn McpToolHandler>) {
        self.tools.write().await.push(handler);
    }

    pub async fn add_resource(&self, handler: Arc<dyn McpResourceHandler>) {
        self.resources.write().await.push(handler);
    }

    pub async fn add_prompt(&self, handler: Arc<dyn McpPromptHandler>) {
        self.prompts.write().await.push(handler);
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_call_tool(request.params).await,
            "resources/list" => self.handle_list_resources().await,
            "resources/read" => self.handle_read_resource(request.params).await,
            "prompts/list" => self.handle_list_prompts().await,
            "prompts/get" => self.handle_get_prompt(request.params).await,
            _ => Err(kkr_core::error::other(format!(
                "Unknown method: {}",
                request.method
            ))),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(
        &self,
        _params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let tools = self.tools.read().await;
        let resources = self.resources.read().await;
        let prompts = self.prompts.read().await;

        let result = InitializeResult {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: if tools.is_empty() {
                    None
                } else {
                    Some(ToolsCapability::default())
                },
                resources: if resources.is_empty() {
                    None
                } else {
                    Some(ResourcesCapability::default())
                },
                prompts: if prompts.is_empty() {
                    None
                } else {
                    Some(PromptsCapability::default())
                },
            },
            server_info: Implementation {
                name: self.name.clone(),
                version: self.version.clone(),
            },
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_tools(&self) -> Result<serde_json::Value> {
        let tools = self.tools.read().await;
        let tool_list: Vec<McpTool> = tools
            .iter()
            .map(|t| McpTool {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect();

        Ok(serde_json::json!({ "tools": tool_list }))
    }

    async fn handle_call_tool(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| kkr_core::error::other("Missing params"))?;
        let name = params["name"]
            .as_str()
            .ok_or_else(|| kkr_core::error::other("Missing tool name"))?;
        let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

        let tools = self.tools.read().await;
        let tool = tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| kkr_core::error::other(format!("Tool not found: {}", name)))?;

        let result = tool.call(arguments).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_resources(&self) -> Result<serde_json::Value> {
        let resources = self.resources.read().await;
        let resource_list: Vec<McpResource> = resources
            .iter()
            .map(|r| McpResource {
                uri: r.uri().to_string(),
                name: r.name().to_string(),
                description: r.description().map(|s| s.to_string()),
                mime_type: r.mime_type().map(|s| s.to_string()),
            })
            .collect();

        Ok(serde_json::json!({ "resources": resource_list }))
    }

    async fn handle_read_resource(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| kkr_core::error::other("Missing params"))?;
        let uri = params["uri"]
            .as_str()
            .ok_or_else(|| kkr_core::error::other("Missing resource uri"))?;

        let resources = self.resources.read().await;
        let resource = resources
            .iter()
            .find(|r| r.uri() == uri)
            .ok_or_else(|| kkr_core::error::other(format!("Resource not found: {}", uri)))?;

        let content = resource.read().await?;
        Ok(serde_json::json!({ "contents": [content] }))
    }

    async fn handle_list_prompts(&self) -> Result<serde_json::Value> {
        let prompts = self.prompts.read().await;
        let prompt_list: Vec<McpPrompt> = prompts
            .iter()
            .map(|p| McpPrompt {
                name: p.name().to_string(),
                description: p.description().map(|s| s.to_string()),
                arguments: p.arguments(),
            })
            .collect();

        Ok(serde_json::json!({ "prompts": prompt_list }))
    }

    async fn handle_get_prompt(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| kkr_core::error::other("Missing params"))?;
        let name = params["name"]
            .as_str()
            .ok_or_else(|| kkr_core::error::other("Missing prompt name"))?;

        let arguments: HashMap<String, String> = params
            .get("arguments")
            .and_then(|a| serde_json::from_value(a.clone()).ok())
            .unwrap_or_default();

        let prompts = self.prompts.read().await;
        let prompt = prompts
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| kkr_core::error::other(format!("Prompt not found: {}", name)))?;

        let messages = prompt.get(arguments).await?;
        Ok(serde_json::json!({ "messages": messages }))
    }

    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = serde_json::json!({
                        "jsonrpc": JSONRPC_VERSION,
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e)
                        }
                    });
                    let response_str = serde_json::to_string(&error_response)?;
                    stdout.write_all(response_str.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let response = self.handle_request(request).await;
            let response_str = serde_json::to_string(&response)?;
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }
}

pub struct McpClient {
    process: Mutex<Child>,
    pending_requests: Arc<Mutex<HashMap<String, mpsc::Sender<JsonRpcResponse>>>>,
    request_id: Mutex<i64>,
}

impl McpClient {
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| kkr_core::error::other(format!("Failed to spawn MCP server: {}", e)))?;

        let stdout = child.stdout.take().unwrap();
        let pending: Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<JsonRpcResponse>>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    let id_str = match &response.id {
                        RequestId::String(s) => s.clone(),
                        RequestId::Number(n) => n.to_string(),
                    };

                    let mut pending = pending_clone.lock().await;
                    if let Some(sender) = pending.remove(&id_str) {
                        let _ = sender.send(response).await;
                    }
                }
            }
        });

        Ok(Self {
            process: Mutex::new(child),
            pending_requests: pending,
            request_id: Mutex::new(0),
        })
    }

    async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };

        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(id),
            method: method.to_string(),
            params,
        };

        let (tx, mut rx) = mpsc::channel(1);
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id.to_string(), tx);
        }

        let request_str = serde_json::to_string(&request)?;
        {
            let mut process = self.process.lock().await;
            if let Some(stdin) = process.stdin.as_mut() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(request_str.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
            }
        }

        let response = rx
            .recv()
            .await
            .ok_or_else(|| kkr_core::error::other("No response received"))?;

        if let Some(error) = response.error {
            return Err(kkr_core::error::other(format!(
                "MCP error {}: {}",
                error.code, error.message
            )));
        }

        response
            .result
            .ok_or_else(|| kkr_core::error::other("Empty response"))
    }

    pub async fn initialize(&self, client_name: &str, client_version: &str) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability::default()),
                sampling: None,
            },
            client_info: Implementation {
                name: client_name.to_string(),
                version: client_version.to_string(),
            },
        };

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;

        self.send_request("notifications/initialized", None).await.ok();

        Ok(serde_json::from_value(result)?)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.send_request("tools/list", None).await?;
        let tools: Vec<McpTool> = serde_json::from_value(result["tools"].clone())?;
        Ok(tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params)).await?;
        Ok(serde_json::from_value(result)?)
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let result = self.send_request("resources/list", None).await?;
        let resources: Vec<McpResource> = serde_json::from_value(result["resources"].clone())?;
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContent>> {
        let params = serde_json::json!({ "uri": uri });
        let result = self.send_request("resources/read", Some(params)).await?;
        let contents: Vec<ResourceContent> = serde_json::from_value(result["contents"].clone())?;
        Ok(contents)
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        let result = self.send_request("prompts/list", None).await?;
        let prompts: Vec<McpPrompt> = serde_json::from_value(result["prompts"].clone())?;
        Ok(prompts)
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: HashMap<String, String>,
    ) -> Result<Vec<PromptMessage>> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("prompts/get", Some(params)).await?;
        let messages: Vec<PromptMessage> = serde_json::from_value(result["messages"].clone())?;
        Ok(messages)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let mut process = self.process.lock().await;
        process.kill().await.ok();
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Ok(mut process) = self.process.try_lock() {
            let _ = process.start_kill();
        }
    }
}

