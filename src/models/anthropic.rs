use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

pub struct Anthropic {
    model: String,
    api_key: String,
    client: Client,
}

impl Anthropic {
    pub fn new(model: String, api_key: String) -> Self {
        Self { model, api_key, client: Client::new() }
    }

    pub async fn review(&self, prompt: &str) -> Result<String> {
        let response: Value = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "max_tokens": 4096,
                "messages": [{ "role": "user", "content": prompt }]
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Unexpected Anthropic response format"))
    }
}
