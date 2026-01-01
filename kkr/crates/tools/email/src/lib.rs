use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: Option<String>,
}

impl SmtpConfig {
    pub fn new(host: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 587,
            username: username.into(),
            password: password.into(),
            from_email: String::new(),
            from_name: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let host = env::var("SMTP_HOST")
            .map_err(|_| kkr_core::Error::Config("SMTP_HOST not set".into()))?;
        let username = env::var("SMTP_USERNAME")
            .map_err(|_| kkr_core::Error::Config("SMTP_USERNAME not set".into()))?;
        let password = env::var("SMTP_PASSWORD")
            .map_err(|_| kkr_core::Error::Config("SMTP_PASSWORD not set".into()))?;
        let from_email = env::var("SMTP_FROM_EMAIL")
            .map_err(|_| kkr_core::Error::Config("SMTP_FROM_EMAIL not set".into()))?;

        Ok(Self {
            host,
            port: env::var("SMTP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(587),
            username,
            password,
            from_email,
            from_name: env::var("SMTP_FROM_NAME").ok(),
        })
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn from_email(mut self, email: impl Into<String>) -> Self {
        self.from_email = email.into();
        self
    }

    pub fn from_name(mut self, name: impl Into<String>) -> Self {
        self.from_name = Some(name.into());
        self
    }

    pub fn gmail(username: impl Into<String>, app_password: impl Into<String>) -> Self {
        Self::new("smtp.gmail.com", username, app_password).port(587)
    }

    pub fn outlook(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::new("smtp-mail.outlook.com", username, password).port(587)
    }
}

pub struct SmtpEmailTool {
    config: SmtpConfig,
}

impl SmtpEmailTool {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(SmtpConfig::from_env()?))
    }
}

#[derive(Debug, Deserialize)]
struct SendEmailParams {
    to: String,
    subject: String,
    body: String,
    #[serde(default)]
    html: bool,
    #[serde(default)]
    cc: Option<Vec<String>>,
    #[serde(default)]
    bcc: Option<Vec<String>>,
}

#[async_trait]
impl Tool for SmtpEmailTool {
    fn name(&self) -> &str {
        "smtp_send_email"
    }

    fn description(&self) -> &str {
        "Send an email via SMTP. Supports HTML content, CC, and BCC recipients."
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
                },
                "cc": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "CC recipients"
                },
                "bcc": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "BCC recipients"
                }
            }),
            required: vec!["to".to_string(), "subject".to_string(), "body".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["email", "smtp", "send", "communication"])
            .with_read_only(false)
            .with_priority(70)
            .with_requires(vec!["SMTP_HOST", "SMTP_USERNAME", "SMTP_PASSWORD"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: SendEmailParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let from = if let Some(ref name) = self.config.from_name {
            format!("{} <{}>", name, self.config.from_email)
        } else {
            self.config.from_email.clone()
        };

        let content_type = if params.html {
            ContentType::TEXT_HTML
        } else {
            ContentType::TEXT_PLAIN
        };

        let mut email_builder = Message::builder()
            .from(from.parse().map_err(|e| kkr_core::Error::Tool(format!("Invalid from address: {}", e)))?)
            .to(params.to.parse().map_err(|e| kkr_core::Error::Tool(format!("Invalid to address: {}", e)))?)
            .subject(&params.subject);

        if let Some(cc_list) = params.cc {
            for cc in cc_list {
                email_builder = email_builder.cc(cc.parse().map_err(|e| kkr_core::Error::Tool(format!("Invalid CC address: {}", e)))?);
            }
        }

        if let Some(bcc_list) = params.bcc {
            for bcc in bcc_list {
                email_builder = email_builder.bcc(bcc.parse().map_err(|e| kkr_core::Error::Tool(format!("Invalid BCC address: {}", e)))?);
            }
        }

        let email = email_builder
            .header(content_type)
            .body(params.body)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(creds)
            .port(self.config.port)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to send email: {}", e)))?;

        Ok(json!({
            "success": true,
            "message": format!("Email sent to {}", params.to)
        }))
    }
}

pub struct ResendEmailTool {
    client: Client,
    api_key: Option<String>,
    from_email: String,
}

impl ResendEmailTool {
    pub fn new(from_email: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: env::var("RESEND_API_KEY").ok(),
            from_email: from_email.into(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let from_email = env::var("RESEND_FROM_EMAIL")
            .map_err(|_| kkr_core::Error::Config("RESEND_FROM_EMAIL not set".into()))?;
        Ok(Self::new(from_email))
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[derive(Debug, Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResendEmailParams {
    to: Vec<String>,
    subject: String,
    #[serde(default)]
    html: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    cc: Option<Vec<String>>,
    #[serde(default)]
    bcc: Option<Vec<String>>,
    #[serde(default)]
    reply_to: Option<String>,
}

#[async_trait]
impl Tool for ResendEmailTool {
    fn name(&self) -> &str {
        "resend_send_email"
    }

    fn description(&self) -> &str {
        "Send an email via Resend API. Modern email API with great deliverability."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "to": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Recipient email addresses"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject line"
                },
                "html": {
                    "type": "string",
                    "description": "HTML body content"
                },
                "text": {
                    "type": "string",
                    "description": "Plain text body content"
                },
                "cc": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "CC recipients"
                },
                "bcc": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "BCC recipients"
                },
                "reply_to": {
                    "type": "string",
                    "description": "Reply-to email address"
                }
            }),
            required: vec!["to".to_string(), "subject".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["email", "resend", "send", "api", "communication"])
            .with_read_only(false)
            .with_priority(75)
            .with_requires(vec!["RESEND_API_KEY"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = env::var("RESEND_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::Tool("RESEND_API_KEY not set".to_string()))?;

        let params: ResendEmailParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        if params.html.is_none() && params.text.is_none() {
            return Err(kkr_core::Error::Tool("Either html or text must be provided".to_string()));
        }

        let request = ResendRequest {
            from: self.from_email.clone(),
            to: params.to.clone(),
            subject: params.subject,
            html: params.html,
            text: params.text,
            cc: params.cc,
            bcc: params.bcc,
            reply_to: params.reply_to,
        };

        let response = self
            .client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Tool(format!("Resend API error: {}", body)));
        }

        Ok(json!({
            "success": true,
            "id": body.get("id"),
            "recipients": params.to
        }))
    }
}

pub mod gmail;

pub use gmail::{
    GmailClient, GmailCredentials, GmailDraftTool, GmailLabel, GmailLabelsTool, GmailMessage,
    GmailModifyLabelsTool, GmailReadTool, GmailSendTool, GmailTrashTool,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_config_gmail() {
        let config = SmtpConfig::gmail("user@gmail.com", "app-password")
            .from_email("user@gmail.com")
            .from_name("Test User");

        assert_eq!(config.host, "smtp.gmail.com");
        assert_eq!(config.port, 587);
    }

    #[test]
    fn test_resend_tool_metadata() {
        let tool = ResendEmailTool::new("test@example.com");
        assert_eq!(tool.metadata().category, ToolCategory::Network);
    }
}
