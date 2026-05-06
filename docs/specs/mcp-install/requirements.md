# Feature 11 — MCP Server Install Command

## Overview

`ai-pr-mcp install` is a subcommand that sets up the MCP server in one step. It registers the server with Claude Code via `claude mcp add` and writes `.claude/commands/` slash command files to the current project directory. After running it, `/review-pr` and `/apply-review` work as typed slash commands in Claude Code.

## Requirements

### Requirement 1: Register MCP Server with Claude Code

**User Story:** As a developer, I want `ai-pr-mcp install` to register the server with Claude Code so that I don't have to manually edit JSON config.

#### Acceptance Criteria

1. WHEN `ai-pr-mcp install` is run, the binary SHALL invoke `claude mcp add ai-pr-review <path-to-binary>` with the env vars from the current environment.
2. The binary path used SHALL be the absolute path of the currently running `ai-pr-mcp` binary.
3. WHEN `GITHUB_TOKEN` is present in the environment, it SHALL be passed to `claude mcp add` via `-e GITHUB_TOKEN=...`.
4. WHEN `ANTHROPIC_API_KEY` is present in the environment, it SHALL be passed via `-e ANTHROPIC_API_KEY=...`.
5. WHEN `OPENAI_API_KEY` is present in the environment, it SHALL be passed via `-e OPENAI_API_KEY=...`.
6. WHEN the `claude` CLI is not found on PATH, the binary SHALL return a descriptive error with a link to Claude Code installation docs.
7. WHEN `claude mcp add` fails, the binary SHALL return the error output from the CLI and exit with a non-zero code.
8. WHEN `claude mcp add` succeeds, the binary SHALL print a confirmation message.

---

### Requirement 2: Write Slash Command Files

**User Story:** As a developer, I want slash command files written to `.claude/commands/` so that `/review-pr` and `/apply-review` work as typed commands in Claude Code.

#### Acceptance Criteria

1. WHEN `ai-pr-mcp install` is run, the binary SHALL write the following files to `.claude/commands/` relative to the current working directory:
   - `review-pr.md`
   - `apply-review.md`
2. WHEN `.claude/commands/` does not exist, the binary SHALL create it.
3. WHEN a command file already exists, the binary SHALL overwrite it.
4. Each command file SHALL contain a prompt that instructs Claude Code to call the corresponding MCP tool (`review_pr`, `apply_review`) with the arguments provided by the user.
5. The `review-pr.md` command SHALL format output as grouped high/medium/low severity Markdown sections.
6. The `apply-review.md` command SHALL instruct Claude to present the plan before applying and ask for confirmation.
7. WHEN command files are written successfully, the binary SHALL print the paths of the files created.

---

### Requirement 3: Install Summary

**User Story:** As a developer, I want a clear summary after install so that I know exactly what was set up and how to use it.

#### Acceptance Criteria

1. WHEN install completes successfully, the binary SHALL print a summary listing:
   - The MCP server name registered (`ai-pr-review`)
   - The command files written
   - The slash commands now available (`/review-pr`, `/apply-review`)
   - A note to restart Claude Code for changes to take effect.
2. WHEN any step fails, the binary SHALL indicate which step failed and which steps succeeded.
