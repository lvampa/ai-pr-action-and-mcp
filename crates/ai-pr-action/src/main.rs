use ai_pr_core::{
    config::load_config,
    github::GitHubClient,
    model::{create_provider, ModelConfig},
    review::run_review,
};
use anyhow::{Context, Result};

// ── Context ───────────────────────────────────────────────────────────────────

struct AppContext {
    github_token: String,
    github_repository: String,
    github_sha: String,
    pr_number: u64,
    provider_override: Option<String>,
    model_override: Option<String>,
}

impl AppContext {
    fn from_env() -> Result<Self> {
        let github_token = std::env::var("INPUT_GITHUB_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .context("INPUT_GITHUB_TOKEN is not set or empty")?;

        let github_repository =
            std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY is not set")?;

        let github_sha = std::env::var("GITHUB_SHA").context("GITHUB_SHA is not set")?;

        let event_path =
            std::env::var("GITHUB_EVENT_PATH").context("GITHUB_EVENT_PATH is not set")?;

        let pr_number = read_pr_number(&event_path)?;

        let provider_override = std::env::var("INPUT_PROVIDER")
            .ok()
            .filter(|s| !s.is_empty());
        let model_override = std::env::var("INPUT_MODEL").ok().filter(|s| !s.is_empty());

        Ok(Self {
            github_token,
            github_repository,
            github_sha,
            pr_number,
            provider_override,
            model_override,
        })
    }

    fn owner(&self) -> Result<&str> {
        self.github_repository
            .split_once('/')
            .map(|(o, _)| o)
            .context("Invalid GITHUB_REPOSITORY format, expected 'owner/repo'")
    }

    fn repo(&self) -> Result<&str> {
        self.github_repository
            .split_once('/')
            .map(|(_, r)| r)
            .context("Invalid GITHUB_REPOSITORY format, expected 'owner/repo'")
    }
}

fn read_pr_number(event_path: &str) -> Result<u64> {
    let content = std::fs::read_to_string(event_path)
        .with_context(|| format!("Failed to read GITHUB_EVENT_PATH: {event_path}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse GitHub event JSON")?;
    json["pull_request"]["number"]
        .as_u64()
        .context("Missing pull_request.number in GitHub event payload")
}

// ── Entry points ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    if let Err(e) = run().await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let ctx = AppContext::from_env()?;

    let repo_root = std::env::current_dir().context("Failed to get current directory")?;
    let mut config = load_config(&repo_root)?;

    if let Some(provider) = &ctx.provider_override {
        config.summary.provider.clone_from(provider);
        config.inline.provider.clone_from(provider);
    }
    if let Some(model) = &ctx.model_override {
        config.summary.model.clone_from(model);
        config.inline.model.clone_from(model);
    }

    let owner = ctx.owner()?;
    let repo = ctx.repo()?;

    let summary_config = ModelConfig {
        provider: config.summary.provider.clone(),
        model: config.summary.model.clone(),
        retries: config.rate_limiting.retries as u32,
        backoff_seconds: config.rate_limiting.backoff_seconds as u64,
    };
    let inline_config = ModelConfig {
        provider: config.inline.provider.clone(),
        model: config.inline.model.clone(),
        retries: config.rate_limiting.retries as u32,
        backoff_seconds: config.rate_limiting.backoff_seconds as u64,
    };

    let summary_provider = create_provider(&summary_config)?;
    let inline_provider = create_provider(&inline_config)?;

    let client = GitHubClient::new(&ctx.github_token);

    run_review(
        &client,
        owner,
        repo,
        ctx.pr_number,
        &ctx.github_sha,
        summary_provider.as_ref(),
        inline_provider.as_ref(),
        &config,
    )
    .await
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn event_file(pr_number: u64) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"pull_request":{{"number":{pr_number}}}}}"#).unwrap();
        f
    }

    // ── R1: Environment Variable Inputs ───────────────────────────────────────

    #[test]
    fn github_action_binary_missing_github_token_returns_error() {
        let f = event_file(1);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", None),
                ("GITHUB_REPOSITORY", Some("owner/repo")),
                ("GITHUB_SHA", Some("abc")),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
            ],
            || {
                let err = AppContext::from_env().err().expect("expected error");
                assert!(
                    err.to_string().contains("INPUT_GITHUB_TOKEN"),
                    "should name the missing var, got: {err}"
                );
            },
        );
    }

    #[test]
    fn github_action_binary_empty_github_token_returns_error() {
        let f = event_file(1);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", Some("")),
                ("GITHUB_REPOSITORY", Some("owner/repo")),
                ("GITHUB_SHA", Some("abc")),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
            ],
            || {
                let err = AppContext::from_env().err().expect("expected error");
                assert!(err.to_string().contains("INPUT_GITHUB_TOKEN"));
            },
        );
    }

    #[test]
    fn github_action_binary_provider_and_model_overrides_captured() {
        let f = event_file(7);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", Some("tok")),
                ("GITHUB_REPOSITORY", Some("owner/repo")),
                ("GITHUB_SHA", Some("abc")),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
                ("INPUT_PROVIDER", Some("openai")),
                ("INPUT_MODEL", Some("gpt-4")),
            ],
            || {
                let ctx = AppContext::from_env().unwrap();
                assert_eq!(ctx.provider_override.as_deref(), Some("openai"));
                assert_eq!(ctx.model_override.as_deref(), Some("gpt-4"));
            },
        );
    }

    // ── R3: GitHub Actions Context ────────────────────────────────────────────

    #[test]
    fn github_action_binary_reads_pr_number_from_event_file() {
        let f = event_file(42);
        let pr = read_pr_number(f.path().to_str().unwrap()).unwrap();
        assert_eq!(pr, 42);
    }

    #[test]
    fn github_action_binary_missing_pr_number_in_event_returns_error() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"push":{{"ref":"main"}}}}"#).unwrap();
        let err = read_pr_number(f.path().to_str().unwrap())
            .err()
            .expect("expected error");
        assert!(
            err.to_string().contains("pull_request"),
            "should mention missing field, got: {err}"
        );
    }

    #[test]
    fn github_action_binary_missing_event_file_returns_error() {
        let err = read_pr_number("/nonexistent/path/event.json")
            .err()
            .expect("expected error");
        assert!(
            err.to_string().contains("GITHUB_EVENT_PATH"),
            "should name the failing path, got: {err}"
        );
    }

    #[test]
    fn github_action_binary_missing_github_repository_returns_error() {
        let f = event_file(1);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", Some("tok")),
                ("GITHUB_REPOSITORY", None),
                ("GITHUB_SHA", Some("abc")),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
            ],
            || {
                let err = AppContext::from_env().err().expect("expected error");
                assert!(err.to_string().contains("GITHUB_REPOSITORY"));
            },
        );
    }

    #[test]
    fn github_action_binary_missing_github_sha_returns_error() {
        let f = event_file(1);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", Some("tok")),
                ("GITHUB_REPOSITORY", Some("owner/repo")),
                ("GITHUB_SHA", None),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
            ],
            || {
                let err = AppContext::from_env().err().expect("expected error");
                assert!(err.to_string().contains("GITHUB_SHA"));
            },
        );
    }

    #[test]
    fn github_action_binary_owner_and_repo_parsed_from_repository() {
        let f = event_file(1);
        temp_env::with_vars(
            [
                ("INPUT_GITHUB_TOKEN", Some("tok")),
                ("GITHUB_REPOSITORY", Some("my-org/my-repo")),
                ("GITHUB_SHA", Some("abc")),
                ("GITHUB_EVENT_PATH", Some(f.path().to_str().unwrap())),
            ],
            || {
                let ctx = AppContext::from_env().unwrap();
                assert_eq!(ctx.owner().unwrap(), "my-org");
                assert_eq!(ctx.repo().unwrap(), "my-repo");
            },
        );
    }

    // ── R2: Config File Integration ───────────────────────────────────────────

    #[test]
    fn github_action_binary_missing_config_file_proceeds_with_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let cfg = load_config(dir.path()).unwrap();
        assert_eq!(cfg.summary.provider, "anthropic");
        assert_eq!(cfg.diff.max_kb, 100);
    }
}
