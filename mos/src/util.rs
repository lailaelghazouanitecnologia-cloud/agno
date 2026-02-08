//! Shared utilities — string manipulation, JSON extraction, timestamps.
//!
//! Small helper functions used across multiple modules.
//! No domain logic — just text processing and formatting.

/// Sanitize a string into a valid node/feature ID (lowercase, hyphenated).
pub fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Extract JSON from a model response (handles ```json blocks or raw JSON).
pub fn extract_json(response: &str) -> String {
    // Try ```json block first
    if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    // Try raw JSON
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }
    response.to_string()
}

/// Convert CamelCase to hyphen-case: "ArithmeticVM" → "arithmetic-vm"
pub fn camel_to_hyphen(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

/// Current time as a Unix timestamp string.
pub fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// Estimate token count from text (~4 chars per token).
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32) / 4
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("Hello World"), "hello-world");
        assert_eq!(sanitize_id("foo/bar.ts"), "foo-bar-ts");
        assert_eq!(sanitize_id("my_feature-1"), "my_feature-1");
    }

    #[test]
    fn test_extract_json() {
        let md = "Here is the plan:\n```json\n{\"features\": []}\n```\nDone.";
        assert_eq!(extract_json(md), "{\"features\": []}");

        let raw = "Some text {\"key\": \"value\"} more text";
        assert_eq!(extract_json(raw), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_camel_to_hyphen() {
        assert_eq!(camel_to_hyphen("ArithmeticVM"), "arithmetic-v-m");
        assert_eq!(camel_to_hyphen("lexer"), "lexer");
        assert_eq!(camel_to_hyphen("MyParser"), "my-parser");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens(&"a".repeat(400)), 100);
    }
}
