mod anthropic;
mod openai;

use anyhow::{bail, Result};

pub enum Model {
    Anthropic(anthropic::Anthropic),
    OpenAI(openai::OpenAI),
}

impl Model {
    pub async fn review(&self, prompt: &str) -> Result<String> {
        match self {
            Model::Anthropic(m) => m.review(prompt).await,
            Model::OpenAI(m) => m.review(prompt).await,
        }
    }
}

pub fn build(provider: &str, model: &str, api_key: &str) -> Result<Model> {
    match provider.to_lowercase().as_str() {
        "anthropic" => Ok(Model::Anthropic(anthropic::Anthropic::new(
            model.to_string(),
            api_key.to_string(),
        ))),
        "openai" => Ok(Model::OpenAI(openai::OpenAI::new(
            model.to_string(),
            api_key.to_string(),
        ))),
        _ => bail!("Unknown provider: {provider}. Valid options: anthropic, openai"),
    }
}
