use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error, warn};

const ANTHROPIC_API: &str = "https://api.anthropic.com";
const OPENAI_API: &str = "https://api.openai.com";

// ── Trait ─────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
}

// ── Config ────────────────────────────────────────────────────────────────────

pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub retries: u32,
    pub backoff_seconds: u64,
}

// ── Anthropic ─────────────────────────────────────────────────────────────────

pub struct AnthropicProvider {
    http: Client,
    api_key: String,
    model: String,
    retries: u32,
    backoff_seconds: u64,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String, retries: u32, backoff_seconds: u64) -> Self {
        Self::with_base_url(api_key, model, retries, backoff_seconds, ANTHROPIC_API)
    }

    pub fn with_base_url(
        api_key: String,
        model: String,
        retries: u32,
        backoff_seconds: u64,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model,
            retries,
            backoff_seconds,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
        });

        let max_attempts = self.retries + 1;
        debug!(
            provider = "anthropic",
            model = self.model.as_str(),
            "Sending completion request"
        );
        for attempt in 1..=max_attempts {
            let resp = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Failed to connect to Anthropic API")?;

            let status = resp.status();
            if status.as_u16() == 429 {
                if attempt < max_attempts {
                    warn!(
                        provider = "anthropic",
                        model = self.model.as_str(),
                        attempt,
                        max_attempts,
                        backoff_seconds = self.backoff_seconds,
                        "Rate limited, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(self.backoff_seconds)).await;
                    continue;
                }
                error!(
                    provider = "anthropic",
                    model = self.model.as_str(),
                    "Rate limit exhausted"
                );
                bail!("Anthropic API rate limited after {max_attempts} attempt(s): HTTP 429");
            }
            if !status.is_success() {
                bail!("Anthropic API error: HTTP {status}");
            }

            let json: Value = resp
                .json()
                .await
                .context("Failed to parse Anthropic response")?;
            let text = json["content"][0]["text"]
                .as_str()
                .context("Unexpected Anthropic response format")?
                .to_string();
            debug!(
                provider = "anthropic",
                model = self.model.as_str(),
                response_chars = text.len(),
                "Completion received"
            );
            return Ok(text);
        }
        bail!("Anthropic API rate limited after {max_attempts} attempt(s)");
    }
}

// ── OpenAI ────────────────────────────────────────────────────────────────────

pub struct OpenAIProvider {
    http: Client,
    api_key: String,
    model: String,
    retries: u32,
    backoff_seconds: u64,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String, retries: u32, backoff_seconds: u64) -> Self {
        Self::with_base_url(api_key, model, retries, backoff_seconds, OPENAI_API)
    }

    pub fn with_base_url(
        api_key: String,
        model: String,
        retries: u32,
        backoff_seconds: u64,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model,
            retries,
            backoff_seconds,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl ModelProvider for OpenAIProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}]
        });

        let max_attempts = self.retries + 1;
        debug!(
            provider = "openai",
            model = self.model.as_str(),
            "Sending completion request"
        );
        for attempt in 1..=max_attempts {
            let resp = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Failed to connect to OpenAI API")?;

            let status = resp.status();
            if status.as_u16() == 429 {
                if attempt < max_attempts {
                    warn!(
                        provider = "openai",
                        model = self.model.as_str(),
                        attempt,
                        max_attempts,
                        backoff_seconds = self.backoff_seconds,
                        "Rate limited, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(self.backoff_seconds)).await;
                    continue;
                }
                error!(
                    provider = "openai",
                    model = self.model.as_str(),
                    "Rate limit exhausted"
                );
                bail!("OpenAI API rate limited after {max_attempts} attempt(s): HTTP 429");
            }
            if !status.is_success() {
                bail!("OpenAI API error: HTTP {status}");
            }

            let json: Value = resp
                .json()
                .await
                .context("Failed to parse OpenAI response")?;
            let text = json["choices"][0]["message"]["content"]
                .as_str()
                .context("Unexpected OpenAI response format")?
                .to_string();
            debug!(
                provider = "openai",
                model = self.model.as_str(),
                response_chars = text.len(),
                "Completion received"
            );
            return Ok(text);
        }
        bail!("OpenAI API rate limited after {max_attempts} attempt(s)");
    }
}

// ── Factory ───────────────────────────────────────────────────────────────────

fn read_api_key(candidates: &[&str]) -> Result<String> {
    for var in candidates {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return Ok(val);
            }
        }
    }
    bail!(
        "None of the following environment variables are set or non-empty: {}",
        candidates.join(", ")
    )
}

pub fn create_provider(config: &ModelConfig) -> Result<Box<dyn ModelProvider>> {
    match config.provider.as_str() {
        "anthropic" => {
            let api_key = read_api_key(&["INPUT_ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY"])?;
            Ok(Box::new(AnthropicProvider::new(
                api_key,
                config.model.clone(),
                config.retries,
                config.backoff_seconds,
            )))
        }
        "openai" => {
            let api_key = read_api_key(&["INPUT_OPENAI_API_KEY", "OPENAI_API_KEY"])?;
            Ok(Box::new(OpenAIProvider::new(
                api_key,
                config.model.clone(),
                config.retries,
                config.backoff_seconds,
            )))
        }
        other => bail!("Unknown provider: '{other}'. Accepted values: anthropic, openai"),
    }
}
