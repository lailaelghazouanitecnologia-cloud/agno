use kkr_provider_mistral::*;

#[test]
fn test_model_str() {
    assert_eq!(MistralModel::MistralLarge.as_str(), "mistral-large-latest");
    assert_eq!(MistralModel::Codestral.as_str(), "codestral-latest");
}

#[test]
fn test_config_builder() {
    let config = MistralConfig::new("test-key")
        .model(MistralModel::Codestral)
        .max_tokens(8192)
        .temperature(0.3);

    assert_eq!(config.max_tokens, Some(8192));
    assert_eq!(config.temperature, Some(0.3));
}
