use kkr_provider_anthropic::*;

#[test]
fn test_model_str() {
    assert_eq!(ClaudeModel::Claude35Sonnet.as_str(), "claude-3-5-sonnet-20241022");
}

#[test]
fn test_config_builder() {
    let config = AnthropicConfig::new("test-key")
        .model(ClaudeModel::Claude35Sonnet)
        .max_tokens(8192)
        .temperature(0.7);

    assert_eq!(config.max_tokens, 8192);
    assert_eq!(config.temperature, Some(0.7));
}
