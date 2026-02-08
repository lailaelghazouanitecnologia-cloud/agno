use kkr_provider_ollama::*;

#[test]
fn test_config_default() {
    let config = OllamaConfig::default();
    assert_eq!(config.base_url, "http://localhost:11434");
    assert_eq!(config.model, "llama3.2");
}

#[test]
fn test_config_builder() {
    let config = OllamaConfig::new("codellama")
        .base_url("http://192.168.1.100:11434")
        .num_predict(2048)
        .temperature(0.7)
        .num_ctx(8192);

    assert_eq!(config.model, "codellama");
    assert_eq!(config.num_predict, Some(2048));
    assert_eq!(config.num_ctx, Some(8192));
}
