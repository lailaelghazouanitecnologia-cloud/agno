use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expiry: Option<i64>,
}

impl GmailCredentials {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            access_token: None,
            refresh_token: None,
            token_expiry: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let client_id = env::var("GMAIL_CLIENT_ID")
            .map_err(|_| kkr_core::Error::Config("GMAIL_CLIENT_ID not set".into()))?;
        let client_secret = env::var("GMAIL_CLIENT_SECRET")
            .map_err(|_| kkr_core::Error::Config("GMAIL_CLIENT_SECRET not set".into()))?;

        let mut creds = Self::new(client_id, client_secret);
        creds.access_token = env::var("GMAIL_ACCESS_TOKEN").ok();
        creds.refresh_token = env::var("GMAIL_REFRESH_TOKEN").ok();

        Ok(creds)
    }

    pub fn with_access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct GmailClient {
    client: Client,
    credentials: Arc<RwLock<GmailCredentials>>,
    user_id: String,
}

impl GmailClient {
    pub fn new(credentials: GmailCredentials) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            credentials: Arc::new(RwLock::new(credentials)),
            user_id: "me".to_string(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailCredentials::from_env()?))
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = user_id.into();
        self
    }

    async fn get_access_token(&self) -> Result<String> {
        let creds = self.credentials.read().await;

        if let Some(ref token) = creds.access_token {
            if let Some(expiry) = creds.token_expiry {
                let now = chrono::Utc::now().timestamp();
                if now < expiry - 60 {
                    return Ok(token.clone());
                }
            } else {
                return Ok(token.clone());
            }
        }

        drop(creds);
        self.refresh_access_token().await
    }

    async fn refresh_access_token(&self) -> Result<String> {
        let creds = self.credentials.read().await;
        let refresh_token = creds.refresh_token.clone().ok_or_else(|| {
            kkr_core::Error::Tool("No refresh token available".to_string())
        })?;
        let client_id = creds.client_id.clone();
        let client_secret = creds.client_secret.clone();
        drop(creds);

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_token),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ])
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Token refresh failed: {}", e)))?;

        let token_data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse token response: {}", e)))?;

        let access_token = token_data["access_token"]
            .as_str()
            .ok_or_else(|| kkr_core::Error::Tool("No access token in response".to_string()))?
            .to_string();

        let expires_in = token_data["expires_in"].as_i64().unwrap_or(3600);

        let mut creds = self.credentials.write().await;
        creds.access_token = Some(access_token.clone());
        creds.token_expiry = Some(chrono::Utc::now().timestamp() + expires_in);

        Ok(access_token)
    }

    pub async fn list_messages(
        &self,
        query: Option<&str>,
        max_results: usize,
        label_ids: Option<Vec<&str>>,
    ) -> Result<Vec<GmailMessage>> {
        let token = self.get_access_token().await?;

        let mut url = format!(
            "{}/users/{}/messages?maxResults={}",
            GMAIL_API_BASE, self.user_id, max_results
        );

        if let Some(q) = query {
            url.push_str(&format!("&q={}", urlencoding::encode(q)));
        }

        if let Some(labels) = label_ids {
            for label in labels {
                url.push_str(&format!("&labelIds={}", label));
            }
        }

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to list messages: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        let message_ids: Vec<String> = data["messages"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();

        let mut messages = Vec::new();
        for id in message_ids.iter().take(max_results) {
            if let Ok(msg) = self.get_message(id).await {
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    pub async fn get_message(&self, message_id: &str) -> Result<GmailMessage> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/messages/{}?format=full",
            GMAIL_API_BASE, self.user_id, message_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to get message: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse message: {}", e)))?;

        parse_gmail_message(data)
    }

    pub async fn send_message(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        html: bool,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        let content_type = if html { "text/html" } else { "text/plain" };
        let raw_message = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: {}; charset=utf-8\r\n\r\n{}",
            to, subject, content_type, body
        );

        let encoded = URL_SAFE.encode(raw_message.as_bytes());

        let url = format!("{}/users/{}/messages/send", GMAIL_API_BASE, self.user_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({ "raw": encoded }))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to send message: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        data["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| kkr_core::Error::Tool("No message ID in response".to_string()))
    }

    pub async fn create_draft(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        html: bool,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        let content_type = if html { "text/html" } else { "text/plain" };
        let raw_message = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: {}; charset=utf-8\r\n\r\n{}",
            to, subject, content_type, body
        );

        let encoded = URL_SAFE.encode(raw_message.as_bytes());

        let url = format!("{}/users/{}/drafts", GMAIL_API_BASE, self.user_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "message": { "raw": encoded }
            }))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to create draft: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        data["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| kkr_core::Error::Tool("No draft ID in response".to_string()))
    }

    pub async fn list_labels(&self) -> Result<Vec<GmailLabel>> {
        let token = self.get_access_token().await?;

        let url = format!("{}/users/{}/labels", GMAIL_API_BASE, self.user_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to list labels: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        let labels: Vec<GmailLabel> = data["labels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|l| {
                Some(GmailLabel {
                    id: l["id"].as_str()?.to_string(),
                    name: l["name"].as_str()?.to_string(),
                    label_type: l["type"].as_str().unwrap_or("user").to_string(),
                })
            })
            .collect();

        Ok(labels)
    }

    pub async fn modify_labels(
        &self,
        message_id: &str,
        add_labels: Vec<&str>,
        remove_labels: Vec<&str>,
    ) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/messages/{}/modify",
            GMAIL_API_BASE, self.user_id, message_id
        );

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "addLabelIds": add_labels,
                "removeLabelIds": remove_labels
            }))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to modify labels: {}", e)))?;

        Ok(())
    }

    pub async fn trash_message(&self, message_id: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/messages/{}/trash",
            GMAIL_API_BASE, self.user_id, message_id
        );

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to trash message: {}", e)))?;

        Ok(())
    }

    /// Reply to an existing message in the same thread
    pub async fn reply_message(
        &self,
        message_id: &str,
        thread_id: &str,
        to: &str,
        body: &str,
        html: bool,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        // Get original message to extract subject and references
        let original = self.get_message(message_id).await?;
        let subject = if original.subject.starts_with("Re: ") {
            original.subject.clone()
        } else {
            format!("Re: {}", original.subject)
        };

        let content_type = if html { "text/html" } else { "text/plain" };
        let raw_message = format!(
            "To: {}\r\nSubject: {}\r\nIn-Reply-To: {}\r\nReferences: {}\r\nContent-Type: {}; charset=utf-8\r\n\r\n{}",
            to, subject, message_id, message_id, content_type, body
        );

        let encoded = URL_SAFE.encode(raw_message.as_bytes());

        let url = format!("{}/users/{}/messages/send", GMAIL_API_BASE, self.user_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "raw": encoded,
                "threadId": thread_id
            }))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to send reply: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        data["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| kkr_core::Error::Tool("No message ID in response".to_string()))
    }

    /// Get all messages in a thread
    pub async fn get_thread(&self, thread_id: &str) -> Result<Vec<GmailMessage>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/threads/{}?format=full",
            GMAIL_API_BASE, self.user_id, thread_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to get thread: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse thread: {}", e)))?;

        let messages = data["messages"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| parse_gmail_message(m.clone()).ok())
            .collect();

        Ok(messages)
    }

    /// Create a new label
    pub async fn create_label(&self, name: &str) -> Result<GmailLabel> {
        let token = self.get_access_token().await?;

        let url = format!("{}/users/{}/labels", GMAIL_API_BASE, self.user_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "name": name,
                "labelListVisibility": "labelShow",
                "messageListVisibility": "show"
            }))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to create label: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        Ok(GmailLabel {
            id: data["id"].as_str().unwrap_or("").to_string(),
            name: data["name"].as_str().unwrap_or("").to_string(),
            label_type: data["type"].as_str().unwrap_or("user").to_string(),
        })
    }

    /// Delete a label (user-created labels only)
    pub async fn delete_label(&self, label_id: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/labels/{}",
            GMAIL_API_BASE, self.user_id, label_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete label: {}", e)))?;

        if !response.status().is_success() {
            return Err(kkr_core::Error::Tool(format!(
                "Failed to delete label: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Download an attachment
    pub async fn get_attachment(&self, message_id: &str, attachment_id: &str) -> Result<Vec<u8>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/users/{}/messages/{}/attachments/{}",
            GMAIL_API_BASE, self.user_id, message_id, attachment_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to get attachment: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse attachment: {}", e)))?;

        let encoded = data["data"]
            .as_str()
            .ok_or_else(|| kkr_core::Error::Tool("No attachment data".to_string()))?;

        URL_SAFE
            .decode(encoded)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to decode attachment: {}", e)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub to: String,
    pub cc: Option<String>,
    pub subject: String,
    pub date: String,
    pub snippet: String,
    pub body: String,
    pub labels: Vec<String>,
    pub attachments: Vec<GmailAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailAttachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailLabel {
    pub id: String,
    pub name: String,
    pub label_type: String,
}

fn parse_gmail_message(data: Value) -> Result<GmailMessage> {
    let headers = data["payload"]["headers"].as_array();

    let get_header = |name: &str| -> String {
        headers
            .and_then(|h| {
                h.iter()
                    .find(|hdr| hdr["name"].as_str().map(|s| s.eq_ignore_ascii_case(name)) == Some(true))
                    .and_then(|hdr| hdr["value"].as_str())
            })
            .unwrap_or("")
            .to_string()
    };

    let body = extract_body(&data["payload"]);
    let attachments = extract_attachments(&data["payload"]);

    let cc = get_header("Cc");
    let cc = if cc.is_empty() { None } else { Some(cc) };

    Ok(GmailMessage {
        id: data["id"].as_str().unwrap_or("").to_string(),
        thread_id: data["threadId"].as_str().unwrap_or("").to_string(),
        from: get_header("From"),
        to: get_header("To"),
        cc,
        subject: get_header("Subject"),
        date: get_header("Date"),
        snippet: data["snippet"].as_str().unwrap_or("").to_string(),
        body,
        labels: data["labelIds"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        attachments,
    })
}

fn extract_attachments(payload: &Value) -> Vec<GmailAttachment> {
    let mut attachments = Vec::new();

    if let Some(parts) = payload["parts"].as_array() {
        for part in parts {
            let filename = part["filename"].as_str().unwrap_or("");
            if !filename.is_empty() {
                if let Some(body) = part["body"].as_object() {
                    attachments.push(GmailAttachment {
                        id: body.get("attachmentId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        filename: filename.to_string(),
                        mime_type: part["mimeType"].as_str().unwrap_or("").to_string(),
                        size: body.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                    });
                }
            }
            // Recurse into nested parts
            let nested = extract_attachments(part);
            attachments.extend(nested);
        }
    }

    attachments
}

fn extract_body(payload: &Value) -> String {
    if let Some(body_data) = payload["body"]["data"].as_str() {
        if let Ok(decoded) = URL_SAFE.decode(body_data) {
            if let Ok(text) = String::from_utf8(decoded) {
                return text;
            }
        }
    }

    if let Some(parts) = payload["parts"].as_array() {
        for part in parts {
            let mime_type = part["mimeType"].as_str().unwrap_or("");
            if mime_type == "text/plain" || mime_type == "text/html" {
                if let Some(body_data) = part["body"]["data"].as_str() {
                    if let Ok(decoded) = URL_SAFE.decode(body_data) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            return text;
                        }
                    }
                }
            }
            let nested = extract_body(part);
            if !nested.is_empty() {
                return nested;
            }
        }
    }

    String::new()
}

// ============================================================================
// Gmail Tools
// ============================================================================

pub struct GmailReadTool {
    client: GmailClient,
}

impl GmailReadTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct GmailReadParams {
    #[serde(default)]
    query: Option<String>,
    #[serde(default = "default_max_results")]
    max_results: usize,
    #[serde(default)]
    labels: Option<Vec<String>>,
}

fn default_max_results() -> usize {
    10
}

#[async_trait]
impl Tool for GmailReadTool {
    fn name(&self) -> &str {
        "gmail_read"
    }

    fn description(&self) -> &str {
        "Read emails from Gmail inbox. Supports Gmail search queries like 'is:unread', 'from:user@example.com', 'has:attachment', etc."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Gmail search query (e.g., 'is:unread', 'from:user@example.com', 'newer_than:1d')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of emails to return (default: 10)",
                    "minimum": 1,
                    "maximum": 100
                },
                "labels": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by label IDs (e.g., 'INBOX', 'UNREAD', 'STARRED')"
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "read", "inbox", "google"])
            .with_read_only(true)
            .with_priority(80)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
            .with_example(ToolExample {
                description: "Read unread emails".to_string(),
                input: json!({"query": "is:unread", "max_results": 5}),
                output: Some(json!({"messages": []})),
            })
            .with_example(ToolExample {
                description: "Read emails from specific sender".to_string(),
                input: json!({"query": "from:boss@company.com", "max_results": 10}),
                output: Some(json!({"messages": []})),
            })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailReadParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let labels: Option<Vec<&str>> = params
            .labels
            .as_ref()
            .map(|l| l.iter().map(|s| s.as_str()).collect());

        let messages = self
            .client
            .list_messages(params.query.as_deref(), params.max_results, labels)
            .await?;

        Ok(json!({
            "count": messages.len(),
            "messages": messages
        }))
    }
}

pub struct GmailSendTool {
    client: GmailClient,
}

impl GmailSendTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct GmailSendParams {
    to: String,
    subject: String,
    body: String,
    #[serde(default)]
    html: bool,
}

#[async_trait]
impl Tool for GmailSendTool {
    fn name(&self) -> &str {
        "gmail_send"
    }

    fn description(&self) -> &str {
        "Send an email via Gmail API. Supports plain text and HTML content."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "to": {
                    "type": "string",
                    "description": "Recipient email address"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject line"
                },
                "body": {
                    "type": "string",
                    "description": "Email body content"
                },
                "html": {
                    "type": "boolean",
                    "description": "Whether body is HTML (default: false)"
                }
            }),
            required: vec!["to".to_string(), "subject".to_string(), "body".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "send", "google"])
            .with_read_only(false)
            .with_priority(80)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailSendParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let message_id = self
            .client
            .send_message(&params.to, &params.subject, &params.body, params.html)
            .await?;

        Ok(json!({
            "success": true,
            "message_id": message_id,
            "to": params.to
        }))
    }
}

