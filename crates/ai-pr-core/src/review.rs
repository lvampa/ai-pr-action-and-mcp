use crate::{
    config::Config,
    github::{CommentData, GitHubClient, ReviewCommentInput},
    inline::{filter_valid_findings, parse_findings},
    model::ModelProvider,
};
use anyhow::Result;
use tracing::{debug, info, warn, error};

const SUMMARY_MARKER: &str = "<!-- ai-pr-review -->";
const SHA_PREFIX: &str = "<!-- sha: ";
const SHA_SUFFIX: &str = " -->";

const INLINE_FORMAT_INSTRUCTION: &str =
    "Return a JSON array of findings. \
     Each finding must have: \"file\" (string), \"line\" (integer), \
     \"severity\" (\"high\"|\"medium\"|\"low\"), \"comment\" (string). \
     Only reference files and lines present in the diff. \
     Return ONLY the JSON array, no other text.";

pub fn build_summary_prompt(instructions: &str, diff: &str) -> String {
    format!(
        "{instructions}\n\nProvide a concise Markdown summary of the changes, \
         potential issues, and overall code quality.\n\nDiff:\n{diff}"
    )
}

pub fn build_inline_prompt(instructions: &str, diff: &str) -> String {
    format!("{instructions}\n\n{INLINE_FORMAT_INSTRUCTION}\n\nDiff:\n{diff}")
}

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Extract the SHA from a `<!-- sha: {sha} -->` hidden marker.
pub fn extract_sha(body: &str) -> Option<String> {
    let start = body.find(SHA_PREFIX)? + SHA_PREFIX.len();
    let end = body[start..].find(SHA_SUFFIX)? + start;
    Some(body[start..end].trim().to_string())
}

/// Build the full summary comment body with both hidden markers appended.
pub fn build_summary_body(content: &str, sha: &str) -> String {
    format!("{content}\n\n{SUMMARY_MARKER}\n{SHA_PREFIX}{sha}{SHA_SUFFIX}")
}

/// Find the bot's existing summary comment by the `<!-- ai-pr-review -->` marker.
pub fn find_summary_comment(comments: &[CommentData]) -> Option<&CommentData> {
    comments.iter().find(|c| c.body.contains(SUMMARY_MARKER))
}

