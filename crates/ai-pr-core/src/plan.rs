use crate::github::{CommentData, PrInfo};
use std::path::Path;

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Extract all `@bot`-prefixed comment bodies.
pub fn extract_bot_commands(comments: &[CommentData]) -> Vec<String> {
    comments
        .iter()
        .filter(|c| c.body.starts_with("@bot"))
        .map(|c| c.body.clone())
        .collect()
}

/// Extract unique changed file names from a unified diff.
pub fn extract_changed_files(diff: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some((_, b)) = rest.split_once(" b/") {
                if !files.contains(&b.to_string()) {
                    files.push(b.to_string());
                }
            }
        }
    }
    files
}

/// Build a Markdown review plan from PR metadata, diff, and comments.
pub fn build_review_plan(info: &PrInfo, diff: &str, comments: &[CommentData]) -> String {
    let bot_commands = extract_bot_commands(comments);
    let changed_files = extract_changed_files(diff);
    let force_rereview = bot_commands.iter().any(|c| c.contains("re-review"));

    let mut out = String::new();

    out.push_str(&format!(
        "# Review Plan — PR #{}\n\n## PR Metadata\n\n- **Title:** {}\n- **Branch:** `{}`\n- **HEAD SHA:** `{}`\n\n",
        info.number, info.title, info.head.branch, info.head.sha
    ));

    if force_rereview {
        out.push_str("> ⚠️ **Full re-review requested** (`@bot re-review` detected — stored SHA will be cleared)\n\n");
    }

    out.push_str("## Changed Files\n\n");
    if changed_files.is_empty() {
        out.push_str("_(no changed files detected)_\n\n");
    } else {
        for f in &changed_files {
            out.push_str(&format!("- `{f}`\n"));
        }
        out.push('\n');
    }

    if !bot_commands.is_empty() {
        out.push_str("## Reviewer Instructions\n\n");
        for cmd in &bot_commands {
            out.push_str(&format!("> {cmd}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Suggested Focus Areas\n\n");
    if changed_files.is_empty() {
        out.push_str("_(no diff to analyze)_\n");
    } else {
        let src_files: Vec<&str> = changed_files
            .iter()
            .map(|s| s.as_str())
            .filter(|f| f.ends_with(".rs") || f.ends_with(".ts") || f.ends_with(".py") || f.ends_with(".go"))
            .collect();
        let config_files: Vec<&str> = changed_files
            .iter()
            .map(|s| s.as_str())
            .filter(|f| {
                f.ends_with(".toml") || f.ends_with(".yaml") || f.ends_with(".yml") || f.ends_with(".json")
            })
            .collect();

        if !src_files.is_empty() {
            out.push_str("- Logic and correctness in changed source files\n");
        }
        if !config_files.is_empty() {
            out.push_str("- Configuration changes and their downstream effects\n");
        }
        out.push_str("- Error handling and edge cases\n");
        out.push_str("- Test coverage for new or modified behaviour\n");
    }

    out
}

/// Write the plan to `.ai-reviews/<branch>-review-plan.md` relative to `root`.
/// Returns the path written.
pub fn write_plan(root: &Path, branch: &str, content: &str) -> anyhow::Result<String> {
    let dir = root.join(".ai-reviews");
    std::fs::create_dir_all(&dir)?;
    let safe_branch = branch.replace('/', "-");
    let path = dir.join(format!("{safe_branch}-review-plan.md"));
    std::fs::write(&path, content)?;
    Ok(path.to_string_lossy().into_owned())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{PrHead, UserData};

    fn comment(body: &str) -> CommentData {
        CommentData { id: 1, body: body.to_string(), user: UserData { login: "u".into(), id: 1 } }
    }

    fn pr_info() -> PrInfo {
        PrInfo {
            number: 42,
            title: "My PR".into(),
            head: PrHead { branch: "feature/x".into(), sha: "abc123".into() },
        }
    }

    #[test]
    fn extract_bot_commands_returns_only_at_bot_prefixed() {
        let comments = vec![
            comment("regular comment"),
            comment("@bot focus on error handling"),
            comment("@bot re-review"),
        ];
        let cmds = extract_bot_commands(&comments);
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("focus on error handling"));
        assert!(cmds[1].contains("re-review"));
    }

    #[test]
    fn extract_bot_commands_case_sensitive_prefix() {
        let comments = vec![comment("@Bot re-review"), comment("@bot re-review")];
        let cmds = extract_bot_commands(&comments);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("@bot"));
    }

    #[test]
    fn extract_bot_commands_empty_when_none_present() {
        let comments = vec![comment("looks good"), comment("nit: style")];
        assert!(extract_bot_commands(&comments).is_empty());
    }

    #[test]
    fn extract_changed_files_returns_unique_filenames() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\ndiff --git a/Cargo.toml b/Cargo.toml\n";
        let files = extract_changed_files(diff);
        assert_eq!(files, vec!["src/lib.rs", "Cargo.toml"]);
    }

    #[test]
    fn extract_changed_files_empty_diff_returns_empty() {
        assert!(extract_changed_files("").is_empty());
    }

    #[test]
    fn build_review_plan_contains_pr_metadata() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n";
        let plan = build_review_plan(&pr_info(), diff, &[]);
        assert!(plan.contains("PR #42"));
        assert!(plan.contains("My PR"));
        assert!(plan.contains("feature/x"));
        assert!(plan.contains("abc123"));
    }

    #[test]
    fn build_review_plan_lists_changed_files() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n";
        let plan = build_review_plan(&pr_info(), diff, &[]);
        assert!(plan.contains("src/lib.rs"));
    }

    #[test]
    fn build_review_plan_includes_bot_commands_section() {
        let comments = vec![comment("@bot focus on security")];
        let plan = build_review_plan(&pr_info(), "", &comments);
        assert!(plan.contains("Reviewer Instructions"));
        assert!(plan.contains("focus on security"));
    }

    #[test]
    fn build_review_plan_no_bot_commands_omits_section() {
        let plan = build_review_plan(&pr_info(), "", &[]);
        assert!(!plan.contains("Reviewer Instructions"));
    }

    #[test]
    fn build_review_plan_force_rereview_adds_prominent_note() {
        let comments = vec![comment("@bot re-review")];
        let plan = build_review_plan(&pr_info(), "", &comments);
        assert!(plan.contains("Full re-review requested"));
    }

    #[test]
    fn write_plan_creates_directory_and_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_plan(dir.path(), "feature/my-branch", "# plan").unwrap();
        assert!(std::path::Path::new(&path).exists());
        assert!(path.contains("feature-my-branch-review-plan.md"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "# plan");
    }

    #[test]
    fn write_plan_overwrites_existing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        write_plan(dir.path(), "main", "old").unwrap();
        write_plan(dir.path(), "main", "new").unwrap();
        let path = dir.path().join(".ai-reviews/main-review-plan.md");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new");
    }
}