pub struct GmailDraftTool {
    client: GmailClient,
}

impl GmailDraftTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailDraftTool {
    fn name(&self) -> &str {
        "gmail_draft"
    }

    fn description(&self) -> &str {
        "Create a draft email in Gmail. The draft can be edited and sent later from Gmail."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "to": {
                    "type": "string",
                    "description": "Recipient email address"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject line"
                },
                "body": {
                    "type": "string",
                    "description": "Email body content"
                },
                "html": {
                    "type": "boolean",
                    "description": "Whether body is HTML (default: false)"
                }
            }),
            required: vec!["to".to_string(), "subject".to_string(), "body".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "draft", "google"])
            .with_read_only(false)
            .with_priority(75)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailSendParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let draft_id = self
            .client
            .create_draft(&params.to, &params.subject, &params.body, params.html)
            .await?;

        Ok(json!({
            "success": true,
            "draft_id": draft_id,
            "to": params.to
        }))
    }
}

pub struct GmailLabelsTool {
    client: GmailClient,
}

impl GmailLabelsTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailLabelsTool {
    fn name(&self) -> &str {
        "gmail_labels"
    }

    fn description(&self) -> &str {
        "List all labels in the Gmail account. Labels can be used to filter and organize emails."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({}),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "labels", "google"])
            .with_read_only(true)
            .with_priority(60)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> Result<Value> {
        let labels = self.client.list_labels().await?;

        Ok(json!({
            "count": labels.len(),
            "labels": labels
        }))
    }
}

