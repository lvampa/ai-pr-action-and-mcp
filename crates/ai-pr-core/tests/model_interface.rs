use ai_pr_core::model::{
    AnthropicProvider, ModelConfig, ModelProvider, OpenAIProvider, create_provider,
};
use mockito::Server;

// ── Requirement 1 + 2: Anthropic Implementation ───────────────────────────────

#[tokio::test]
async fn model_interface_anthropic_complete_returns_response_text() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_body(r#"{"content":[{"type":"text","text":"hello from claude"}]}"#)
        .create_async()
        .await;

    let provider =
        AnthropicProvider::with_base_url("key".into(), "claude-3".into(), 0, 0, server.url());
    let result = provider.complete("say hello").await.unwrap();
    assert_eq!(result, "hello from claude");
    m.assert_async().await;
}

#[tokio::test]
async fn model_interface_anthropic_api_error_returns_descriptive_error() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/v1/messages")
        .with_status(500)
        .with_body(r#"{"error":{"message":"Internal Server Error"}}"#)
        .create_async()
        .await;

    let provider =
        AnthropicProvider::with_base_url("key".into(), "claude-3".into(), 0, 0, server.url());
    let err = provider.complete("prompt").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("500"), "error should include HTTP status, got: {msg}");
    assert!(msg.contains("Anthropic"), "error should name the provider, got: {msg}");
}

#[test]
fn model_interface_anthropic_missing_api_key_returns_error() {
    temp_env::with_var_unset("INPUT_ANTHROPIC_API_KEY", || {
        let config = ModelConfig {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            retries: 0,
            backoff_seconds: 0,
        };
        let err = create_provider(&config).err().expect("expected error");
        let msg = err.to_string();
        assert!(
            msg.contains("INPUT_ANTHROPIC_API_KEY"),
            "should name the missing env var, got: {msg}"
        );
    });
}

#[test]
fn model_interface_anthropic_empty_api_key_returns_error() {
    temp_env::with_var("INPUT_ANTHROPIC_API_KEY", Some(""), || {
        let config = ModelConfig {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            retries: 0,
            backoff_seconds: 0,
        };
        let err = create_provider(&config).err().expect("expected error");
        let msg = err.to_string();
        assert!(
            msg.contains("INPUT_ANTHROPIC_API_KEY"),
            "should name the empty env var, got: {msg}"
        );
    });
}

// retries=1 → 2 total attempts; first returns 429 (exhausted after 1 hit), second returns 200
#[tokio::test]
async fn model_interface_anthropic_rate_limit_retries_and_succeeds() {
    let mut server = Server::new_async().await;

    // Registered first (FIFO) — exhausted after one match, then falls through
    let retry_mock = server
        .mock("POST", "/v1/messages")
        .with_status(429)
        .expect(1)
        .create_async()
        .await;

    // Fallback (registered second) — matches the retry attempt
    server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_body(r#"{"content":[{"type":"text","text":"ok"}]}"#)
        .create_async()
        .await;

    let provider =
        AnthropicProvider::with_base_url("key".into(), "claude-3".into(), 1, 0, server.url());
    let result = provider.complete("prompt").await.unwrap();
    assert_eq!(result, "ok");
    retry_mock.assert_async().await; // asserts exactly 1 hit on the 429 mock
}

#[tokio::test]
async fn model_interface_anthropic_rate_limit_exhausted_returns_error() {
    let mut server = Server::new_async().await;
    // retries=1 → 2 total attempts
    let m = server
        .mock("POST", "/v1/messages")
        .with_status(429)
        .expect(2)
        .create_async()
        .await;

    let provider =
        AnthropicProvider::with_base_url("key".into(), "claude-3".into(), 1, 0, server.url());
    let err = provider.complete("prompt").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("2"), "should include attempt count, got: {msg}");
    assert!(
        msg.contains("attempt") || msg.contains("rate"),
        "should describe exhaustion, got: {msg}"
    );
    m.assert_async().await;
}

// ── Requirement 3: OpenAI Implementation ─────────────────────────────────────

#[tokio::test]
async fn model_interface_openai_complete_returns_response_text() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"content":"hello from gpt"}}]}"#)
        .create_async()
        .await;

    let provider =
        OpenAIProvider::with_base_url("key".into(), "gpt-4".into(), 0, 0, server.url());
    let result = provider.complete("say hello").await.unwrap();
    assert_eq!(result, "hello from gpt");
    m.assert_async().await;
}

