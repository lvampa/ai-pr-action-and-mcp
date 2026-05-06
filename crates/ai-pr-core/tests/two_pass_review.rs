use ai_pr_core::{
    config::Config,
    github::GitHubClient,
    model::ModelProvider,
    review::run_review,
};
use async_trait::async_trait;
use mockito::Server;
use serde_json::json;

// ── Mock provider helpers ─────────────────────────────────────────────────────

struct OkProvider(String);

#[async_trait]
impl ModelProvider for OkProvider {
    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        Ok(self.0.clone())
    }
}

struct FailProvider;

#[async_trait]
impl ModelProvider for FailProvider {
    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        anyhow::bail!("model error")
    }
}

// Inline provider returns a single valid finding for `src/lib.rs` line 2.
fn inline_json() -> String {
    r#"[{"file":"src/lib.rs","line":2,"severity":"low","comment":"nitpick"}]"#.to_string()
}

// Minimal diff: file `src/lib.rs`, hunk starting at line 1 with 3 lines.
fn diff_body() -> String {
    "diff --git a/src/lib.rs b/src/lib.rs\nindex a..b 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,3 @@\n context\n+added\n context\n".to_string()
}

fn user_json() -> &'static str {
    r#"{"login":"github-actions[bot]","id":1}"#
}

// ── Requirement 1: Pass 1 — Summary Comment ───────────────────────────────────

#[tokio::test]
async fn two_pass_review_no_existing_comment_creates_new_summary() {
    let mut server = Server::new_async().await;

    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1")
        .with_status(200).with_body(diff_body()).create_async().await;
    let post_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":101}"#).create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("POST", "/repos/o/r/pulls/1/reviews")
        .with_status(200).with_body(r#"{"id":201}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    post_m.assert_async().await;
}

#[tokio::test]
async fn two_pass_review_existing_summary_updates_in_place() {
    let mut server = Server::new_async().await;

    let existing_body = "old summary\n<!-- ai-pr-review -->\n<!-- sha: oldsha -->";
    let comments_json = json!([{"id":55,"body":existing_body,"user":{"login":"bot","id":1}}]).to_string();
    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body(comments_json).create_async().await;
    server.mock("GET", "/repos/o/r/compare/oldsha...headsha")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200).with_body(diff_body()).create_async().await;
    let patch_m = server.mock("PATCH", "/repos/o/r/issues/comments/55")
        .with_status(200).with_body("{}").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("POST", "/repos/o/r/pulls/1/reviews")
        .with_status(200).with_body(r#"{"id":201}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("new summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    patch_m.assert_async().await;
}


#[tokio::test]
async fn two_pass_review_pass1_failure_posts_error_comment() {
    let mut server = Server::new_async().await;

    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1")
        .with_status(200).with_body(diff_body()).create_async().await;
    let err_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":101}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    let result = run_review(&client, "o", "r", 1, "headsha",
        &FailProvider, &OkProvider(inline_json()),
        &Config::default()).await;
    assert!(result.is_err(), "expected Err for pass 1 failure");
    err_m.assert_async().await;
}

#[tokio::test]
async fn two_pass_review_pass1_failure_leaves_existing_summary_unchanged() {
    let mut server = Server::new_async().await;

    let existing_body = "old summary\n<!-- ai-pr-review -->\n<!-- sha: oldsha -->";
    let comments_json = json!([{"id":55,"body":existing_body,"user":{"login":"bot","id":1}}]).to_string();
    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body(comments_json).create_async().await;
    server.mock("GET", "/repos/o/r/compare/oldsha...headsha")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200).with_body(diff_body()).create_async().await;
    // PATCH must NOT be called
    let patch_m = server.mock("PATCH", "/repos/o/r/issues/comments/55")
        .expect(0).create_async().await;
    // Error comment posted instead
    let err_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":102}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    let result = run_review(&client, "o", "r", 1, "headsha",
        &FailProvider, &OkProvider(inline_json()),
        &Config::default()).await;
    assert!(result.is_err(), "expected Err for pass 1 failure");
    patch_m.assert_async().await;
    err_m.assert_async().await;
}

// ── Requirement 2: Pass 2 — Inline Comments ───────────────────────────────────

