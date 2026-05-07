use crate::{github::PrInfo, inline::Finding};
use anyhow::{Context, Result};
use std::path::Path;

const FINDINGS_PREFIX: &str = "<!-- findings-json: ";
const FINDINGS_SUFFIX: &str = " -->";

// ── Format ────────────────────────────────────────────────────────────────────

/// Build the full Markdown review document (summary + grouped findings).
/// Embeds findings as a machine-readable JSON comment for round-trip parsing.
pub fn format_review_markdown(pr_info: &PrInfo, summary: &str, findings: &[Finding]) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# AI Review — PR #{}: {}\n\n**Branch:** `{}` | **HEAD SHA:** `{}`\n\n",
        pr_info.number, pr_info.title, pr_info.head.branch, pr_info.head.sha
    ));

    out.push_str("## Summary\n\n");
    out.push_str(summary);
    out.push_str("\n\n");

    let high: Vec<&Finding> = findings.iter().filter(|f| f.severity == "high").collect();
    let medium: Vec<&Finding> = findings.iter().filter(|f| f.severity == "medium").collect();
    let low: Vec<&Finding> = findings.iter().filter(|f| f.severity == "low").collect();

    if high.is_empty() && medium.is_empty() && low.is_empty() {
        out.push_str("## Findings\n\n_No findings._\n\n");
    } else {
        out.push_str("## Findings\n\n");
        if !high.is_empty() {
            out.push_str("### 🔴 High\n\n");
            for f in &high {
                out.push_str(&format!(
                    "- **`{}` line {}**: {}\n",
                    f.file, f.line, f.comment
                ));
            }
            out.push('\n');
        }
        if !medium.is_empty() {
            out.push_str("### 🟡 Medium\n\n");
            for f in &medium {
                out.push_str(&format!(
                    "- **`{}` line {}**: {}\n",
                    f.file, f.line, f.comment
                ));
            }
            out.push('\n');
        }
        if !low.is_empty() {
            out.push_str("### 🔵 Low\n\n");
            for f in &low {
                out.push_str(&format!(
                    "- **`{}` line {}**: {}\n",
                    f.file, f.line, f.comment
                ));
            }
            out.push('\n');
        }
    }

    let json = serde_json::to_string(findings).unwrap_or_default();
    out.push_str(&format!("{FINDINGS_PREFIX}{json}{FINDINGS_SUFFIX}\n"));

    out
}

// ── Write ─────────────────────────────────────────────────────────────────────

/// Write review Markdown to `.ai-reviews/<branch>-review.md`.
/// Returns the path written.
pub fn write_review(root: &Path, branch: &str, content: &str) -> Result<String> {
    let dir = root.join(".ai-reviews");
    std::fs::create_dir_all(&dir)?;
    let safe_branch = branch.replace('/', "-");
    let path = dir.join(format!("{safe_branch}-review.md"));
    std::fs::write(&path, content)?;
    Ok(path.to_string_lossy().into_owned())
}

// ── Parse ─────────────────────────────────────────────────────────────────────

/// Extract findings embedded as JSON in the review Markdown.
pub fn parse_review_findings(content: &str) -> Result<Vec<Finding>> {
    let start = content
        .find(FINDINGS_PREFIX)
        .context("No findings marker found in review file — was it written by ai-pr-mcp?")?
        + FINDINGS_PREFIX.len();
    let end = content[start..]
        .find(FINDINGS_SUFFIX)
        .context("Malformed findings marker in review file")?
        + start;
    serde_json::from_str(&content[start..end]).context("Failed to parse embedded findings JSON")
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::PrHead;

    fn pr_info() -> PrInfo {
        PrInfo {
            number: 7,
            title: "Test PR".into(),
            head: PrHead {
                branch: "feat/test".into(),
                sha: "deadbeef".into(),
            },
        }
    }

    fn finding(file: &str, line: u64, severity: &str, comment: &str) -> Finding {
        Finding {
            file: file.into(),
            line,
            severity: severity.into(),
            comment: comment.into(),
        }
    }

    #[test]
    fn format_review_markdown_contains_pr_metadata() {
        let md = format_review_markdown(&pr_info(), "looks good", &[]);
        assert!(md.contains("PR #7"));
        assert!(md.contains("Test PR"));
        assert!(md.contains("feat/test"));
        assert!(md.contains("deadbeef"));
    }

    #[test]
    fn format_review_markdown_contains_summary() {
        let md = format_review_markdown(&pr_info(), "no issues found", &[]);
        assert!(md.contains("no issues found"));
    }

    #[test]
    fn format_review_markdown_groups_findings_by_severity() {
        let findings = vec![
            finding("a.rs", 1, "high", "critical bug"),
            finding("b.rs", 2, "low", "minor nit"),
            finding("c.rs", 3, "medium", "consider refactoring"),
        ];
        let md = format_review_markdown(&pr_info(), "summary", &findings);
        let high_pos = md.find("🔴 High").unwrap();
        let med_pos = md.find("🟡 Medium").unwrap();
        let low_pos = md.find("🔵 Low").unwrap();
        assert!(high_pos < med_pos && med_pos < low_pos);
        assert!(md.contains("critical bug"));
        assert!(md.contains("consider refactoring"));
        assert!(md.contains("minor nit"));
    }

    #[test]
    fn format_review_markdown_omits_empty_severity_sections() {
        let findings = vec![finding("a.rs", 1, "high", "bug")];
        let md = format_review_markdown(&pr_info(), "summary", &findings);
        assert!(md.contains("🔴 High"));
        assert!(!md.contains("🟡 Medium"));
        assert!(!md.contains("🔵 Low"));
    }

    #[test]
    fn format_review_markdown_no_findings_shows_no_findings_message() {
        let md = format_review_markdown(&pr_info(), "all good", &[]);
        assert!(md.contains("No findings"));
        assert!(!md.contains("🔴"));
    }

    #[test]
    fn parse_review_findings_roundtrip() {
        let findings = vec![
            finding("src/main.rs", 10, "high", "potential panic"),
            finding("src/lib.rs", 5, "low", "use clippy suggestion"),
        ];
        let md = format_review_markdown(&pr_info(), "summary", &findings);
        let parsed = parse_review_findings(&md).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].file, "src/main.rs");
        assert_eq!(parsed[0].line, 10);
        assert_eq!(parsed[0].severity, "high");
        assert_eq!(parsed[1].file, "src/lib.rs");
    }

    #[test]
    fn parse_review_findings_empty_array_roundtrip() {
        let md = format_review_markdown(&pr_info(), "all clear", &[]);
        let parsed = parse_review_findings(&md).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_review_findings_missing_marker_returns_error() {
        let err = parse_review_findings("# Some markdown without the marker").unwrap_err();
        assert!(err.to_string().contains("findings marker"));
    }

    #[test]
    fn write_review_creates_file_with_correct_name() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_review(dir.path(), "feature/my-branch", "content").unwrap();
        assert!(std::path::Path::new(&path).exists());
        assert!(path.contains("feature-my-branch-review.md"));
    }

    #[test]
    fn write_review_overwrites_existing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        write_review(dir.path(), "main", "old").unwrap();
        write_review(dir.path(), "main", "new").unwrap();
        let path = dir.path().join(".ai-reviews/main-review.md");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new");
    }
}
