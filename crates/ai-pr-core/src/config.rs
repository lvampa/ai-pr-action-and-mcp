use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Top-level config loaded from `ai-pr-review.yml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub summary: SummaryConfig,
    #[serde(default)]
    pub inline: InlineConfig,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub diff: DiffConfig,
    #[serde(default)]
    pub rate_limiting: RateLimitingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            summary: SummaryConfig::default(),
            inline: InlineConfig::default(),
            filters: FiltersConfig::default(),
            diff: DiffConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SummaryConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_summary_model")]
    pub model: String,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_summary_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InlineConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_inline_model")]
    pub model: String,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_inline_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FiltersConfig {
    #[serde(default = "default_exclude_patterns")]
    pub exclude: Vec<String>,
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude_patterns(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiffConfig {
    #[serde(default = "default_max_kb")]
    pub max_kb: i64,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_kb: default_max_kb(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitingConfig {
    #[serde(default = "default_retries")]
    pub retries: i64,
    #[serde(default = "default_backoff_seconds")]
    pub backoff_seconds: i64,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            retries: default_retries(),
            backoff_seconds: default_backoff_seconds(),
        }
    }
}

fn default_provider() -> String {
    "anthropic".to_string()
}
fn default_summary_model() -> String {
    "claude-haiku-4-5".to_string()
}
fn default_inline_model() -> String {
    "claude-opus-4-5".to_string()
}
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/*.lock".to_string(),
        "vendor/**".to_string(),
        "generated/**".to_string(),
    ]
}
fn default_max_kb() -> i64 {
    100
}
fn default_retries() -> i64 {
    3
}
fn default_backoff_seconds() -> i64 {
    5
}

fn validate_provider(field: &str, value: &str) -> Result<()> {
    match value {
        "anthropic" | "openai" => Ok(()),
        _ => bail!("Invalid {field}: '{value}'. Valid values: anthropic, openai"),
    }
}

impl Config {
    fn validate(&self) -> Result<()> {
        validate_provider("summary.provider", &self.summary.provider)?;
        validate_provider("inline.provider", &self.inline.provider)?;

        if self.diff.max_kb <= 0 {
            bail!("diff.max_kb must be positive, got {}", self.diff.max_kb);
        }
        if self.rate_limiting.retries < 0 {
            bail!(
                "rate_limiting.retries must be non-negative, got {}",
                self.rate_limiting.retries
            );
        }
        if self.rate_limiting.backoff_seconds < 0 {
            bail!(
                "rate_limiting.backoff_seconds must be non-negative, got {}",
                self.rate_limiting.backoff_seconds
            );
        }
        Ok(())
    }
}

/// Load and validate `ai-pr-review.yml` from `repo_root`.
/// Returns fully-resolved defaults when the file is absent.
pub fn load_config(repo_root: &Path) -> Result<Config> {
    let path = repo_root.join("ai-pr-review.yml");
    let config: Config = if path.exists() {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        serde_yaml::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", path.display()))?
    } else {
        Config::default()
    };
    config.validate()?;
    Ok(config)
}