#[tokio::test]
async fn model_interface_openai_api_error_returns_descriptive_error() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/v1/chat/completions")
        .with_status(500)
        .with_body(r#"{"error":{"message":"Internal Server Error"}}"#)
        .create_async()
        .await;

    let provider =
        OpenAIProvider::with_base_url("key".into(), "gpt-4".into(), 0, 0, server.url());
    let err = provider.complete("prompt").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("500"), "error should include HTTP status, got: {msg}");
    assert!(msg.contains("OpenAI"), "error should name the provider, got: {msg}");
}

#[test]
fn model_interface_openai_missing_api_key_returns_error() {
    temp_env::with_var_unset("INPUT_OPENAI_API_KEY", || {
        let config = ModelConfig {
            provider: "openai".into(),
            model: "gpt-4".into(),
            retries: 0,
            backoff_seconds: 0,
        };
        let err = create_provider(&config).err().expect("expected error");
        let msg = err.to_string();
        assert!(
            msg.contains("INPUT_OPENAI_API_KEY"),
            "should name the missing env var, got: {msg}"
        );
    });
}

#[test]
fn model_interface_openai_empty_api_key_returns_error() {
    temp_env::with_var("INPUT_OPENAI_API_KEY", Some(""), || {
        let config = ModelConfig {
            provider: "openai".into(),
            model: "gpt-4".into(),
            retries: 0,
            backoff_seconds: 0,
        };
        let err = create_provider(&config).err().expect("expected error");
        let msg = err.to_string();
        assert!(
            msg.contains("INPUT_OPENAI_API_KEY"),
            "should name the empty env var, got: {msg}"
        );
    });
}

#[tokio::test]
async fn model_interface_openai_rate_limit_retries_and_succeeds() {
    let mut server = Server::new_async().await;

    // Registered first (FIFO) — exhausted after one match, then falls through
    let retry_mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(429)
        .expect(1)
        .create_async()
        .await;

    // Fallback (registered second) — matches the retry attempt
    server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"content":"ok"}}]}"#)
        .create_async()
        .await;

    let provider =
        OpenAIProvider::with_base_url("key".into(), "gpt-4".into(), 1, 0, server.url());
    let result = provider.complete("prompt").await.unwrap();
    assert_eq!(result, "ok");
    retry_mock.assert_async().await;
}

#[tokio::test]
async fn model_interface_openai_rate_limit_exhausted_returns_error() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/v1/chat/completions")
        .with_status(429)
        .expect(2)
        .create_async()
        .await;

    let provider =
        OpenAIProvider::with_base_url("key".into(), "gpt-4".into(), 1, 0, server.url());
    let err = provider.complete("prompt").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("2"), "should include attempt count, got: {msg}");
    assert!(
        msg.contains("attempt") || msg.contains("rate"),
        "should describe exhaustion, got: {msg}"
    );
    m.assert_async().await;
}

// ── Requirement 4: Factory Resolution ────────────────────────────────────────

#[test]
fn model_interface_create_provider_anthropic_returns_anthropic_provider() {
    temp_env::with_var("INPUT_ANTHROPIC_API_KEY", Some("test-key"), || {
        let config = ModelConfig {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            retries: 3,
            backoff_seconds: 5,
        };
        assert!(create_provider(&config).is_ok());
    });
}

#[test]
fn model_interface_create_provider_openai_returns_openai_provider() {
    temp_env::with_var("INPUT_OPENAI_API_KEY", Some("test-key"), || {
        let config = ModelConfig {
            provider: "openai".into(),
            model: "gpt-4".into(),
            retries: 3,
            backoff_seconds: 5,
        };
        assert!(create_provider(&config).is_ok());
    });
}

#[test]
fn model_interface_create_provider_unknown_returns_error() {
    let config = ModelConfig {
        provider: "cohere".into(),
        model: "command".into(),
        retries: 0,
        backoff_seconds: 0,
    };
    let err = create_provider(&config).err().expect("expected error");
    let msg = err.to_string();
    assert!(msg.contains("cohere"), "should name the invalid provider, got: {msg}");
    assert!(msg.contains("anthropic"), "should list valid values, got: {msg}");
    assert!(msg.contains("openai"), "should list valid values, got: {msg}");
}
