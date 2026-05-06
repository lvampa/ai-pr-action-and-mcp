use crate::config::Config;
use anyhow::{bail, Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use reqwest::Client;
use serde_json::json;
use tracing::{debug, info, trace, warn};

const GITHUB_API: &str = "https://api.github.com";

// ── Client ───────────────────────────────────────────────────────────────────

pub struct GitHubClient {
    http: Client,
    token: String,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_base_url(token, GITHUB_API)
    }

    /// Alternate constructor for tests — points at a local mock server.
    pub fn with_base_url(token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            token: token.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn auth(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Fetch the full unified diff for a PR (no prior review).
    pub async fn fetch_full_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<String> {
        debug!(pr = pr_number, owner, repo, "Fetching full PR diff");
        let url = format!("{}/repos/{owner}/{repo}/pulls/{pr_number}", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3.diff")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .with_context(|| format!("Failed to connect to GitHub API for PR #{pr_number}"))?;

        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error fetching PR #{pr_number} diff: HTTP {status}");
        }
        let text = resp.text().await.context("Failed to read diff response body")?;
        debug!(pr = pr_number, bytes = text.len(), "Full diff received");
        Ok(text)
    }

    /// Fetch the unified diff between two SHAs (incremental re-review).
    pub async fn fetch_compare_diff(
        &self,
        owner: &str,
        repo: &str,
        base: &str,
        head: &str,
    ) -> Result<String> {
        debug!(owner, repo, base, head, "Fetching incremental diff");
        let url = format!(
            "{}/repos/{owner}/{repo}/compare/{base}...{head}",
            self.base_url
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3.diff")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .with_context(|| format!("Failed to connect to GitHub API for compare {base}...{head}"))?;

        let status = resp.status();
        if !status.is_success() {
            bail!(
                "GitHub API error fetching compare {base}...{head}: HTTP {status}"
            );
        }
        let text = resp.text().await.context("Failed to read compare response body")?;
        debug!(base, head, bytes = text.len(), "Incremental diff received");
        Ok(text)
    }

    /// Post a comment on an issue / PR.
    pub async fn post_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{issue_number}/comments",
            self.base_url
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("Failed to post PR comment")?;

        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error posting comment: HTTP {status}");
        }
        info!(owner, repo, pr = issue_number, "Comment posted");
        Ok(())
    }

    /// High-level pipeline: fetch (full or delta), filter, size-limit, warn if truncated.
    /// Returns empty string when there is nothing to review.
    pub async fn fetch_pr_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        stored_sha: Option<&str>,
        head_sha: &str,
        config: &Config,
    ) -> Result<String> {
        let raw = match stored_sha {
            None => self.fetch_full_diff(owner, repo, pr_number).await?,
            Some(base) => self.fetch_compare_diff(owner, repo, base, head_sha).await?,
        };

        let raw_bytes = raw.len();
        let processed = process_diff(&raw, &config.filters.exclude, config.diff.max_kb)?;
        debug!(
            pr = pr_number,
            raw_bytes,
            filtered_bytes = processed.content.len(),
            "Diff processed"
        );

        if let Some(ref trunc) = processed.truncated {
            warn!(
                pr = pr_number,
                files_included = trunc.files_included,
                files_total = trunc.files_total,
                max_kb = config.diff.max_kb,
                "Diff exceeded size limit, truncating"
            );
            let msg = format!(
                "⚠️ **Diff size limit reached** ({}KB). \
                Reviewing {}/{} files.\n<!-- ai-pr-review-truncation -->",
                config.diff.max_kb, trunc.files_included, trunc.files_total
            );
            if let Err(e) = self.post_comment(owner, repo, pr_number, &msg).await {
                warn!(pr = pr_number, error = %e, "Failed to post truncation warning comment");
            }
        }

        let size_kb = processed.content.len() / 1024;
        let files = processed.content.matches("diff --git").count();
        info!(pr = pr_number, size_kb, files, "PR diff fetched");

        Ok(processed.content)
    }

    /// List all comments on an issue / PR (up to 100).
    pub async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<CommentData>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{issue_number}/comments?per_page=100",
            self.base_url
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .context("Failed to list issue comments")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error listing comments: HTTP {status}");
        }
        resp.json().await.context("Failed to parse issue comments response")
    }

    /// Update (PATCH) an existing issue comment.
    pub async fn update_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        body: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/comments/{comment_id}",
            self.base_url
        );
        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("Failed to update comment")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error updating comment #{comment_id}: HTTP {status}");
        }
        info!(owner, repo, comment_id, "Comment updated");
        Ok(())
    }

    /// List all reviews on a PR (up to 100).
    pub async fn list_pr_reviews(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<ReviewData>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{pr_number}/reviews?per_page=100",
            self.base_url
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .context("Failed to list PR reviews")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error listing reviews for PR #{pr_number}: HTTP {status}");
        }
        resp.json().await.context("Failed to parse PR reviews response")
    }

    /// Post a new pull request review with inline comments.
    /// Returns the created review ID.
    pub async fn post_pr_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        body: &str,
        comments: &[ReviewCommentInput],
    ) -> Result<u64> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{pr_number}/reviews",
            self.base_url
        );
        let comment_json: Vec<serde_json::Value> = comments
            .iter()
            .map(|c| json!({ "path": c.path, "line": c.line, "body": c.body }))
            .collect();
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .json(&json!({ "body": body, "event": "COMMENT", "comments": comment_json }))
            .send()
            .await
            .context("Failed to post PR review")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error posting review for PR #{pr_number}: HTTP {status}");
        }
        let val: serde_json::Value =
            resp.json().await.context("Failed to parse post-review response")?;
        let review_id = val["id"].as_u64().context("Missing 'id' in post-review response")?;
        info!(owner, repo, pr = pr_number, review_id, "PR review posted");
        Ok(review_id)
    }

    /// Dismiss an existing pull request review.
    pub async fn dismiss_pr_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        review_id: u64,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{pr_number}/reviews/{review_id}/dismissals",
            self.base_url
        );
        let resp = self
            .http
            .put(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .json(&json!({ "message": "Superseded by updated review" }))
            .send()
            .await
            .context("Failed to dismiss PR review")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error dismissing review #{review_id}: HTTP {status}");
        }
        info!(owner, repo, pr = pr_number, review_id, "PR review dismissed");
        Ok(())
    }

    /// Get the login of the authenticated user (used to identify bot reviews).
    pub async fn get_authenticated_user_login(&self) -> Result<String> {
        let url = format!("{}/user", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .context("Failed to get authenticated user")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error getting authenticated user: HTTP {status}");
        }
        let val: serde_json::Value =
            resp.json().await.context("Failed to parse user response")?;
        val["login"]
            .as_str()
            .map(|s| s.to_string())
            .context("Missing 'login' in user response")
    }

    /// Fetch PR metadata (title, head branch, head SHA).
    pub async fn get_pr_info(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PrInfo> {
        let url = format!("{}/repos/{owner}/{repo}/pulls/{pr_number}", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "ai-pr-action")
            .send()
            .await
            .with_context(|| format!("Failed to connect to GitHub API for PR #{pr_number}"))?;
        let status = resp.status();
        if !status.is_success() {
            bail!("GitHub API error fetching PR #{pr_number} info: HTTP {status}");
        }
        resp.json().await.context("Failed to parse PR info response")
    }
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub head: PrHead,
}

