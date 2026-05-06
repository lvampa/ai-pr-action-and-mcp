use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::warn;

#[derive(Debug, Clone, serde::Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub file: String,
    pub line: u64,
    pub severity: String,
    pub comment: String,
}

/// Parse model output as a JSON array of findings.
pub fn parse_findings(json: &str) -> Result<Vec<Finding>> {
    serde_json::from_str(json).context("Failed to parse model output as JSON findings array")
}

/// Keep only findings whose (file, line) pair appears in the diff hunks.
pub fn filter_valid_findings(findings: Vec<Finding>, diff: &str) -> Vec<Finding> {
    let valid = build_valid_lines(diff);
    findings
        .into_iter()
        .filter(|f| match valid.get(&f.file) {
            None => {
                warn!(file = f.file.as_str(), "Skipping finding: file not in diff");
                false
            }
            Some(lines) if !lines.contains(&f.line) => {
                warn!(file = f.file.as_str(), line = f.line, "Skipping finding: line not in diff hunk");
                false
            }
            _ => true,
        })
        .collect()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Build a map of filename → set of line numbers present in the diff hunks.
fn build_valid_lines(diff: &str) -> HashMap<String, HashSet<u64>> {
    let mut result: HashMap<String, HashSet<u64>> = HashMap::new();
    let mut current_file: Option<String> = None;

    for line in diff.lines() {
        if let Some(filename) = parse_file_header(line) {
            current_file = Some(filename);
        } else if let Some((new_start, new_count)) = parse_hunk_header(line) {
            if let Some(file) = &current_file {
                let set = result.entry(file.clone()).or_default();
                for l in new_start..new_start + new_count {
                    set.insert(l);
                }
            }
        }
    }
    result
}

fn parse_file_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    let (_a, b) = rest.split_once(" b/")?;
    Some(b.to_string())
}

/// Parse `@@ -old +new_start[,new_count] @@` and return `(new_start, new_count)`.
fn parse_hunk_header(line: &str) -> Option<(u64, u64)> {
    let rest = line.strip_prefix("@@ ")?;
    let plus_pos = rest.find(" +")?;
    let new_part = &rest[plus_pos + 2..];
    let end = new_part.find([' ', '@']).unwrap_or(new_part.len());
    let new_range = &new_part[..end];
    if let Some(comma) = new_range.find(',') {
        let start: u64 = new_range[..comma].parse().ok()?;
        let count: u64 = new_range[comma + 1..].parse().ok()?;
        Some((start, count))
    } else {
        let start: u64 = new_range.parse().ok()?;
        Some((start, 1))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn diff_with_hunk(file: &str, new_start: u64, new_count: u64) -> String {
        format!(
            "diff --git a/{file} b/{file}\nindex a..b 100644\n--- a/{file}\n+++ b/{file}\n@@ -{new_start},{new_count} +{new_start},{new_count} @@\n"
        )
    }

    #[test]
    fn parse_hunk_header_with_count() {
        assert_eq!(parse_hunk_header("@@ -1,3 +10,5 @@"), Some((10, 5)));
    }

    #[test]
    fn parse_hunk_header_without_count_defaults_to_one() {
        assert_eq!(parse_hunk_header("@@ -1 +7 @@"), Some((7, 1)));
    }

    #[test]
    fn parse_hunk_header_returns_none_for_non_header() {
        assert_eq!(parse_hunk_header("+added line"), None);
    }

    #[test]
    fn build_valid_lines_single_file() {
        let diff = diff_with_hunk("src/main.rs", 5, 3);
        let map = build_valid_lines(&diff);
        let lines = map.get("src/main.rs").unwrap();
        assert!(lines.contains(&5));
        assert!(lines.contains(&6));
        assert!(lines.contains(&7));
        assert!(!lines.contains(&8));
    }
}
