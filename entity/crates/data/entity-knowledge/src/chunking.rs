use regex::Regex;
use uuid::Uuid;

use crate::document::Document;
use common_error::Result;

const DEFAULT_CHUNK_SIZE: usize = 1000;
const DEFAULT_OVERLAP: usize = 200;

pub trait ChunkingStrategy: Send + Sync {
    fn chunk(&self, document: Document) -> Result<Vec<Document>>;

    fn clean_text(&self, text: &str) -> String {
        let re_newlines = Regex::new(r"\n+").unwrap();
        let re_spaces = Regex::new(r" +").unwrap();

        let cleaned = re_newlines.replace_all(text, "\n");
        let cleaned = re_spaces.replace_all(&cleaned, " ");
        cleaned.trim().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct FixedSizeChunking {
    pub chunk_size: usize,
    pub overlap: usize,
    pub clean: bool,
}

impl Default for FixedSizeChunking {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_OVERLAP,
            clean: true,
        }
    }
}

impl FixedSizeChunking {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
            clean: true,
        }
    }

    pub fn with_clean(mut self, clean: bool) -> Self {
        self.clean = clean;
        self
    }
}

impl ChunkingStrategy for FixedSizeChunking {
    fn chunk(&self, document: Document) -> Result<Vec<Document>> {
        let content = if self.clean {
            self.clean_text(&document.content)
        } else {
            document.content.clone()
        };

        if content.len() <= self.chunk_size {
            return Ok(vec![document]);
        }

        let parent_id = document.id.clone();
        let chars: Vec<char> = content.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            let chunk_content: String = chars[start..end].iter().collect();

            let mut chunk = Document::new(chunk_content)
                .with_id(Uuid::new_v4().to_string())
                .with_chunk_info(chunk_index, &parent_id);

            if let Some(name) = &document.name {
                chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
            }

            for (key, value) in &document.metadata {
                chunk = chunk.with_metadata(key.clone(), value.clone());
            }

            chunks.push(chunk);

            if end >= chars.len() {
                break;
            }

            start = end - self.overlap;
            chunk_index += 1;
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone)]
pub struct RecursiveChunking {
    pub chunk_size: usize,
    pub overlap: usize,
    pub separators: Vec<String>,
}

impl Default for RecursiveChunking {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_OVERLAP,
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                ". ".to_string(),
                " ".to_string(),
            ],
        }
    }
}

impl RecursiveChunking {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
            ..Default::default()
        }
    }

    pub fn with_separators(mut self, separators: Vec<String>) -> Self {
        self.separators = separators;
        self
    }

    fn split_text(&self, text: &str, separators: &[String]) -> Vec<String> {
        if separators.is_empty() {
            return vec![text.to_string()];
        }

        let separator = &separators[0];
        let remaining_separators = &separators[1..];

        let parts: Vec<&str> = text.split(separator).collect();
        let mut result = Vec::new();
        let mut current = String::new();

        for part in parts {
            let test = if current.is_empty() {
                part.to_string()
            } else {
                format!("{}{}{}", current, separator, part)
            };

            if test.len() > self.chunk_size && !current.is_empty() {
                if current.len() > self.chunk_size && !remaining_separators.is_empty() {
                    result.extend(self.split_text(&current, remaining_separators));
                } else {
                    result.push(current);
                }
                current = part.to_string();
            } else {
                current = test;
            }
        }

        if !current.is_empty() {
            if current.len() > self.chunk_size && !remaining_separators.is_empty() {
                result.extend(self.split_text(&current, remaining_separators));
            } else {
                result.push(current);
            }
        }

        result
    }
}

