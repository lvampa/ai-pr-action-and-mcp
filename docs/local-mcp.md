# Local MCP Setup

The MCP server lets you run reviews and apply suggestions from inside a Claude Code session — no CI run needed.

## Install

Build the binary and run the install command. It registers the MCP server with Claude Code and writes the slash command files in one step:

```bash
cargo build -p ai-pr-mcp

# Set your keys first
export GITHUB_TOKEN=ghp_...
export ANTHROPIC_API_KEY=sk-ant-...   # or OPENAI_API_KEY

./target/debug/ai-pr-mcp install
```

This runs `claude mcp add` to register the server and writes `.claude/commands/review-pr.md` and `.claude/commands/apply-review.md` to the current project directory.

Restart Claude Code after installing.

## Manual setup (alternative)

If you prefer to configure manually:

```bash
claude mcp add ai-pr-review /absolute/path/to/ai-pr-mcp \
  -e GITHUB_TOKEN=ghp_... \
  -e ANTHROPIC_API_KEY=sk-ant-...
```

Then write the `.claude/commands/` files yourself — see the command file format in the spec at `docs/specs/mcp-install/requirements.md`.

## Prerequisites

- Rust toolchain (stable) — see `rust-toolchain.toml`
- A GitHub personal access token with `repo` scope
- An Anthropic or OpenAI API key

## Build

```bash
cargo build -p ai-pr-mcp
```

The binary lands at `target/debug/ai-pr-mcp`.

## Environment

```bash
cp .env.example .env
# fill in GITHUB_TOKEN and ANTHROPIC_API_KEY (or OPENAI_API_KEY)
```

See [environment.md](environment.md) for the full variable reference.

## Add to Claude Code

Run the install command — it handles registration and slash command setup in one step:

```bash
./target/debug/ai-pr-mcp install
```

See the [Install](#install) section above for full details.

## Slash commands

### `/review-pr <pr-number>`

Runs the full two-pass review on a PR. Fetches the diff from GitHub, runs Pass 1 (summary) then Pass 2 (inline findings). Results render inline in Claude Code grouped by severity.

Also writes the full review to `.ai-reviews/<branch>-review.md` for editor reference.

Nothing is posted to GitHub — this is a local review only.

```
/review-pr 142
```

Repo owner and name are inferred from the local git remote. You can also pass them explicitly if needed.

### `/apply-review`

Reads the review findings written by `/review-pr`, builds an actionable plan, and presents it before touching anything.

```
/apply-review

Plan for PR #142 (3 actionable changes):
  1. src/auth/middleware.rs  line 84 — regenerate session ID after privilege escalation
  2. src/auth/middleware.rs  line 102 — fix timing leak on error path
  3. src/cache.rs  line 42 — include locale in cache key

Apply these changes? [y/N]
```

On confirmation, applies the changes and commits them to the current branch. Does not push — that's left to you.

Warns if you have uncommitted work. Pass `force: true` to proceed anyway.

The plan is also written to `.ai-reviews/<branch>-review-plan.md`.

## Output files

All output goes to `.ai-reviews/` in the current working directory. This directory is git-ignored.

| File | Written by | Contents |
|------|-----------|----------|
| `<branch>-review.md` | `/review-pr` | Full review — summary + findings grouped by severity |
| `<branch>-review-plan.md` | `/apply-review` | Actionable plan derived from review findings |

## Configuration

The MCP server reads `ai-pr-review.yml` from the current working directory, same as the GitHub Action. See [ci-setup.md](ci-setup.md#configuration) for the full schema.