pub struct GmailModifyLabelsTool {
    client: GmailClient,
}

impl GmailModifyLabelsTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct GmailModifyLabelsParams {
    message_id: String,
    #[serde(default)]
    add_labels: Vec<String>,
    #[serde(default)]
    remove_labels: Vec<String>,
}

#[async_trait]
impl Tool for GmailModifyLabelsTool {
    fn name(&self) -> &str {
        "gmail_modify_labels"
    }

    fn description(&self) -> &str {
        "Add or remove labels from an email. Use 'UNREAD' to mark as read/unread, 'STARRED' to star/unstar."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "message_id": {
                    "type": "string",
                    "description": "The ID of the email message"
                },
                "add_labels": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Labels to add (e.g., 'STARRED', 'IMPORTANT')"
                },
                "remove_labels": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Labels to remove (e.g., 'UNREAD' to mark as read)"
                }
            }),
            required: vec!["message_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "labels", "organize", "google"])
            .with_read_only(false)
            .with_priority(65)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailModifyLabelsParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let add_labels: Vec<&str> = params.add_labels.iter().map(|s| s.as_str()).collect();
        let remove_labels: Vec<&str> = params.remove_labels.iter().map(|s| s.as_str()).collect();

        self.client
            .modify_labels(&params.message_id, add_labels, remove_labels)
            .await?;

        Ok(json!({
            "success": true,
            "message_id": params.message_id
        }))
    }
}

