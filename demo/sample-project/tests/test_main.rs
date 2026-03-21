use sample_project::{greet, Config};

#[test]
fn test_greet_returns_greeting() {
    let result = greet("Rust");
    assert!(result.contains("Rust"));
}

#[test]
fn test_config_serialization() {
    let config = Config::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("sample-project"));
}
