use kkr_provider_google::*;

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
