use ai_pr_core::config::load_config;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, content: &str) {
    fs::write(dir.path().join("ai-pr-review.yml"), content).unwrap();
}

// ── Requirement 1: Load Configuration File ────────────────────────────────────

#[test]
fn config_loader_valid_yaml_parses_to_config() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        "summary:\n  provider: openai\n  model: gpt-4\n",
    );
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.summary.provider, "openai");
    assert_eq!(cfg.summary.model, "gpt-4");
}

#[test]
fn config_loader_missing_file_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.summary.provider, "anthropic");
    assert_eq!(cfg.summary.model, "claude-haiku-4-5");
    assert_eq!(cfg.inline.provider, "anthropic");
    assert_eq!(cfg.inline.model, "claude-opus-4-5");
    assert_eq!(
        cfg.filters.exclude,
        vec!["**/*.lock", "vendor/**", "generated/**"]
    );
    assert_eq!(cfg.diff.max_kb, 100);
    assert_eq!(cfg.rate_limiting.retries, 3);
    assert_eq!(cfg.rate_limiting.backoff_seconds, 5);
}

#[test]
fn config_loader_yaml_syntax_error_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "summary: [\nbad yaml");
    let err = load_config(dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ai-pr-review.yml"),
        "error should identify the file, got: {msg}"
    );
}

#[test]
fn config_loader_unknown_key_is_ignored() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        "totally_unknown_section:\n  foo: bar\nsummary:\n  provider: anthropic\n",
    );
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.summary.provider, "anthropic");
}

// ── Requirement 2: Summary and Inline Model Configuration ─────────────────────

#[test]
fn config_loader_summary_provider_defaults_to_anthropic() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "summary:\n  model: my-model\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.summary.provider, "anthropic");
}

#[test]
fn config_loader_summary_model_defaults_to_haiku() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "summary:\n  provider: anthropic\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.summary.model, "claude-haiku-4-5");
}

#[test]
fn config_loader_inline_provider_defaults_to_anthropic() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "inline:\n  model: my-model\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.inline.provider, "anthropic");
}

#[test]
fn config_loader_inline_model_defaults_to_opus() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "inline:\n  provider: anthropic\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.inline.model, "claude-opus-4-5");
}

#[test]
fn config_loader_invalid_summary_provider_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "summary:\n  provider: gemini\n");
    let err = load_config(dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("gemini"), "error should name the invalid value, got: {msg}");
    assert!(msg.contains("anthropic"), "error should list valid values, got: {msg}");
}

#[test]
fn config_loader_invalid_inline_provider_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "inline:\n  provider: cohere\n");
    let err = load_config(dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("cohere"), "error should name the invalid value, got: {msg}");
}

// ── Requirement 3: File Exclusion Filters ─────────────────────────────────────

#[test]
fn config_loader_custom_exclude_patterns_are_used() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "filters:\n  exclude:\n    - \"dist/**\"\n    - \"*.min.js\"\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.filters.exclude, vec!["dist/**", "*.min.js"]);
}

#[test]
fn config_loader_exclude_absent_applies_defaults() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "diff:\n  max_kb: 50\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(
        cfg.filters.exclude,
        vec!["**/*.lock", "vendor/**", "generated/**"]
    );
}

#[test]
fn config_loader_empty_exclude_list_applies_no_exclusions() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "filters:\n  exclude: []\n");
    let cfg = load_config(dir.path()).unwrap();
    assert!(cfg.filters.exclude.is_empty());
}

// ── Requirement 4: Diff Size Limit ────────────────────────────────────────────

#[test]
fn config_loader_custom_max_kb_is_used() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "diff:\n  max_kb: 250\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.diff.max_kb, 250);
}

#[test]
fn config_loader_max_kb_absent_defaults_to_100() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "summary:\n  provider: anthropic\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.diff.max_kb, 100);
}

#[test]
fn config_loader_diff_size_zero_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "diff:\n  max_kb: 0\n");
    let err = load_config(dir.path()).unwrap_err();
    assert!(err.to_string().contains("max_kb"));
}

#[test]
fn config_loader_diff_size_negative_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "diff:\n  max_kb: -10\n");
    let err = load_config(dir.path()).unwrap_err();
    assert!(err.to_string().contains("max_kb"));
}

// ── Requirement 5: Rate Limiting Configuration ────────────────────────────────

#[test]
fn config_loader_retries_absent_defaults_to_3() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "rate_limiting:\n  backoff_seconds: 10\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.rate_limiting.retries, 3);
}

#[test]
fn config_loader_backoff_absent_defaults_to_5() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "rate_limiting:\n  retries: 5\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.rate_limiting.backoff_seconds, 5);
}

#[test]
fn config_loader_negative_retries_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "rate_limiting:\n  retries: -1\n");
    let err = load_config(dir.path()).unwrap_err();
    assert!(err.to_string().contains("retries"));
}

#[test]
fn config_loader_negative_backoff_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "rate_limiting:\n  backoff_seconds: -5\n");
    let err = load_config(dir.path()).unwrap_err();
    assert!(err.to_string().contains("backoff_seconds"));
}

// ── Requirement 2b: Review Instructions File ──────────────────────────────────

#[test]
fn config_loader_no_instructions_file_uses_builtin_default() {
    let dir = TempDir::new().unwrap();
    // No review-instructions.md, no review_instructions key
    let cfg = load_config(dir.path()).unwrap();
    assert!(!cfg.instructions.is_empty(), "instructions should be populated from built-in default");
    assert!(cfg.instructions.contains("Correctness"), "built-in instructions should mention Correctness");
}

#[test]
fn config_loader_default_review_instructions_file_is_used_when_present() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("review-instructions.md"), "focus on memory safety").unwrap();
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.instructions.trim(), "focus on memory safety");
}

#[test]
fn config_loader_custom_review_instructions_path_is_used() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("my-instructions.md"), "check all error paths").unwrap();
    write_config(&dir, "review_instructions: my-instructions.md\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.instructions.trim(), "check all error paths");
}

#[test]
fn config_loader_missing_custom_instructions_file_returns_error() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "review_instructions: nonexistent.md\n");
    let err = load_config(dir.path()).unwrap_err();
    assert!(
        err.to_string().contains("nonexistent.md"),
        "error should name the missing file, got: {err}"
    );
}

#[test]
fn config_loader_custom_path_takes_precedence_over_default_file() {
    let dir = TempDir::new().unwrap();
    // Both exist — the configured path should win
    fs::write(dir.path().join("review-instructions.md"), "default file").unwrap();
    fs::write(dir.path().join("custom.md"), "custom file").unwrap();
    write_config(&dir, "review_instructions: custom.md\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.instructions.trim(), "custom file");
}

// ── Requirement 6: Public API ─────────────────────────────────────────────────

#[test]
fn config_loader_returns_fully_resolved_config() {
    // Partial file — every unset field should be filled with defaults.
    let dir = TempDir::new().unwrap();
    write_config(&dir, "diff:\n  max_kb: 200\n");
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.diff.max_kb, 200);
    assert_eq!(cfg.summary.provider, "anthropic");
    assert_eq!(cfg.inline.model, "claude-opus-4-5");
    assert_eq!(cfg.rate_limiting.retries, 3);
}

#[test]
fn config_loader_error_identifies_failure_source() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, ": bad\n");
    let err = load_config(dir.path()).unwrap_err();
    // Should identify the file path in the error chain.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("ai-pr-review.yml"),
        "error chain should identify the file, got: {chain}"
    );
}