pub struct GmailTrashTool {
    client: GmailClient,
}

impl GmailTrashTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct GmailTrashParams {
    message_id: String,
}

#[async_trait]
impl Tool for GmailTrashTool {
    fn name(&self) -> &str {
        "gmail_trash"
    }

    fn description(&self) -> &str {
        "Move an email to trash. The email can be recovered from trash within 30 days."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "message_id": {
                    "type": "string",
                    "description": "The ID of the email message to trash"
                }
            }),
            required: vec!["message_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "delete", "trash", "google"])
            .with_read_only(false)
            .with_priority(60)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailTrashParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        self.client.trash_message(&params.message_id).await?;

        Ok(json!({
            "success": true,
            "message_id": params.message_id,
            "action": "moved_to_trash"
        }))
    }
}

// ============================================================================
// Gmail Reply Tool
// ============================================================================

pub struct GmailReplyTool {
    client: GmailClient,
}

impl GmailReplyTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct GmailReplyParams {
    message_id: String,
    thread_id: String,
    body: String,
    #[serde(default)]
    html: bool,
}

#[async_trait]
impl Tool for GmailReplyTool {
    fn name(&self) -> &str {
        "gmail_reply"
    }

    fn description(&self) -> &str {
        "Reply to an existing email in the same thread. Automatically adds Re: prefix and proper threading headers."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to reply to"
                },
                "thread_id": {
                    "type": "string",
                    "description": "The thread ID for proper threading"
                },
                "body": {
                    "type": "string",
                    "description": "Reply body content"
                },
                "html": {
                    "type": "boolean",
                    "description": "Whether body is HTML (default: false)"
                }
            }),
            required: vec!["message_id".to_string(), "thread_id".to_string(), "body".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "reply", "thread", "google"])
            .with_read_only(false)
            .with_priority(80)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: GmailReplyParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        // Get original message to find the sender
        let original = self.client.get_message(&params.message_id).await?;

        let message_id = self
            .client
            .reply_message(
                &params.message_id,
                &params.thread_id,
                &original.from,
                &params.body,
                params.html,
            )
            .await?;

        Ok(json!({
            "success": true,
            "message_id": message_id,
            "thread_id": params.thread_id,
            "replied_to": original.from
        }))
    }
}

