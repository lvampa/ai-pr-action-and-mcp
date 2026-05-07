# ai-pr-review

Three tools for AI-assisted code review, built around the same core:

- **GitHub Action** — runs on every PR push, posts a summary and inline comments automatically
- **`/review-pr`** — MCP tool for Claude Code, triggers a local review on any PR from your editor
- **`/apply-review`** — MCP tool that reads the review findings, builds a plan, and applies the changes to your local branch

Inspired by [Codex Code Review](https://github.com/marketplace/actions/codex-code-review-actor) — same idea, bring your own API key, works with Anthropic or OpenAI.

---

## How the review works

Two passes on every run:

```
PR push (or /review-pr)
        │
        ▼
┌──────────────────────────────────────────────┐
│  Pass 1 — fast model                          │
│  Reviews only new commits since last run      │
│  Updates summary comment in place (CI only)   │
└──────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────┐
│  Pass 2 — better model                        │
│  Same delta, line-level findings              │
│  Dismisses old inline review (CI only)        │
│  Posts fresh inline comments (CI only)        │
└──────────────────────────────────────────────┘
```

**Incremental by default.** The last-reviewed SHA is stored in the summary comment. Each run only sends new commits to the model — not the whole PR.

Comment `@bot re-review` on the PR to force a full re-review from scratch.

---

## GitHub Action

→ [Full CI setup guide](docs/ci-setup.md)

```yaml
name: AI Review
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: your-org/ai-pr-review@v1
        with:
          anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}
          github_token: ${{ secrets.GITHUB_TOKEN }}
```

---

## MCP tools for Claude Code

→ [Full local setup guide](docs/local-mcp.md)

Build and install in one step:

```bash
cargo build -p ai-pr-mcp
export GITHUB_TOKEN=ghp_...
export ANTHROPIC_API_KEY=sk-ant-...
./target/debug/ai-pr-mcp install
```

This registers the MCP server with Claude Code and writes the slash command files. Restart Claude Code, then:

**`/review-pr 142`** — runs the two-pass review locally on PR #142. Findings render inline grouped by severity. Writes `.ai-reviews/<branch>-review.md`. Nothing posted to GitHub.

**`/apply-review`** — reads the review findings, builds an actionable plan, shows it to you, then applies and commits on confirmation.

---

## Configuration

Add `ai-pr-review.yml` to the root of the repo being reviewed. All fields optional.

```yaml
summary:
  provider: anthropic
  model: claude-haiku-4-5

inline:
  provider: anthropic
  model: claude-opus-4-5

review_instructions: review-instructions.md  # optional — path relative to repo root
                                              # defaults to review-instructions.md at root
                                              # built-in default used if file is absent

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

→ [Environment variables reference](docs/environment.md)

---

## Development

```bash
cargo test                    # all tests
cargo test -p core            # core crate only
cargo clippy -- -D warnings
```

Specs in `docs/specs/` — one `requirements.md` per feature. Test conventions in `docs/specs/testing.md`.
