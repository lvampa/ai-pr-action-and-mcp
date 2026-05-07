# Feature 10 — /apply-review Slash Command

## Overview

An MCP tool exposed by `ai-pr-mcp` that reads the review findings from `/review-pr` (stored in `.ai-reviews/<branch>-review.md`), synthesizes them into an actionable plan, applies the suggested changes to the local codebase, and commits them to the current branch.

## Requirements

### Requirement 1: Read Review Findings

**User Story:** As a developer, I want the tool to read the review findings for the current branch so that I don't have to specify the file manually.

#### Acceptance Criteria

1. The tool SHALL infer the current branch name from the local git state.
2. The tool SHALL read `.ai-reviews/<branch>-review.md` relative to the current working directory.
3. WHEN the review file does not exist for the current branch, the tool SHALL return a descriptive error suggesting the user run `/review-pr` first.
4. WHEN the review file exists but is empty, the tool SHALL return a descriptive error and make no changes.

---

### Requirement 2: Build and Present Actionable Plan

**User Story:** As a developer, I want the tool to synthesize review findings into a concrete plan and show it to me before touching any files so that I can decide whether to proceed.

#### Acceptance Criteria

1. The tool SHALL parse the review findings and produce an ordered list of actionable changes, each with a file path, description, and suggested content or diff.
2. The tool SHALL write the plan to `.ai-reviews/<branch>-review-plan.md` and return it to the MCP client.
3. The tool SHALL prompt the user to confirm before applying any changes (e.g., "Apply these N changes?").
4. WHEN the user does not confirm, the tool SHALL exit without modifying any files. The plan file SHALL remain on disk.
5. WHEN a finding is informational only (no actionable file change), the tool SHALL include it in the plan summary but not attempt to apply it.
6. WHEN no actionable changes are found, the tool SHALL return a descriptive message and exit without modifying any files.

---

### Requirement 3: Apply Suggested Changes

**User Story:** As a developer, I want the tool to apply the planned changes to my local codebase so that I don't have to implement each suggestion manually.

#### Acceptance Criteria

1. WHEN a planned change targets a file that exists, the tool SHALL apply the change to that file.
2. WHEN a planned change targets a file that does not exist, the tool SHALL create the file with the suggested content.
3. WHEN a planned change cannot be applied cleanly (e.g., the file has changed since the review was written), the tool SHALL skip that change, log a warning identifying the file, and continue applying remaining changes.
4. The tool SHALL return a summary of applied changes, skipped changes, and any warnings.

---

### Requirement 4: Commit Changes

**User Story:** As a developer, I want the applied changes committed to the current branch so that the work is saved and ready to push.

#### Acceptance Criteria

1. WHEN at least one change is applied successfully, the tool SHALL stage all modified files and create a git commit on the current branch.
2. The commit message SHALL reference the PR number and indicate the changes were applied from the AI review (e.g., `Apply AI review suggestions for PR #<number>`).
3. WHEN no changes are applied (all skipped), the tool SHALL not create an empty commit.
4. WHEN the git commit fails, the tool SHALL return a descriptive error and leave the working tree in its modified state (changes applied but not committed).
5. The tool SHALL NOT push the commit — pushing is left to the developer.

---

### Requirement 5: Safety Guards

**User Story:** As a developer, I want the tool to check for uncommitted changes before applying so that I don't accidentally lose work.

#### Acceptance Criteria

1. WHEN the working tree has uncommitted changes, the tool SHALL warn the user and require an explicit `force: true` parameter to proceed.
2. WHEN `force: true` is provided, the tool SHALL proceed without further prompting.
3. WHEN `force: false` (default) and uncommitted changes exist, the tool SHALL return a descriptive error listing the modified files and exit without making any changes.
