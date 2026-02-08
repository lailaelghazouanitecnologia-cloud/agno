//! Supported programming languages.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Languages that the AST extractor supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    C,
    Cpp,
    Go,
    Java,
    Unknown,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Language::Rust,
            Some("js") | Some("mjs") | Some("cjs") => Language::JavaScript,
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("py") => Language::Python,
            Some("c") | Some("h") => Language::C,
            Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") => Language::Cpp,
            Some("go") => Language::Go,
            Some("java") => Language::Java,
            _ => Language::Unknown,
        }
    }

    /// File extensions for this language.
    pub fn extensions(&self) -> &[&str] {
        match self {
            Language::Rust => &["rs"],
            Language::JavaScript => &["js", "mjs", "cjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Python => &["py"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp"],
            Language::Go => &["go"],
            Language::Java => &["java"],
            Language::Unknown => &[],
        }
    }

    /// Single-line comment prefix for this language.
    pub fn comment_prefix(&self) -> &str {
        match self {
            Language::Python => "#",
            _ => "//",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Python => write!(f, "Python"),
            Language::C => write!(f, "C"),
            Language::Cpp => write!(f, "C++"),
            Language::Go => write!(f, "Go"),
            Language::Java => write!(f, "Java"),
            Language::Unknown => write!(f, "Unknown"),
        }
    }
}
