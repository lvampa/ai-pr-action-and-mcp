use ai_pr_core::inline::{filter_valid_findings, parse_findings};

fn diff_with_hunk(file: &str, new_start: u64, new_count: u64) -> String {
    format!(
        "diff --git a/{file} b/{file}\nindex a..b 100644\n--- a/{file}\n+++ b/{file}\n\
         @@ -{new_start},{new_count} +{new_start},{new_count} @@\n context\n"
    )
}

// ── Requirement 1: Parse Model Findings ──────────────────────────────────────

#[test]
fn post_inline_comments_parse_valid_json_returns_findings() {
    let json = r#"[{"file":"src/main.rs","line":5,"severity":"high","comment":"bug here"}]"#;
    let findings = parse_findings(json).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].file, "src/main.rs");
    assert_eq!(findings[0].line, 5);
    assert_eq!(findings[0].severity, "high");
    assert_eq!(findings[0].comment, "bug here");
}

#[test]
fn post_inline_comments_parse_empty_array_returns_empty_vec() {
    let findings = parse_findings("[]").unwrap();
    assert!(findings.is_empty());
}

#[test]
fn post_inline_comments_invalid_json_returns_error() {
    let err = parse_findings("not json at all").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("JSON") || msg.contains("parse"),
        "error should describe JSON parse failure, got: {msg}"
    );
}

#[test]
fn post_inline_comments_missing_field_returns_error() {
    // Missing "severity" field
    let json = r#"[{"file":"src/main.rs","line":5,"comment":"bug"}]"#;
    let err = parse_findings(json).unwrap_err();
    assert!(
        err.to_string().contains("severity") || err.to_string().contains("JSON"),
        "error should mention missing field"
    );
}

#[test]
fn post_inline_comments_finding_file_not_in_diff_is_skipped() {
    let findings = parse_findings(
        r#"[{"file":"other.rs","line":1,"severity":"low","comment":"note"}]"#,
    )
    .unwrap();
    let diff = diff_with_hunk("src/main.rs", 1, 5);
    let valid = filter_valid_findings(findings, &diff);
    assert!(valid.is_empty());
}

#[test]
fn post_inline_comments_finding_line_outside_hunk_is_skipped() {
    let findings = parse_findings(
        r#"[{"file":"src/main.rs","line":99,"severity":"low","comment":"note"}]"#,
    )
    .unwrap();
    // Hunk covers lines 1-5 only
    let diff = diff_with_hunk("src/main.rs", 1, 5);
    let valid = filter_valid_findings(findings, &diff);
    assert!(valid.is_empty());
}

#[test]
fn post_inline_comments_finding_within_hunk_is_kept() {
    let findings = parse_findings(
        r#"[{"file":"src/main.rs","line":3,"severity":"medium","comment":"note"}]"#,
    )
    .unwrap();
    let diff = diff_with_hunk("src/main.rs", 1, 5);
    let valid = filter_valid_findings(findings, &diff);
    assert_eq!(valid.len(), 1);
}

#[test]
fn post_inline_comments_all_findings_skipped_completes_without_error() {
    let findings = parse_findings(
        r#"[{"file":"missing.rs","line":1,"severity":"low","comment":"note"}]"#,
    )
    .unwrap();
    let diff = diff_with_hunk("src/main.rs", 1, 5);
    let valid = filter_valid_findings(findings, &diff);
    // No error — empty result is valid
    assert!(valid.is_empty());
}

#[test]
fn post_inline_comments_multiple_findings_mixed_validity() {
    let json = r#"[
        {"file":"src/main.rs","line":2,"severity":"high","comment":"keep"},
        {"file":"src/main.rs","line":99,"severity":"low","comment":"skip line"},
        {"file":"other.rs","line":1,"severity":"low","comment":"skip file"}
    ]"#;
    let findings = parse_findings(json).unwrap();
    let diff = diff_with_hunk("src/main.rs", 1, 5);
    let valid = filter_valid_findings(findings, &diff);
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].comment, "keep");
}

// ── Requirement 2+3: Posting reviews is covered in two_pass_review.rs ────────
// (integration tests there verify POST /reviews and dismissal via the full run_review pipeline)