// ============================================================================
// Gmail Thread Tool
// ============================================================================

pub struct GmailThreadTool {
    client: GmailClient,
}

impl GmailThreadTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailThreadTool {
    fn name(&self) -> &str {
        "gmail_thread"
    }

    fn description(&self) -> &str {
        "Get all messages in an email thread/conversation."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "thread_id": {
                    "type": "string",
                    "description": "The thread ID to retrieve"
                }
            }),
            required: vec!["thread_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "thread", "conversation", "google"])
            .with_read_only(true)
            .with_priority(75)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let thread_id = params
            .get("thread_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing thread_id".to_string()))?;

        let messages = self.client.get_thread(thread_id).await?;

        Ok(json!({
            "thread_id": thread_id,
            "message_count": messages.len(),
            "messages": messages
        }))
    }
}

// ============================================================================
// Gmail Create Label Tool
// ============================================================================

pub struct GmailCreateLabelTool {
    client: GmailClient,
}

impl GmailCreateLabelTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailCreateLabelTool {
    fn name(&self) -> &str {
        "gmail_create_label"
    }

    fn description(&self) -> &str {
        "Create a new custom label in Gmail."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "name": {
                    "type": "string",
                    "description": "Name for the new label"
                }
            }),
            required: vec!["name".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "label", "create", "google"])
            .with_read_only(false)
            .with_priority(60)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing name".to_string()))?;

        let label = self.client.create_label(name).await?;

        Ok(json!({
            "success": true,
            "label": label
        }))
    }
}

