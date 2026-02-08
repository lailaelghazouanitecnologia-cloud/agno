use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub name: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

impl Document {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: content.into(),
            name: None,
            metadata: HashMap::new(),
            embedding: None,
            score: None,
            chunk_index: None,
            parent_id: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    pub fn with_chunk_info(mut self, index: usize, parent_id: impl Into<String>) -> Self {
        self.chunk_index = Some(index);
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn content_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    pub fn estimate_tokens(&self) -> usize {
        self.content.len() / 4
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Pdf,
    Markdown,
    Html,
    Json,
    Csv,
    Docx,
    Pptx,
    Image,
    Audio,
    Video,
    Unknown,
}

impl ContentType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "txt" => ContentType::Text,
            "pdf" => ContentType::Pdf,
            "md" | "markdown" => ContentType::Markdown,
            "html" | "htm" => ContentType::Html,
            "json" => ContentType::Json,
            "csv" => ContentType::Csv,
            "docx" | "doc" => ContentType::Docx,
            "pptx" | "ppt" => ContentType::Pptx,
            "png" | "jpg" | "jpeg" | "gif" | "webp" => ContentType::Image,
            "mp3" | "wav" | "ogg" | "m4a" => ContentType::Audio,
            "mp4" | "mov" | "avi" | "webm" => ContentType::Video,
            _ => ContentType::Unknown,
        }
    }

    pub fn from_mime_type(mime: &str) -> Self {
        match mime {
            "text/plain" => ContentType::Text,
            "application/pdf" => ContentType::Pdf,
            "text/markdown" => ContentType::Markdown,
            "text/html" => ContentType::Html,
            "application/json" => ContentType::Json,
            "text/csv" => ContentType::Csv,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                ContentType::Docx
            }
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                ContentType::Pptx
            }
            m if m.starts_with("image/") => ContentType::Image,
            m if m.starts_with("audio/") => ContentType::Audio,
            m if m.starts_with("video/") => ContentType::Video,
            _ => ContentType::Unknown,
        }
    }
}