#[tokio::test]
async fn two_pass_review_pass2_dismisses_previous_review() {
    let mut server = Server::new_async().await;

    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1")
        .with_status(200).with_body(diff_body()).create_async().await;
    server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":101}"#).create_async().await;
    // Previous review by the bot
    let reviews_json = r#"[{"id":77,"user":{"login":"github-actions[bot]","id":1},"state":"COMMENTED"}]"#;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body(reviews_json).create_async().await;
    server.mock("GET", "/user")
        .with_status(200).with_body(user_json()).create_async().await;
    let dismiss_m = server.mock("PUT", "/repos/o/r/pulls/1/reviews/77/dismissals")
        .with_status(200).with_body("{}").create_async().await;
    server.mock("POST", "/repos/o/r/pulls/1/reviews")
        .with_status(200).with_body(r#"{"id":201}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    dismiss_m.assert_async().await;
}

#[tokio::test]
async fn two_pass_review_no_previous_review_skips_dismissal() {
    let mut server = Server::new_async().await;

    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1")
        .with_status(200).with_body(diff_body()).create_async().await;
    server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":101}"#).create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    // GET /user must NOT be called (no reviews to check)
    let user_m = server.mock("GET", "/user").expect(0).create_async().await;
    server.mock("POST", "/repos/o/r/pulls/1/reviews")
        .with_status(200).with_body(r#"{"id":201}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    user_m.assert_async().await;
}

#[tokio::test]
async fn two_pass_review_pass2_failure_posts_error_and_preserves_summary() {
    let mut server = Server::new_async().await;

    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1")
        .with_status(200).with_body(diff_body()).create_async().await;
    // Summary is posted successfully
    let summary_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":101}"#).create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    // Error comment for Pass 2 failure
    let err_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .with_status(201).with_body(r#"{"id":102}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    let result = run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("summary".into()), &FailProvider,
        &Config::default()).await;
    assert!(result.is_err(), "expected Err for pass 2 failure");
    summary_m.assert_async().await;
    err_m.assert_async().await;
}

// ── Requirement 3: SHA-Based Incremental Review ───────────────────────────────

#[tokio::test]
async fn two_pass_review_empty_delta_exits_without_comment() {
    let mut server = Server::new_async().await;

    let existing_body = "summary\n<!-- ai-pr-review -->\n<!-- sha: abc123 -->";
    let comments_json = json!([{"id":55,"body":existing_body,"user":{"login":"bot","id":1}}]).to_string();
    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body(comments_json).create_async().await;
    server.mock("GET", "/repos/o/r/compare/abc123...newsha")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200).with_body("").create_async().await;
    // No comments should be posted
    let post_m = server.mock("POST", "/repos/o/r/issues/1/comments")
        .expect(0).create_async().await;
    let patch_m = server.mock("PATCH", "/repos/o/r/issues/comments/55")
        .expect(0).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "newsha",
        &OkProvider("summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    post_m.assert_async().await;
    patch_m.assert_async().await;
}

// ── Requirement 4: Force Re-Review ───────────────────────────────────────────

#[tokio::test]
async fn two_pass_review_force_review_ignores_stored_sha() {
    let mut server = Server::new_async().await;

    // Comments include existing summary (with sha) AND a re-review request
    let existing_body = "summary\n<!-- ai-pr-review -->\n<!-- sha: oldsha -->";
    let comments_json = json!([
        {"id":55,"body":existing_body,"user":{"login":"bot","id":1}},
        {"id":56,"body":"@bot re-review","user":{"login":"user","id":2}}
    ]).to_string();
    server.mock("GET", "/repos/o/r/issues/1/comments?per_page=100")
        .with_status(200).with_body(comments_json).create_async().await;
    // Should fetch FULL diff (not compare), because force=true → stored_sha=None
    server.mock("GET", "/repos/o/r/pulls/1")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200).with_body(diff_body()).create_async().await;
    // Should update existing summary comment
    let patch_m = server.mock("PATCH", "/repos/o/r/issues/comments/55")
        .with_status(200).with_body("{}").create_async().await;
    server.mock("GET", "/repos/o/r/pulls/1/reviews?per_page=100")
        .with_status(200).with_body("[]").create_async().await;
    server.mock("POST", "/repos/o/r/pulls/1/reviews")
        .with_status(200).with_body(r#"{"id":201}"#).create_async().await;

    let client = GitHubClient::with_base_url("tok", server.url());
    run_review(&client, "o", "r", 1, "headsha",
        &OkProvider("summary".into()), &OkProvider(inline_json()),
        &Config::default()).await.unwrap();

    patch_m.assert_async().await;
}
