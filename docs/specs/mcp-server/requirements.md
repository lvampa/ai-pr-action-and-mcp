# Feature 8 — MCP Server

## Overview

A standalone MCP server binary (`ai-pr-mcp`) that fetches PR diffs and comments, reads `@bot`-tagged comments as reviewer intent, synthesizes everything into a structured review plan, and writes it to `.ai-reviews/<branch>-review-plan.md`. The `.ai-reviews/` directory is git-ignored. All local tools are exposed as MCP tools callable from Claude Code as slash commands.

## Requirements

### Requirement 1: Fetch PR Diff and Comments

**User Story:** As an MCP client, I want the server to fetch the PR diff and all comments so that the review plan has full context.

#### Acceptance Criteria

1. The MCP server SHALL expose a tool that accepts a PR number, repo owner, and repo name as inputs.
2. WHEN called, the server SHALL fetch the full PR diff using the GitHub pulls API.
3. WHEN called, the server SHALL fetch all PR comments (both review comments and issue comments) using the GitHub API.
4. WHEN the GitHub API returns an error, the server SHALL return a descriptive error to the MCP client including the HTTP status.
5. The server SHALL read the GitHub token from the `GITHUB_TOKEN` environment variable.
6. WHEN `GITHUB_TOKEN` is absent or empty, the server SHALL return a descriptive error before making any API calls.

---

### Requirement 2: Read @bot Commands

**User Story:** As a developer, I want `@bot`-prefixed comments treated as reviewer intent so that I can guide the AI review through natural PR comments.

#### Acceptance Criteria

1. The server SHALL identify all PR comments where the body starts with `@bot` (case-sensitive, literal string).
2. Each `@bot` comment SHALL be extracted and included in the review plan as a reviewer instruction.
3. WHEN a comment contains `@bot re-review`, the server SHALL include an explicit instruction in the plan to perform a full re-review (clear stored SHA).
4. WHEN no `@bot` comments exist, the server SHALL produce a plan with no reviewer instructions section.

---

### Requirement 3: Synthesize Review Plan

**User Story:** As a developer, I want the server to synthesize the diff and `@bot` comments into a structured plan so that I have a clear picture of what the AI will focus on.

#### Acceptance Criteria

1. The server SHALL produce a Markdown review plan containing: PR metadata (number, branch, HEAD SHA), a summary of changed files, any `@bot` reviewer instructions, and suggested review focus areas derived from the diff.
2. WHEN `@bot re-review` is present, the plan SHALL include a prominent note that a full re-review has been requested.
3. The plan SHALL be deterministic given the same inputs (no random content).

---

### Requirement 4: Write Plan to File

**User Story:** As a developer, I want the plan written to `.ai-reviews/<branch>-review-plan.md` so that it's accessible locally and ignored by git.

#### Acceptance Criteria

1. The server SHALL write the review plan to `.ai-reviews/<branch>-review-plan.md` relative to the current working directory, where `<branch>` is the PR's head branch name.
2. WHEN the `.ai-reviews/` directory does not exist, the server SHALL create it.
3. WHEN a plan file already exists for the branch, the server SHALL overwrite it.
4. The server SHALL return the path of the written file to the MCP client upon success.
5. The `.ai-reviews/` directory SHALL be documented as requiring a `.gitignore` entry (the server does not modify `.gitignore` itself).

---

### Requirement 5: MCP Tool Exposure

**User Story:** As a Claude Code user, I want all server tools exposed as MCP-callable slash commands so that I can trigger review operations directly from my editor session.

#### Acceptance Criteria

1. The server SHALL expose all local tools (plan synthesis, review trigger, apply review) as MCP tools conforming to the MCP tool schema (name, description, inputSchema).
2. Each tool SHALL be callable from Claude Code as a slash command without any additional configuration beyond adding the server to the MCP config.
3. WHEN a tool is called with missing required parameters, the server SHALL return a descriptive error identifying the missing fields.
4. The server SHALL run as a long-lived process and handle multiple sequential tool calls without restarting.
