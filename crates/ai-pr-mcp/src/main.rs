use std::{path::PathBuf, sync::Arc};

use ai_pr_core::{
    config::load_config,
    github::GitHubClient,
    inline::{filter_valid_findings, parse_findings},
    model::{create_provider, ModelConfig},
    output::{format_review_markdown, parse_review_findings, write_review},
    plan::{build_review_plan, write_plan},
    review::{build_inline_prompt, build_summary_prompt},
};
use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::ServerInfo,
    tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Tool input types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SynthesizePlanInput {
    /// GitHub repository owner (e.g. "octocat").
    pub owner: String,
    /// GitHub repository name (e.g. "hello-world").
    pub repo: String,
    /// Pull request number.
    pub pr_number: u64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReviewPrInput {
    /// Pull request number.
    pub pr_number: u64,
    /// Repository owner. Inferred from git remote if omitted.
    pub owner: Option<String>,
    /// Repository name. Inferred from git remote if omitted.
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ApplyReviewInput {
    /// When true, apply planned changes and commit. Default: false (plan only).
    pub confirmed: Option<bool>,
    /// When true, proceed even if there are uncommitted changes. Default: false.
    pub force: Option<bool>,
}

// ── Server ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ReviewServer {
    tool_router: ToolRouter<Self>,
    github_token: Arc<String>,
    repo_root: Arc<PathBuf>,
}

#[tool_router]
impl ReviewServer {
    fn new(github_token: String, repo_root: PathBuf) -> Self {
        Self {
            tool_router: Self::tool_router(),
            github_token: Arc::new(github_token),
            repo_root: Arc::new(repo_root),
        }
    }

    /// Fetch the PR diff and all comments, synthesize a structured review plan,
    /// and write it to `.ai-reviews/<branch>-review-plan.md`.
    #[tool(
        description = "Fetch PR diff and comments, synthesize a review plan, write to .ai-reviews/<branch>-review-plan.md"
    )]
    async fn synthesize_plan(
        &self,
        Parameters(input): Parameters<SynthesizePlanInput>,
    ) -> Result<String, ErrorData> {
        let client = self.github_client()?;

        let pr_info = client
            .get_pr_info(&input.owner, &input.repo, input.pr_number)
            .await
            .map_err(internal)?;

        let diff = client
            .fetch_full_diff(&input.owner, &input.repo, input.pr_number)
            .await
            .map_err(internal)?;

        let comments = client
            .list_issue_comments(&input.owner, &input.repo, input.pr_number)
            .await
            .map_err(internal)?;

        let plan = build_review_plan(&pr_info, &diff, &comments);
        let path = write_plan(&self.repo_root, &pr_info.head.branch, &plan).map_err(internal)?;

        Ok(format!("Review plan written to: {path}"))
    }

    /// Run a full two-pass AI review on a PR. Returns the review as Markdown
    /// grouped by severity and writes it to `.ai-reviews/<branch>-review.md`.
    /// Optionally posts the review to GitHub when `post_to_github` is true.
    #[tool(
        description = "Run full two-pass AI review on a PR, return Markdown findings, write to .ai-reviews/<branch>-review.md"
    )]
    async fn review_pr(
        &self,
        Parameters(input): Parameters<ReviewPrInput>,
    ) -> Result<String, ErrorData> {
        let client = self.github_client()?;

        let (owner, repo) = match (input.owner, input.repo) {
            (Some(o), Some(r)) => (o, r),
            _ => infer_git_remote(&self.repo_root)
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?,
        };

        let pr_info = client
            .get_pr_info(&owner, &repo, input.pr_number)
            .await
            .map_err(internal)?;

        let config = load_config(&self.repo_root).map_err(internal)?;

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

        let summary_provider = create_provider(&summary_config).map_err(internal)?;
        let inline_provider = create_provider(&inline_config).map_err(internal)?;

        let diff = client
            .fetch_full_diff(&owner, &repo, input.pr_number)
            .await
            .map_err(internal)?;

        if diff.is_empty() {
            return Ok("No diff found for this PR.".into());
        }

        // Pass 1: summary
        let summary_prompt = build_summary_prompt(&config.instructions, &diff);
        let summary = summary_provider
            .complete(&summary_prompt)
            .await
            .map_err(|e| internal(anyhow::anyhow!("Pass 1 failed: {e}")))?;

        // Pass 2: inline
        let inline_prompt = build_inline_prompt(&config.instructions, &diff);
        let inline_json = inline_provider
            .complete(&inline_prompt)
            .await
            .map_err(|e| internal(anyhow::anyhow!("Pass 2 failed: {e}")))?;

        let findings = match parse_findings(&inline_json) {
            Ok(f) => filter_valid_findings(f, &diff),
            Err(_) => vec![],
        };

        let review_md = format_review_markdown(&pr_info, &summary, &findings);
        let path =
            write_review(&self.repo_root, &pr_info.head.branch, &review_md).map_err(internal)?;

        Ok(format!("{review_md}\n---\n_Review written to: {path}_"))
    }

    /// Read the review for the current branch from `.ai-reviews/<branch>-review.md`,
    /// build an actionable change plan, and (when confirmed) apply the changes and commit.
    #[tool(
        description = "Read review findings for current branch, build actionable plan, apply changes and commit when confirmed=true"
    )]
    async fn apply_review(
        &self,
        Parameters(input): Parameters<ApplyReviewInput>,
    ) -> Result<String, ErrorData> {
        let branch = infer_current_branch(&self.repo_root)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;

        let safe_branch = branch.replace('/', "-");
        let review_path = self
            .repo_root
            .join(".ai-reviews")
            .join(format!("{safe_branch}-review.md"));

        if !review_path.exists() {
            return Err(ErrorData::invalid_params(
                format!(
                    "No review file found at {path}. Run the `review_pr` tool first.",
                    path = review_path.display()
                ),
                None,
            ));
        }

        let review_content = std::fs::read_to_string(&review_path)
            .map_err(|e| internal(anyhow::anyhow!("Failed to read review file: {e}")))?;

        if review_content.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "Review file exists but is empty. Run the `review_pr` tool again.",
                None,
            ));
        }

        let findings = parse_review_findings(&review_content)
            .map_err(|e| internal(anyhow::anyhow!("Failed to parse review findings: {e}")))?;

        // Filter to actionable findings (file + line present, file exists on disk)
        let actionable: Vec<_> = findings
            .iter()
            .filter(|f| !f.file.is_empty() && self.repo_root.join(&f.file).exists())
            .collect();

        let informational_count = findings.len() - actionable.len();

        if actionable.is_empty() {
            let mut msg = "No actionable changes found".to_string();
            if informational_count > 0 {
                msg.push_str(&format!(
                    " ({informational_count} informational finding(s) noted but no file changes required)"
                ));
            }
            msg.push('.');
            return Ok(msg);
        }

        // Safety: check for uncommitted changes unless force=true
        if !input.force.unwrap_or(false) {
            let dirty = git_is_dirty(&self.repo_root)
                .map_err(|e| internal(anyhow::anyhow!("Failed to check git status: {e}")))?;
            if let Some(files) = dirty {
                return Err(ErrorData::invalid_params(
                    format!(
                        "Working tree has uncommitted changes in: {files}\n\
                         Pass `force: true` to proceed anyway."
                    ),
                    None,
                ));
            }
        }

        // Build the plan text
        let mut plan = String::from("## Actionable Changes\n\n");
        for f in &actionable {
            plan.push_str(&format!(
                "- **`{}` line {}** [{}]: {}\n",
                f.file, f.line, f.severity, f.comment
            ));
        }
        if informational_count > 0 {
            plan.push_str(&format!(
                "\n_{informational_count} informational finding(s) not applied (no matching file on disk)._\n"
            ));
        }

        if !input.confirmed.unwrap_or(false) {
            plan.push_str(&format!(
                "\n---\nFound **{}** actionable change(s).\n\
                 Call `apply_review` again with `confirmed: true` to apply and commit.",
                actionable.len()
            ));
            return Ok(plan);
        }

        // Apply changes: get the AI to generate fixes for each finding
        let config = load_config(&self.repo_root).map_err(internal)?;
        let inline_config = ModelConfig {
            provider: config.inline.provider.clone(),
            model: config.inline.model.clone(),
            retries: config.rate_limiting.retries as u32,
            backoff_seconds: config.rate_limiting.backoff_seconds as u64,
        };
        let provider = create_provider(&inline_config).map_err(internal)?;

        let mut applied = vec![];
        let mut skipped = vec![];

        for f in &actionable {
            let file_path = self.repo_root.join(&f.file);
            let current_content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(e) => {
                    skipped.push(format!("{} (read error: {e})", f.file));
                    continue;
                }
            };

            let prompt = format!(
                "You are applying a code review suggestion. Here is the current content of `{file}`:\n\n\
                 ```\n{content}\n```\n\n\
                 The reviewer noted at line {line}: \"{comment}\"\n\n\
                 Return ONLY the complete modified file content with this suggestion applied. \
                 Make the minimal change needed. Do not include any explanation or markdown fencing.",
                file = f.file,
                content = current_content,
                line = f.line,
                comment = f.comment,
            );

            match provider.complete(&prompt).await {
                Ok(new_content) => {
                    if let Err(e) = std::fs::write(&file_path, &new_content) {
                        skipped.push(format!("{} (write error: {e})", f.file));
                    } else {
                        applied.push(f.file.clone());
                    }
                }
                Err(e) => {
                    skipped.push(format!("{} (model error: {e})", f.file));
                }
            }
        }

        if applied.is_empty() {
            let mut msg = format!("No changes applied. Skipped: {}", skipped.join(", "));
            if !skipped.is_empty() {
                msg.push_str("\nAll planned changes were skipped.");
            }
            return Ok(msg);
        }

        // Stage and commit
        let commit_msg = format!("Apply AI review suggestions for branch {branch}");
        match git_commit(&self.repo_root, &applied, &commit_msg) {
            Ok(()) => {
                let mut out = format!("## Applied {} change(s)\n\n", applied.len());
                for f in &applied {
                    out.push_str(&format!("- `{f}`\n"));
                }
                if !skipped.is_empty() {
                    out.push_str(&format!("\n⚠️ Skipped: {}\n", skipped.join(", ")));
                }
                out.push_str("\nChanges committed. Use `git push` to share them.");
                Ok(out)
            }
            Err(e) => Err(internal(anyhow::anyhow!(
                "Changes applied but commit failed: {e}"
            ))),
        }
    }

    fn github_client(&self) -> Result<GitHubClient, ErrorData> {
        let token = self.github_token.as_ref();
        if token.is_empty() {
            return Err(ErrorData::invalid_params(
                "GITHUB_TOKEN is not set or empty",
                None,
            ));
        }
        Ok(GitHubClient::new(token))
    }
}