/// Return true if any comment contains the `@bot re-review` trigger.
pub fn check_force_review(comments: &[CommentData]) -> bool {
    comments.iter().any(|c| c.body.contains("@bot re-review"))
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Run a full two-pass review (summary + inline) for a pull request.
pub async fn run_review(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
    head_sha: &str,
    summary_provider: &dyn ModelProvider,
    inline_provider: &dyn ModelProvider,
    config: &Config,
) -> Result<()> {
    info!(pr = pr_number, owner, repo, "Starting review");

    // Load existing comments to check state.
    let comments = client.list_issue_comments(owner, repo, pr_number).await?;
    let existing_summary = find_summary_comment(&comments);
    let force_review = check_force_review(&comments);

    // Determine base SHA for incremental diff (None = full review).
    let stored_sha: Option<String> = if force_review {
        info!(pr = pr_number, "Force re-review requested, ignoring stored SHA");
        None
    } else {
        existing_summary.and_then(|c| extract_sha(&c.body))
    };

    debug!(pr = pr_number, incremental = stored_sha.is_some(), "Fetching diff");

    // Fetch the diff.
    let diff = client
        .fetch_pr_diff(owner, repo, pr_number, stored_sha.as_deref(), head_sha, config)
        .await?;

    // Nothing new to review.
    if diff.is_empty() {
        info!(pr = pr_number, "Empty delta — nothing new to review");
        return Ok(());
    }

    // ── Pass 1: summary ───────────────────────────────────────────────────────
    info!(pr = pr_number, pass = 1, "Starting summary pass");
    let summary_prompt = build_summary_prompt(&config.instructions, &diff);
    match summary_provider.complete(&summary_prompt).await {
        Ok(summary) => {
            let body = build_summary_body(&summary, head_sha);
            if let Some(existing) = existing_summary {
                client.update_comment(owner, repo, existing.id, &body).await?;
                info!(pr = pr_number, pass = 1, comment_id = existing.id, sha = head_sha, "Summary updated, SHA stored");
            } else {
                client.post_comment(owner, repo, pr_number, &body).await?;
                info!(pr = pr_number, pass = 1, sha = head_sha, "Summary posted, SHA stored");
            }
        }
        Err(e) => {
            error!(pr = pr_number, pass = 1, error = %e, "Summary pass failed");
            let msg = format!(
                "⚠️ **Pass 1 (summary) failed**: {e}\n<!-- ai-pr-review-error -->"
            );
            let _ = client.post_comment(owner, repo, pr_number, &msg).await;
            return Err(e.context("Pass 1 (summary) failed"));
        }
    }

    // ── Pass 2: inline comments ───────────────────────────────────────────────
    info!(pr = pr_number, pass = 2, "Starting inline pass");
    let inline_prompt = build_inline_prompt(&config.instructions, &diff);
    match inline_provider.complete(&inline_prompt).await {
        Ok(json) => {
            let findings = match parse_findings(&json) {
                Ok(f) => f,
                Err(e) => {
                    error!(pr = pr_number, pass = 2, error = %e, "Failed to parse inline findings");
                    let msg = format!(
                        "⚠️ **Pass 2 (inline) failed**: could not parse model output: {e}\n<!-- ai-pr-review-error -->"
                    );
                    let _ = client.post_comment(owner, repo, pr_number, &msg).await;
                    return Err(e.context("Pass 2 (inline) failed: invalid JSON"));
                }
            };
            let total_findings = findings.len();
            let valid = filter_valid_findings(findings, &diff);
            debug!(pr = pr_number, total = total_findings, valid = valid.len(), "Findings filtered");
            let review_comments: Vec<ReviewCommentInput> = valid
                .into_iter()
                .map(|f| ReviewCommentInput {
                    path: f.file,
                    line: f.line,
                    body: format!("**{}**: {}", f.severity, f.comment),
                })
                .collect();

            // Dismiss previous bot review if one exists.
            let reviews = client.list_pr_reviews(owner, repo, pr_number).await?;
            if !reviews.is_empty() {
                if let Ok(bot_login) = client.get_authenticated_user_login().await {
                    if let Some(review) = reviews
                        .iter()
                        .find(|r| r.user.login == bot_login && r.state != "DISMISSED")
                    {
                        if let Err(e) =
                            client.dismiss_pr_review(owner, repo, pr_number, review.id).await
                        {
                            warn!(pr = pr_number, review_id = review.id, error = %e, "Failed to dismiss previous review");
                        }
                    }
                }
            }

            if let Err(e) = client
                .post_pr_review(owner, repo, pr_number, "", &review_comments)
                .await
            {
                error!(pr = pr_number, pass = 2, error = %e, "Failed to post inline review");
                let msg = format!(
                    "⚠️ **Pass 2 (inline) failed**: could not post review: {e}\n<!-- ai-pr-review-error -->"
                );
                let _ = client.post_comment(owner, repo, pr_number, &msg).await;
                return Err(e.context("Pass 2 (inline) failed: could not post review"));
            }
            info!(pr = pr_number, pass = 2, comments = review_comments.len(), "Inline review posted");
        }
        Err(e) => {
            warn!(pr = pr_number, pass = 2, error = %e, "Inline pass failed, summary standing alone");
            let msg = format!(
                "⚠️ **Pass 2 (inline) failed**: {e}\n<!-- ai-pr-review-error -->"
            );
            let _ = client.post_comment(owner, repo, pr_number, &msg).await;
            return Err(e.context("Pass 2 (inline) failed"));
        }
    }

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::UserData;

    fn comment(id: u64, body: &str) -> CommentData {
        CommentData {
            id,
            body: body.to_string(),
            user: UserData { login: "user".into(), id: 1 },
        }
    }

    #[test]
    fn extract_sha_parses_marker() {
        let body = "some content\n<!-- sha: abc123def456 -->";
        assert_eq!(extract_sha(body), Some("abc123def456".to_string()));
    }

    #[test]
    fn extract_sha_returns_none_when_absent() {
        assert_eq!(extract_sha("no sha here"), None);
    }

    #[test]
    fn build_summary_body_contains_both_markers() {
        let body = build_summary_body("summary text", "deadbeef");
        assert!(body.contains(SUMMARY_MARKER));
        assert!(body.contains("<!-- sha: deadbeef -->"));
        assert!(body.contains("summary text"));
    }

    #[test]
    fn find_summary_comment_identifies_by_marker() {
        let comments = vec![
            comment(1, "regular comment"),
            comment(2, &format!("summary\n{SUMMARY_MARKER}")),
        ];
        let found = find_summary_comment(&comments).unwrap();
        assert_eq!(found.id, 2);
    }

    #[test]
    fn find_summary_comment_returns_none_when_absent() {
        let comments = vec![comment(1, "no marker here")];
        assert!(find_summary_comment(&comments).is_none());
    }

    #[test]
    fn check_force_review_detects_trigger() {
        let comments = vec![
            comment(1, "looks good"),
            comment(2, "@bot re-review please"),
        ];
        assert!(check_force_review(&comments));
    }

    #[test]
    fn check_force_review_returns_false_when_absent() {
        let comments = vec![comment(1, "looks good")];
        assert!(!check_force_review(&comments));
    }
}
