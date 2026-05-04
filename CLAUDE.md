# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                        # build all crates
cargo build --release              # release build (used for the committed binary)
cargo test                         # all tests
cargo test -p core                 # single crate
cargo test config_loader_diff_size # specific test by name prefix
cargo clippy -- -D warnings        # lint (CI enforces this)
cargo fmt --check                  # format check
```

## Workspace Layout

```
crates/
  core/           # shared library: config, diff fetcher, model interface, two-pass review, comment posting
  ai-pr-action/   # GitHub Actions binary — reads env vars, invokes core, exits
  ai-pr-mcp/      # MCP server binary — exposes core as MCP tools for Claude Code
bin/              # committed Linux binary (produced by release workflow, not checked in manually)
docs/specs/       # one requirements.md per feature; acceptance criteria in WHEN/SHALL format
docs/specs/testing.md  # test structure, naming convention, coverage rules
```

## Architecture

Two binaries share a `core` crate. `core` owns all domain logic; the binaries are thin entry points.

**Two-pass review flow:**
1. Pass 1 (cheap model) → summary comment posted/updated on the PR
2. Pass 2 (better model) → inline comments posted as a GitHub pull request review

**SHA-based incremental review:** The last-reviewed HEAD SHA is embedded in the summary comment body as `<!-- sha: {sha} -->`. On each run, core reads this SHA, fetches only the delta (`/compare/{base}...{head}`), and reviews only new commits. The SHA is updated after each successful pass. `@bot re-review` in any PR comment forces a full re-review by clearing the stored SHA.

**Summary comment identity:** The `<!-- ai-pr-review -->` hidden HTML marker identifies the bot's summary comment. Core searches for this marker to find the comment to PATCH on subsequent runs.

**Config file:** `ai-pr-review.yml` at the consuming repo root. Core's `load_config(repo_root: &Path) -> Result<Config>` handles loading, defaults, and validation. Schema:

```yaml
summary:
  provider: anthropic        # default
  model: claude-haiku-4-5   # default
  prompt: |                  # optional, built-in default used if absent
    Review this diff and provide a concise summary...
inline:
  provider: anthropic
  model: claude-opus-4-5
  prompt: |                  # optional, built-in default used if absent
    Review this diff and return a JSON array of findings...
filters:
  exclude:
    - "**/*.lock"
    - "vendor/**"
    - "generated/**"
diff:
  max_kb: 100
rate_limiting:
  retries: 3
  backoff_seconds: 5
```

`INPUT_PROVIDER` and `INPUT_MODEL` env vars override the config file values.

**Diff pipeline:** fetch full diff or delta → apply glob exclusions → enforce `max_kb` limit (truncate to N files that fit, post a warning comment if truncated).

**Inline findings format:** Pass 2 model output is a JSON array: `[{"file": "...", "line": N, "severity": "high|medium|low", "comment": "..."}]`. Findings referencing files or lines not in the diff are silently skipped.

**MCP server** (`ai-pr-mcp`): long-lived stdio process, exposes tools callable as slash commands from Claude Code. Tools: plan synthesis, `/review-pr`, `/apply-review`. Writes output to `.ai-reviews/<branch>-*.md` (git-ignored).

**Release:** tag `v*` → CI compiles Linux x86_64 binary → strips debug symbols → commits to `bin/ai-pr-action` → `action.yml` invokes `${{ github.action_path }}/bin/ai-pr-action` directly. Consumers need no Rust toolchain.

## Specs

- `docs/specs/` — feature requirements, one directory per feature
- `docs/specs/testing.md` — test file layout, naming convention (`{feature}_{requirement}_{condition}`), coverage rules

## AI Tool Conventions

Track implementation progress in `.ai-reviews/<branch>-tasks.json` (git-ignored). Schema:

```json
[
  {
    "id": "1",
    "spec": "config-loader",
    "requirement": "Requirement 1",
    "criterion": 2,
    "description": "Missing config returns defaults",
    "status": "pending"
  }
]
```

`status`: `"pending"` | `"in_progress"` | `"done"` | `"skipped"`. Update as you work — this avoids re-reading the full spec on each step.

## Local Development

The easiest way to test locally is via the MCP server — no need to fake GitHub Actions context variables.

**Required env vars for MCP server:**
```bash
export GITHUB_TOKEN=ghp_...
export ANTHROPIC_API_KEY=sk-ant-...   # or OPENAI_API_KEY if using openai
```

Copy `.env.example` to `.env` and fill in your keys. Then build and run the MCP server:

```bash
cargo build -p ai-pr-mcp
./target/debug/ai-pr-mcp
```

Add it to your Claude Code MCP config, then use `/review-pr <pr-number>` directly from the editor.

**The GitHub Action binary** (`ai-pr-action`) is only needed when running as an actual GitHub Action. It requires additional context vars (`GITHUB_SHA`, `GITHUB_EVENT_PATH`, `GITHUB_REPOSITORY`) that the Actions runner sets automatically — these are awkward to fake locally, so use the MCP path instead.