impl ChunkingStrategy for RecursiveChunking {
    fn chunk(&self, document: Document) -> Result<Vec<Document>> {
        let content = self.clean_text(&document.content);

        if content.len() <= self.chunk_size {
            return Ok(vec![document]);
        }

        let parent_id = document.id.clone();
        let split_texts = self.split_text(&content, &self.separators);

        let chunks: Vec<Document> = split_texts
            .into_iter()
            .enumerate()
            .map(|(idx, text)| {
                let mut chunk = Document::new(text)
                    .with_id(Uuid::new_v4().to_string())
                    .with_chunk_info(idx, &parent_id);

                if let Some(name) = &document.name {
                    chunk = chunk.with_name(format!("{} (chunk {})", name, idx));
                }

                for (key, value) in &document.metadata {
                    chunk = chunk.with_metadata(key.clone(), value.clone());
                }

                chunk
            })
            .collect();

        Ok(chunks)
    }
}

#[derive(Debug, Clone)]
pub struct DocumentChunking {
    pub chunk_size: usize,
    pub overlap: usize,
    pub paragraph_separator: String,
}

impl Default for DocumentChunking {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE * 5,
            overlap: DEFAULT_OVERLAP,
            paragraph_separator: "\n\n".to_string(),
        }
    }
}

impl DocumentChunking {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
            ..Default::default()
        }
    }
}

impl ChunkingStrategy for DocumentChunking {
    fn chunk(&self, document: Document) -> Result<Vec<Document>> {
        let content = &document.content;
        let paragraphs: Vec<&str> = content.split(&self.paragraph_separator).collect();

        if paragraphs.len() <= 1 && content.len() <= self.chunk_size {
            return Ok(vec![document]);
        }

        let parent_id = document.id.clone();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_index = 0;

        for paragraph in paragraphs {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            let test_chunk = if current_chunk.is_empty() {
                paragraph.to_string()
            } else {
                format!("{}\n\n{}", current_chunk, paragraph)
            };

            if test_chunk.len() > self.chunk_size && !current_chunk.is_empty() {
                let mut chunk = Document::new(current_chunk.clone())
                    .with_id(Uuid::new_v4().to_string())
                    .with_chunk_info(chunk_index, &parent_id);

                if let Some(name) = &document.name {
                    chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
                }

                for (key, value) in &document.metadata {
                    chunk = chunk.with_metadata(key.clone(), value.clone());
                }

                chunks.push(chunk);
                chunk_index += 1;

                current_chunk = paragraph.to_string();
            } else {
                current_chunk = test_chunk;
            }
        }

        if !current_chunk.is_empty() {
            let mut chunk = Document::new(current_chunk)
                .with_id(Uuid::new_v4().to_string())
                .with_chunk_info(chunk_index, &parent_id);

            if let Some(name) = &document.name {
                chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
            }

            for (key, value) in &document.metadata {
                chunk = chunk.with_metadata(key.clone(), value.clone());
            }

            chunks.push(chunk);
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone)]
pub struct SentenceChunking {
    pub sentences_per_chunk: usize,
    pub overlap_sentences: usize,
}

impl Default for SentenceChunking {
    fn default() -> Self {
        Self {
            sentences_per_chunk: 5,
            overlap_sentences: 1,
        }
    }
}

impl SentenceChunking {
    pub fn new(sentences_per_chunk: usize, overlap: usize) -> Self {
        Self {
            sentences_per_chunk,
            overlap_sentences: overlap,
        }
    }

    fn split_sentences(&self, text: &str) -> Vec<String> {
        let re = Regex::new(r"[.!?]+\s+").unwrap();
        let mut sentences = Vec::new();
        let mut last_end = 0;

        for mat in re.find_iter(text) {
            let sentence = text[last_end..mat.end()].trim().to_string();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            last_end = mat.end();
        }

        if last_end < text.len() {
            let remaining = text[last_end..].trim().to_string();
            if !remaining.is_empty() {
                sentences.push(remaining);
            }
        }

        sentences
    }
}

impl ChunkingStrategy for SentenceChunking {
    fn chunk(&self, document: Document) -> Result<Vec<Document>> {
        let sentences = self.split_sentences(&document.content);

        if sentences.len() <= self.sentences_per_chunk {
            return Ok(vec![document]);
        }

        let parent_id = document.id.clone();
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < sentences.len() {
            let end = (start + self.sentences_per_chunk).min(sentences.len());
            let chunk_sentences = &sentences[start..end];
            let chunk_content = chunk_sentences.join(" ");

            let mut chunk = Document::new(chunk_content)
                .with_id(Uuid::new_v4().to_string())
                .with_chunk_info(chunk_index, &parent_id);

            if let Some(name) = &document.name {
                chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
            }

            for (key, value) in &document.metadata {
                chunk = chunk.with_metadata(key.clone(), value.clone());
            }

            chunks.push(chunk);

            if end >= sentences.len() {
                break;
            }

            start = end - self.overlap_sentences;
            chunk_index += 1;
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone)]
pub struct MarkdownChunking {
    pub chunk_size: usize,
    pub overlap: usize,
}

impl Default for MarkdownChunking {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_OVERLAP,
        }
    }
}

