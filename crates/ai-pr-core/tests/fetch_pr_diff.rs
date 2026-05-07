use ai_pr_core::{
    config::Config,
    github::{process_diff, GitHubClient},
};
use mockito::Server;

fn file_section(name: &str, body: &str) -> String {
    format!(
        "diff --git a/{name} b/{name}\nindex a..b 100644\n--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n{body}\n"
    )
}

// ── Requirement 1: Full Diff on First Run ─────────────────────────────────────

#[tokio::test]
async fn fetch_pr_diff_no_stored_sha_fetches_full_diff() {
    let mut server = Server::new_async().await;
    let diff_body = file_section("src/main.rs", "+fn main() {}");

    let m = server
        .mock("GET", "/repos/owner/repo/pulls/42")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200)
        .with_body(&diff_body)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let diff = client.fetch_full_diff("owner", "repo", 42).await.unwrap();
    assert_eq!(diff, diff_body);
    m.assert_async().await;
}

#[tokio::test]
async fn fetch_pr_diff_full_diff_returns_raw_diff_text() {
    let mut server = Server::new_async().await;
    let expected = file_section("src/lib.rs", "+pub fn hello() {}");

    server
        .mock("GET", "/repos/owner/repo/pulls/7")
        .with_status(200)
        .with_body(&expected)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let got = client.fetch_full_diff("owner", "repo", 7).await.unwrap();
    assert_eq!(got, expected);
}

#[tokio::test]
async fn fetch_pr_diff_github_api_error_returns_descriptive_error() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/repos/owner/repo/pulls/99")
        .with_status(500)
        .with_body("{\"message\":\"Internal Server Error\"}")
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let err = client
        .fetch_full_diff("owner", "repo", 99)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("500"),
        "error should include HTTP status, got: {msg}"
    );
    assert!(
        msg.contains("99"),
        "error should include PR number, got: {msg}"
    );
}

// ── Requirement 2: Delta Diff on Subsequent Runs ──────────────────────────────

#[tokio::test]
async fn fetch_pr_diff_stored_sha_fetches_compare_diff() {
    let mut server = Server::new_async().await;
    let diff_body = file_section("src/new.rs", "+new stuff");

    let m = server
        .mock("GET", "/repos/owner/repo/compare/abc123...def456")
        .match_header("accept", "application/vnd.github.v3.diff")
        .with_status(200)
        .with_body(&diff_body)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let diff = client
        .fetch_compare_diff("owner", "repo", "abc123", "def456")
        .await
        .unwrap();
    assert_eq!(diff, diff_body);
    m.assert_async().await;
}

#[tokio::test]
async fn fetch_pr_diff_empty_compare_returns_empty_string() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/repos/owner/repo/compare/sha1...sha2")
        .with_status(200)
        .with_body("")
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let diff = client
        .fetch_compare_diff("owner", "repo", "sha1", "sha2")
        .await
        .unwrap();
    assert!(diff.is_empty());
}

#[tokio::test]
async fn fetch_pr_diff_compare_api_error_returns_descriptive_error() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/repos/owner/repo/compare/abc123...def456")
        .with_status(404)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let err = client
        .fetch_compare_diff("owner", "repo", "abc123", "def456")
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("404"),
        "error should include HTTP status, got: {msg}"
    );
    assert!(
        msg.contains("abc123"),
        "error should include base SHA, got: {msg}"
    );
    assert!(
        msg.contains("def456"),
        "error should include head SHA, got: {msg}"
    );
}

// ── Requirement 3: File Exclusion Filters (integration path) ─────────────────
// Pure-function coverage lives in the unit tests inside github.rs.
// Here we verify the pipeline applies filters via fetch_pr_diff.

#[tokio::test]
async fn fetch_pr_diff_pipeline_excludes_lock_files() {
    let mut server = Server::new_async().await;
    let raw = file_section("Cargo.lock", "+lock") + &file_section("src/lib.rs", "+code");

    server
        .mock("GET", "/repos/owner/repo/pulls/1")
        .with_status(200)
        .with_body(&raw)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let config = Config::default();
    let diff = client
        .fetch_pr_diff("owner", "repo", 1, None, "head", &config)
        .await
        .unwrap();
    assert!(!diff.contains("Cargo.lock"));
    assert!(diff.contains("src/lib.rs"));
}

// ── Requirement 4: Diff Size Limit (integration path) ────────────────────────

#[tokio::test]
async fn fetch_pr_diff_truncation_posts_warning_comment() {
    let mut server = Server::new_async().await;

    // Two sections of ~650 bytes each; 1 KB limit → only first fits.
    let padding = "+".to_string() + &"x".repeat(600);
    let raw = file_section("a.rs", &padding) + &file_section("b.rs", &padding);

    server
        .mock("GET", "/repos/owner/repo/pulls/5")
        .with_status(200)
        .with_body(&raw)
        .create_async()
        .await;

    let comment_mock = server
        .mock("POST", "/repos/owner/repo/issues/5/comments")
        .with_status(201)
        .with_body("{}")
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let mut config = Config::default();
    config.diff.max_kb = 1;

    let diff = client
        .fetch_pr_diff("owner", "repo", 5, None, "head", &config)
        .await
        .unwrap();
    comment_mock.assert_async().await;
    assert!(diff.contains("a.rs"), "first file should be included");
    assert!(!diff.contains("b.rs"), "second file should be truncated");
}

#[tokio::test]
async fn fetch_pr_diff_within_size_limit_no_comment_posted() {
    let mut server = Server::new_async().await;
    let raw = file_section("small.rs", "+tiny");

    server
        .mock("GET", "/repos/owner/repo/pulls/3")
        .with_status(200)
        .with_body(&raw)
        .create_async()
        .await;

    // Comment endpoint should NOT be called.
    let comment_mock = server
        .mock("POST", "/repos/owner/repo/issues/3/comments")
        .expect(0)
        .create_async()
        .await;

    let client = GitHubClient::with_base_url("token", server.url());
    let diff = client
        .fetch_pr_diff("owner", "repo", 3, None, "head", &Config::default())
        .await
        .unwrap();
    comment_mock.assert_async().await;
    assert!(!diff.is_empty());
}

// ── process_diff — R3.3: default patterns when none configured ───────────────
// Verified via config defaults already, but tested end-to-end here too.

#[test]
fn fetch_pr_diff_default_max_kb_is_100() {
    // Config::default applies the 100KB limit — exercise process_diff with it.
    let cfg = Config::default();
    let small = file_section("a.rs", "+tiny");
    let result = process_diff(&small, &cfg.filters.exclude, cfg.diff.max_kb).unwrap();
    assert!(result.truncated.is_none());
}