#[tool_handler]
impl ServerHandler for ReviewServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "AI PR review tools. Use `synthesize_plan` to build a review plan, \
                 `review_pr` to run a full two-pass review, and `apply_review` to apply suggestions."
                    .into(),
            ),
            ..ServerInfo::default()
        }
    }
}

// ── Git helpers ───────────────────────────────────────────────────────────────

/// Returns (owner, repo) parsed from `git remote get-url origin`.
fn infer_git_remote(root: &PathBuf) -> Result<(String, String)> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

    anyhow::ensure!(out.status.success(), "git remote get-url origin failed");

    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    parse_github_remote(&url)
        .ok_or_else(|| anyhow::anyhow!("Could not parse owner/repo from remote URL: {url}"))
}

fn parse_github_remote(url: &str) -> Option<(String, String)> {
    // https://github.com/owner/repo.git  or  https://github.com/owner/repo
    // git@github.com:owner/repo.git
    let stripped = url.trim_end_matches(".git").trim_end_matches('/');

    if let Some(rest) = stripped.strip_prefix("git@github.com:") {
        let (owner, repo) = rest.split_once('/')?;
        return Some((owner.to_string(), repo.to_string()));
    }

    if let Some(rest) = stripped.strip_prefix("https://github.com/") {
        let (owner, repo) = rest.split_once('/')?;
        return Some((owner.to_string(), repo.to_string()));
    }

    None
}

