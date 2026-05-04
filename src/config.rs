use anyhow::Result;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub github_token: String,
    pub prompt: Option<String>,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_model() -> String {
    "claude-opus-4-5".to_string()
}

impl Config {
    pub fn load() -> Result<Self> {
        Ok(Figment::new()
            .merge(Yaml::file("config.yml"))
            .merge(Env::prefixed("INPUT_"))
            .extract()?)
    }

    pub fn api_key(&self) -> Option<&str> {
        match self.provider.as_str() {
            "openai" => self.openai_api_key.as_deref(),
            _ => self.anthropic_api_key.as_deref(),
        }
    }
}