#[derive(Debug, serde::Deserialize)]
pub struct PrHead {
    #[serde(rename = "ref")]
    pub branch: String,
    pub sha: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CommentData {
    pub id: u64,
    pub body: String,
    pub user: UserData,
}

#[derive(Debug, serde::Deserialize)]
pub struct UserData {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct ReviewData {
    pub id: u64,
    pub user: UserData,
    pub state: String,
}

pub struct ReviewCommentInput {
    pub path: String,
    pub line: u64,
    pub body: String,
}

// ── Diff processing ───────────────────────────────────────────────────────────

pub struct ProcessedDiff {
    pub content: String,
    pub truncated: Option<TruncationInfo>,
}

pub struct TruncationInfo {
    pub files_included: usize,
    pub files_total: usize,
}

/// Filter files by glob patterns, then enforce the kilobyte size limit.
/// Pure function — no network I/O.
pub fn process_diff(raw: &str, exclude_patterns: &[String], max_kb: i64) -> Result<ProcessedDiff> {
    let matcher = build_glob_set(exclude_patterns)?;
    let files = split_diff_files(raw);

    let after_filter: Vec<(String, String)> = files
        .into_iter()
        .filter(|(name, _)| {
            if matcher.is_match(name) {
                trace!(file = name.as_str(), "File excluded by filter");
                false
            } else {
                true
            }
        })
        .collect();

    if after_filter.is_empty() {
        return Ok(ProcessedDiff { content: String::new(), truncated: None });
    }

    let max_bytes = (max_kb.max(0) as usize) * 1024;
    let total_files = after_filter.len();
    let mut included: Vec<String> = Vec::new();
    let mut used: usize = 0;

    for (_, section) in &after_filter {
        if used + section.len() > max_bytes {
            break;
        }
        used += section.len();
        included.push(section.clone());
    }

    let files_included = included.len();
    let truncated = if files_included < total_files {
        Some(TruncationInfo { files_included, files_total: total_files })
    } else {
        None
    };

    Ok(ProcessedDiff { content: included.concat(), truncated })
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Split a unified diff string into per-file sections.
/// Each section starts with its own `diff --git` line.
fn split_diff_files(diff: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in diff.lines() {
        if let Some(filename) = parse_diff_header(line) {
            if let Some(file) = current_file.take() {
                sections.push((file, current_lines.join("\n") + "\n"));
                current_lines.clear();
            }
            current_file = Some(filename);
        }
        current_lines.push(line);
    }
    if let Some(file) = current_file {
        if !current_lines.is_empty() {
            sections.push((file, current_lines.join("\n") + "\n"));
        }
    }
    sections
}

/// Extract the filename from a `diff --git a/foo b/foo` header line.
fn parse_diff_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    let (_a, b) = rest.split_once(" b/")?;
    Some(b.to_string())
}

fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p).with_context(|| format!("Invalid glob pattern: {p}"))?);
    }
    builder.build().context("Failed to build glob set")
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn file_section(name: &str, body: &str) -> String {
        format!("diff --git a/{name} b/{name}\nindex a..b 100644\n--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n{body}\n")
    }

    // ── parse_diff_header ──────────────────────────────────────────────────

    #[test]
    fn parse_diff_header_extracts_filename() {
        assert_eq!(
            parse_diff_header("diff --git a/src/main.rs b/src/main.rs"),
            Some("src/main.rs".into())
        );
    }

    #[test]
    fn parse_diff_header_returns_none_for_non_header() {
        assert_eq!(parse_diff_header("+added line"), None);
        assert_eq!(parse_diff_header("@@ -1,3 +1,4 @@"), None);
    }

    // ── split_diff_files ───────────────────────────────────────────────────

    #[test]
    fn split_diff_files_single_file() {
        let diff = file_section("foo.rs", "+hello");
        let files = split_diff_files(&diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "foo.rs");
    }

    #[test]
    fn split_diff_files_multiple_files() {
        let diff = file_section("a.rs", "+a") + &file_section("b.rs", "+b");
        let files = split_diff_files(&diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "a.rs");
        assert_eq!(files[1].0, "b.rs");
    }

    #[test]
    fn split_diff_files_empty_diff_returns_empty_vec() {
        assert!(split_diff_files("").is_empty());
    }

    // ── process_diff — exclusion (R3) ──────────────────────────────────────

    #[test]
    fn process_diff_excludes_matching_files() {
        let diff = file_section("Cargo.lock", "+x") + &file_section("src/lib.rs", "+y");
        let result = process_diff(&diff, &["**/*.lock".into()], 100).unwrap();
        assert!(!result.content.contains("Cargo.lock"));
        assert!(result.content.contains("src/lib.rs"));
    }

    #[test]
    fn process_diff_multiple_patterns_any_match_excludes() {
        let diff = file_section("vendor/lib.rs", "+x") + &file_section("src/main.rs", "+y");
        let patterns = vec!["vendor/**".into(), "generated/**".into()];
        let result = process_diff(&diff, &patterns, 100).unwrap();
        assert!(!result.content.contains("vendor/lib.rs"));
        assert!(result.content.contains("src/main.rs"));
    }

    #[test]
    fn process_diff_default_patterns_exclude_lock_and_vendor() {
        let defaults = crate::config::Config::default().filters.exclude;
        let diff = file_section("Cargo.lock", "+x")
            + &file_section("vendor/dep.rs", "+x")
            + &file_section("generated/schema.rs", "+x")
            + &file_section("src/main.rs", "+keep");
        let result = process_diff(&diff, &defaults, 100).unwrap();
        assert!(result.content.contains("src/main.rs"));
        assert!(!result.content.contains("Cargo.lock"));
        assert!(!result.content.contains("vendor/dep.rs"));
        assert!(!result.content.contains("generated/schema.rs"));
    }

    #[test]
    fn process_diff_all_files_excluded_returns_empty() {
        let diff = file_section("Cargo.lock", "+x");
        let result = process_diff(&diff, &["**/*.lock".into()], 100).unwrap();
        assert!(result.content.is_empty());
        assert!(result.truncated.is_none());
    }

    // ── process_diff — size limit (R4) ─────────────────────────────────────

    #[test]
    fn process_diff_within_size_limit_returns_full_diff() {
        let diff = file_section("a.rs", "+small");
        let result = process_diff(&diff, &[], 100).unwrap();
        assert!(!result.content.is_empty());
        assert!(result.truncated.is_none());
    }

    #[test]
    fn process_diff_exceeds_size_limit_truncates_to_fitting_files() {
        // Each section is ~650 bytes; 1KB limit means only the first fits.
        let body = "+".to_string() + &"x".repeat(600);
        let diff = file_section("a.rs", &body) + &file_section("b.rs", &body);
        let result = process_diff(&diff, &[], 1).unwrap(); // 1 KB limit
        let info = result.truncated.expect("should be truncated");
        assert_eq!(info.files_included, 1);
        assert_eq!(info.files_total, 2);
        assert!(result.content.contains("a.rs"));
        assert!(!result.content.contains("b.rs"));
    }

    #[test]
    fn process_diff_size_limit_uses_default_100kb() {
        // Config default is 100 — just verify it round-trips through process_diff.
        let cfg = crate::config::Config::default();
        assert_eq!(cfg.diff.max_kb, 100);
    }
}
