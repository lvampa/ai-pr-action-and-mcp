use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;
use tracing::debug;

const BUILTIN_INSTRUCTIONS: &str = include_str!("../defaults/review-instructions.md");

// ── Config structs ────────────────────────────────────────────────────────────

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
    /// Path to a review instructions Markdown file, relative to repo root.
    /// Defaults to `review-instructions.md` at the repo root if absent.
    pub review_instructions: Option<String>,
    /// Resolved instructions text — populated by `load_config`, not from YAML.
    #[serde(skip)]
    pub instructions: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            summary: SummaryConfig::default(),
            inline: InlineConfig::default(),
            filters: FiltersConfig::default(),
            diff: DiffConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            review_instructions: None,
            instructions: BUILTIN_INSTRUCTIONS.to_string(),
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

// ── Defaults ──────────────────────────────────────────────────────────────────

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

// ── Validation ────────────────────────────────────────────────────────────────

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

// ── Loader ────────────────────────────────────────────────────────────────────

/// Load and validate `ai-pr-review.yml` from `repo_root`.
/// Returns fully-resolved defaults when the file is absent.
pub fn load_config(repo_root: &Path) -> Result<Config> {
    let path = repo_root.join("ai-pr-review.yml");
    let mut config: Config = if path.exists() {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        serde_yaml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?
    } else {
        Config::default()
    };

    config.instructions = resolve_instructions(&config, repo_root)?;
    config.validate()?;
    Ok(config)
}

fn resolve_instructions(config: &Config, repo_root: &Path) -> Result<String> {
    if let Some(ref rel_path) = config.review_instructions {
        let path = repo_root.join(rel_path);
        if !path.exists() {
            bail!("review_instructions file not found: {}", path.display());
        }
        debug!(path = %path.display(), "Using review instructions from configured file");
        return std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read review instructions: {}", path.display()));
    }

    let default_path = repo_root.join("review-instructions.md");
    if default_path.exists() {
        debug!(path = %default_path.display(), "Using review instructions from default file");
        return std::fs::read_to_string(&default_path)
            .with_context(|| format!("Failed to read {}", default_path.display()));
    }

    debug!("Using built-in default review instructions");
    Ok(BUILTIN_INSTRUCTIONS.to_string())
}
