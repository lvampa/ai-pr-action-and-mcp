use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

pub struct OpenAI {
    model: String,
    api_key: String,
    client: Client,
}

impl OpenAI {
    pub fn new(model: String, api_key: String) -> Self {
        Self { model, api_key, client: Client::new() }
    }

    pub async fn review(&self, prompt: &str) -> Result<String> {
        let response: Value = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        response["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Unexpected OpenAI response format"))
    }
}
