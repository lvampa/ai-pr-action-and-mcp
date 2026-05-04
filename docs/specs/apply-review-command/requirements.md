# Feature 10 — /apply-review Slash Command

## Overview

An MCP tool exposed by `ai-pr-mcp` that reads `<branch>-review-plan.md` from `.ai-reviews/`, applies the suggested changes to the local codebase, and commits them to the current branch.

## Requirements

### Requirement 1: Read Review Plan

**User Story:** As a developer, I want the tool to read the review plan for the current branch so that I don't have to specify the file manually.

#### Acceptance Criteria

1. The tool SHALL infer the current branch name from the local git state.
2. The tool SHALL read `.ai-reviews/<branch>-review-plan.md` relative to the current working directory.
3. WHEN the plan file does not exist for the current branch, the tool SHALL return a descriptive error suggesting the user run `/review-pr` or the plan synthesis tool first.
4. WHEN the plan file exists but is empty, the tool SHALL return a descriptive error and make no changes.

---

### Requirement 2: Apply Suggested Changes

**User Story:** As a developer, I want the tool to apply the changes described in the review plan so that I don't have to manually implement each suggestion.

#### Acceptance Criteria

1. The tool SHALL parse the review plan to extract actionable file changes (file path, change description, and suggested content or diff).
2. WHEN a suggested change targets a file that exists, the tool SHALL apply the change to that file.
3. WHEN a suggested change targets a file that does not exist, the tool SHALL create the file with the suggested content.
4. WHEN a suggested change cannot be applied cleanly (e.g., the file has changed since the plan was written), the tool SHALL skip that change, log a warning identifying the file, and continue applying remaining changes.
5. The tool SHALL return a summary of applied changes, skipped changes, and any warnings.

---

### Requirement 3: Commit Changes

**User Story:** As a developer, I want the applied changes committed to the current branch so that the work is saved and ready to push.

#### Acceptance Criteria

1. WHEN at least one change is applied successfully, the tool SHALL stage all modified files and create a git commit on the current branch.
2. The commit message SHALL reference the PR number and indicate the changes were applied from the AI review plan (e.g., `Apply AI review suggestions for PR #<number>`).
3. WHEN no changes are applied (all skipped), the tool SHALL not create an empty commit.
4. WHEN the git commit fails, the tool SHALL return a descriptive error and leave the working tree in its modified state (changes applied but not committed).
5. The tool SHALL NOT push the commit — pushing is left to the developer.

---

### Requirement 4: Safety Guards

**User Story:** As a developer, I want the tool to check for uncommitted changes before applying so that I don't accidentally lose work.

#### Acceptance Criteria

1. WHEN the working tree has uncommitted changes, the tool SHALL warn the user and require an explicit `force: true` parameter to proceed.
2. WHEN `force: true` is provided, the tool SHALL proceed without further prompting.
3. WHEN `force: false` (default) and uncommitted changes exist, the tool SHALL return a descriptive error listing the modified files and exit without making any changes.
