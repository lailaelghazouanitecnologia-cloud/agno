use common_error::*;

#[test]
fn test_error_creation() {
    let err = Error::new(ErrorKind::FileNotFound, "config.toml not found")
        .with_context("loading project config")
        .with_location("src/config.rs", 42);

    assert_eq!(err.kind, ErrorKind::FileNotFound);
    assert!(err.to_string().contains("config.toml not found"));
    assert!(err.to_string().contains("loading project config"));
    assert!(err.to_string().contains("src/config.rs:42"));
}

#[test]
fn test_retryable() {
    assert!(Error::new(ErrorKind::Timeout, "timed out").is_retryable());
    assert!(Error::new(ErrorKind::RateLimited, "429").is_retryable());
    assert!(!Error::new(ErrorKind::Parse, "bad syntax").is_retryable());
}

#[test]
fn test_user_facing() {
    assert!(Error::new(ErrorKind::FileNotFound, "x").is_user_facing());
    assert!(!Error::new(ErrorKind::Internal, "x").is_user_facing());
}

#[test]
fn test_io_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    let err: Error = io_err.into();
    assert_eq!(err.kind, ErrorKind::FileNotFound);
}

#[test]
fn test_error_yaml_roundtrip() {
    let err = Error::new(ErrorKind::Parse, "unexpected token")
        .with_context("parsing function body");

    let yaml = serde_yaml::to_string(&err).unwrap();
    let parsed: Error = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.kind, ErrorKind::Parse);
    assert_eq!(parsed.context.len(), 1);
}

#[test]
fn test_err_macro() {
    let e = err!(ErrorKind::Config, "missing key: {}", "api_url");
    assert_eq!(e.kind, ErrorKind::Config);
    assert!(e.message.contains("api_url"));
}