// ============================================================================
// Gmail Delete Label Tool
// ============================================================================

pub struct GmailDeleteLabelTool {
    client: GmailClient,
}

impl GmailDeleteLabelTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailDeleteLabelTool {
    fn name(&self) -> &str {
        "gmail_delete_label"
    }

    fn description(&self) -> &str {
        "Delete a custom label from Gmail. System labels cannot be deleted."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "label_id": {
                    "type": "string",
                    "description": "The ID of the label to delete"
                }
            }),
            required: vec!["label_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "label", "delete", "google"])
            .with_read_only(false)
            .with_priority(55)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let label_id = params
            .get("label_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing label_id".to_string()))?;

        self.client.delete_label(label_id).await?;

        Ok(json!({
            "success": true,
            "deleted_label_id": label_id
        }))
    }
}

// ============================================================================
// Gmail Attachment Tool
// ============================================================================

pub struct GmailAttachmentTool {
    client: GmailClient,
}

impl GmailAttachmentTool {
    pub fn new(client: GmailClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GmailClient::from_env()?))
    }
}

#[async_trait]
impl Tool for GmailAttachmentTool {
    fn name(&self) -> &str {
        "gmail_get_attachment"
    }

    fn description(&self) -> &str {
        "Download an attachment from an email. Returns base64-encoded content."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message containing the attachment"
                },
                "attachment_id": {
                    "type": "string",
                    "description": "The ID of the attachment to download"
                }
            }),
            required: vec!["message_id".to_string(), "attachment_id".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["gmail", "email", "attachment", "download", "google"])
            .with_read_only(true)
            .with_priority(70)
            .with_requires(vec!["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET", "GMAIL_ACCESS_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let message_id = params
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing message_id".to_string()))?;

        let attachment_id = params
            .get("attachment_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing attachment_id".to_string()))?;

        let data = self.client.get_attachment(message_id, attachment_id).await?;

        use base64::{engine::general_purpose::STANDARD, Engine};
        let encoded = STANDARD.encode(&data);

        Ok(json!({
            "success": true,
            "message_id": message_id,
            "attachment_id": attachment_id,
            "size": data.len(),
            "data_base64": encoded
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_credentials_from_env() {
        let creds = GmailCredentials::new("client_id", "client_secret")
            .with_access_token("token")
            .with_refresh_token("refresh");

        assert_eq!(creds.client_id, "client_id");
        assert!(creds.access_token.is_some());
    }

    #[test]
    fn test_gmail_read_tool_metadata() {
        let creds = GmailCredentials::new("id", "secret").with_access_token("token");
        let tool = GmailReadTool::new(GmailClient::new(creds));

        assert_eq!(tool.name(), "gmail_read");
        assert!(tool.metadata().read_only);
    }
}