/// Returns the current branch name.
fn infer_current_branch(root: &PathBuf) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

    anyhow::ensure!(out.status.success(), "git rev-parse failed");
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Returns Some(file_list) when the working tree is dirty, None when clean.
fn git_is_dirty(root: &PathBuf) -> Result<Option<String>> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git status: {e}"))?;

    let output = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if output.is_empty() {
        Ok(None)
    } else {
        Ok(Some(output))
    }
}

/// Stage `files` and create a commit with `message`.
fn git_commit(root: &PathBuf, files: &[String], message: &str) -> Result<()> {
    for file in files {
        let status = std::process::Command::new("git")
            .args(["add", file])
            .current_dir(root)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to run git add: {e}"))?;
        anyhow::ensure!(status.success(), "git add {file} failed");
    }

    let status = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(root)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run git commit: {e}"))?;
    anyhow::ensure!(status.success(), "git commit failed");

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn internal(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    if github_token.is_empty() {
        eprintln!("Warning: GITHUB_TOKEN is not set — tool calls will fail");
    }

    let repo_root = std::env::current_dir()?;
    let server = ReviewServer::new(github_token, repo_root);
    let transport = rmcp::transport::stdio();
    server.serve(transport).await?.waiting().await?;

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_remote_https_with_git_suffix() {
        let (owner, repo) =
            parse_github_remote("https://github.com/octocat/hello-world.git").unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "hello-world");
    }

    #[test]
    fn parse_github_remote_https_without_suffix() {
        let (owner, repo) = parse_github_remote("https://github.com/octocat/hello-world").unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "hello-world");
    }

    #[test]
    fn parse_github_remote_ssh_format() {
        let (owner, repo) = parse_github_remote("git@github.com:octocat/hello-world.git").unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "hello-world");
    }

    #[test]
    fn parse_github_remote_unknown_host_returns_none() {
        assert!(parse_github_remote("https://gitlab.com/owner/repo.git").is_none());
    }

    #[test]
    fn parse_github_remote_malformed_returns_none() {
        assert!(parse_github_remote("not-a-url").is_none());
    }
}
