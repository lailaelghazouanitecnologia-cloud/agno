use kkr_provider_openai::*;

#[test]
fn test_model_str() {
    assert_eq!(OpenAIModel::Gpt4o.as_str(), "gpt-4o");
    assert_eq!(OpenAIModel::Gpt4oMini.as_str(), "gpt-4o-mini");
}

#[test]
fn test_model_supports_tools() {
    assert!(OpenAIModel::Gpt4o.supports_tools());
    assert!(!OpenAIModel::O1Preview.supports_tools());
}

#[test]
fn test_config_builder() {
    let config = OpenAIConfig::new("test-key")
        .model(OpenAIModel::Gpt4Turbo)
        .max_tokens(8192)
        .temperature(0.7);

    assert_eq!(config.max_tokens, Some(8192));
    assert_eq!(config.temperature, Some(0.7));
}
