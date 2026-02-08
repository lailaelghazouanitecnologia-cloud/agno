use kkr_provider_groq::*;

#[test]
fn test_model_str() {
    assert_eq!(GroqModel::Llama33_70B.as_str(), "llama-3.3-70b-versatile");
    assert_eq!(
        GroqModel::DeepSeekR1Llama70B.as_str(),
        "deepseek-r1-distill-llama-70b"
    );
}

#[test]
fn test_reasoning_models() {
    assert!(GroqModel::DeepSeekR1Llama70B.is_reasoning_model());
    assert!(GroqModel::QwenQwQ32B.is_reasoning_model());
    assert!(!GroqModel::Llama33_70B.is_reasoning_model());
}

#[test]
fn test_vision_models() {
    assert!(GroqModel::Llama4Scout.supports_vision());
    assert!(!GroqModel::Llama33_70B.supports_vision());
}

#[test]
fn test_config_builder() {
    let config = GroqConfig::new("test-key")
        .model(GroqModel::DeepSeekR1Llama70B)
        .max_completion_tokens(8192)
        .temperature(0.5)
        .include_reasoning(true);

    assert_eq!(config.max_completion_tokens, 8192);
    assert_eq!(config.temperature, Some(0.5));
    assert!(config.include_reasoning);
}
