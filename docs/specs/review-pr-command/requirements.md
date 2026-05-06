# Feature 9 — /review-pr Slash Command

## Overview

An MCP tool exposed by `ai-pr-mcp` that triggers the full two-pass review on a PR from within a Claude Code session, without needing CI to run. Local only — renders findings inline in Claude Code and writes to `.ai-reviews/<branch>-review.md`. Nothing is posted to GitHub.

## Requirements

### Requirement 1: Trigger Two-Pass Review

**User Story:** As a developer, I want to trigger a full two-pass review from Claude Code so that I can get AI feedback on a PR without waiting for CI.

#### Acceptance Criteria

1. The tool SHALL accept a PR number as a required input. Repo owner and name SHALL be inferred from the local git remote if not provided.
2. WHEN called, the tool SHALL execute Pass 1 (summary) and Pass 2 (inline findings) using the configured providers and models from `ai-pr-review.yml`.
3. WHEN `ai-pr-review.yml` is absent, the tool SHALL proceed with default configuration values.
4. WHEN Pass 1 completes, the tool SHALL not wait for Pass 2 before returning intermediate output — it SHALL stream or return Pass 1 results immediately, then Pass 2 results when ready.
5. WHEN either pass fails, the tool SHALL return a descriptive error identifying which pass failed and why, and include any results from the pass that succeeded.

---

### Requirement 2: Structured Markdown Output

**User Story:** As a developer, I want findings returned as structured Markdown grouped by severity so that I can quickly triage feedback in my editor.

#### Acceptance Criteria

1. The tool SHALL return findings grouped into three sections: **High**, **Medium**, and **Low** severity.
2. Each finding SHALL include: file path, line number, severity, and comment body.
3. The Pass 1 summary SHALL be returned as a top-level section before the grouped findings.
4. WHEN no findings exist for a severity level, that section SHALL be omitted from the output.
5. The output SHALL be valid Markdown renderable inline in a Claude Code session.

---

### Requirement 3: Write Review to File

**User Story:** As a developer, I want the review written to `.ai-reviews/<branch>-review.md` so that I can reference it in my editor without re-running the tool.

#### Acceptance Criteria

1. The tool SHALL write the full review output (Pass 1 summary + all findings) to `.ai-reviews/<branch>-review.md` relative to the current working directory.
2. WHEN the `.ai-reviews/` directory does not exist, the tool SHALL create it.
3. WHEN a review file already exists for the branch, the tool SHALL overwrite it.
4. The file write SHALL not block the tool from returning output to the Claude Code session — both SHALL complete.
5. The tool SHALL return the path of the written file alongside the review output.