impl MarkdownChunking {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }
}

impl ChunkingStrategy for MarkdownChunking {
    fn chunk(&self, document: Document) -> Result<Vec<Document>> {
        let header_re = Regex::new(r"^#{1,6}\s+").unwrap();
        let lines: Vec<&str> = document.content.lines().collect();

        let mut sections = Vec::new();
        let mut current_section = String::new();
        let mut current_header = String::new();

        for line in lines {
            if header_re.is_match(line) {
                if !current_section.is_empty() {
                    sections.push((current_header.clone(), current_section.clone()));
                }
                current_header = line.to_string();
                current_section = String::new();
            } else {
                if !current_section.is_empty() {
                    current_section.push('\n');
                }
                current_section.push_str(line);
            }
        }

        if !current_section.is_empty() || !current_header.is_empty() {
            sections.push((current_header, current_section));
        }

        let parent_id = document.id.clone();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_index = 0;

        for (header, content) in sections {
            let section_text = if header.is_empty() {
                content
            } else {
                format!("{}\n{}", header, content)
            };

            let test_chunk = if current_chunk.is_empty() {
                section_text.clone()
            } else {
                format!("{}\n\n{}", current_chunk, section_text)
            };

            if test_chunk.len() > self.chunk_size && !current_chunk.is_empty() {
                let mut chunk = Document::new(current_chunk.clone())
                    .with_id(Uuid::new_v4().to_string())
                    .with_chunk_info(chunk_index, &parent_id);

                if let Some(name) = &document.name {
                    chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
                }

                for (key, value) in &document.metadata {
                    chunk = chunk.with_metadata(key.clone(), value.clone());
                }

                chunks.push(chunk);
                chunk_index += 1;
                current_chunk = section_text;
            } else {
                current_chunk = test_chunk;
            }
        }

        if !current_chunk.is_empty() {
            let mut chunk = Document::new(current_chunk)
                .with_id(Uuid::new_v4().to_string())
                .with_chunk_info(chunk_index, &parent_id);

            if let Some(name) = &document.name {
                chunk = chunk.with_name(format!("{} (chunk {})", name, chunk_index));
            }

            for (key, value) in &document.metadata {
                chunk = chunk.with_metadata(key.clone(), value.clone());
            }

            chunks.push(chunk);
        }

        if chunks.is_empty() {
            chunks.push(document);
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkingStrategyType {
    FixedSize,
    Recursive,
    Document,
    Sentence,
    Markdown,
}

impl ChunkingStrategyType {
    pub fn create(self, chunk_size: usize, overlap: usize) -> Box<dyn ChunkingStrategy> {
        match self {
            ChunkingStrategyType::FixedSize => {
                Box::new(FixedSizeChunking::new(chunk_size, overlap))
            }
            ChunkingStrategyType::Recursive => {
                Box::new(RecursiveChunking::new(chunk_size, overlap))
            }
            ChunkingStrategyType::Document => {
                Box::new(DocumentChunking::new(chunk_size, overlap))
            }
            ChunkingStrategyType::Sentence => {
                Box::new(SentenceChunking::new(chunk_size / 100, overlap / 100))
            }
            ChunkingStrategyType::Markdown => {
                Box::new(MarkdownChunking::new(chunk_size, overlap))
            }
        }
    }
}

