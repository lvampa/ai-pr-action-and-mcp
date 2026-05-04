# MCP and CI PR review tool

Three tools for AI-assisted code review, built around the same core:

- **GitHub Action** — runs on every PR push, posts a summary and inline comments automatically
- **`/review-pr`** — MCP tool for Claude Code, triggers a full review on any PR from your editor
- **`/apply-review`** — MCP tool that reads the review plan and applies the suggestions to your local branch

Inspired by [Codex Code Review](https://github.com/marketplace/actions/codex-code-review-actor) — same idea, but bring your own API key and works with Anthropic or OpenAI.

---

## How the review works

Two passes on every run:

```
PR push (or /review-pr)
        │
        ▼
┌──────────────────────────────────────────┐
│  Pass 1 — fast model                      │
│  Reviews only new commits since last run  │
│  Updates summary comment in place         │
└──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────┐
│  Pass 2 — better model                    │
│  Same delta, line-level findings          │
│  Dismisses old inline review              │
│  Posts fresh inline comments              │
└──────────────────────────────────────────┘
```

**Incremental by default.** The last-reviewed SHA is stored in the summary comment. Each run only sends new commits to the model — not the whole PR. On a busy PR this makes a real difference to cost and noise.

Comment `@bot re-review` on the PR to force a full re-review from scratch.

---

## GitHub Action

Add to `.github/workflows/review.yml`:

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

No checkout step needed. The action reads PR context from the GitHub environment and handles everything else.

---

## Configuration

Add `ai-pr-review.yml` to the root of the repo being reviewed. All fields are optional.

```yaml
summary:
  provider: anthropic
  model: claude-haiku-4-5      # fast + cheap for the summary pass
  prompt: |                    # optional — built-in default used if absent
    Review this diff and provide a concise summary...

inline:
  provider: anthropic
  model: claude-opus-4-5       # better model for line-level findings
  prompt: |                    # optional — built-in default used if absent
    Review this diff and return a JSON array of findings...

filters:
  exclude:
    - "**/*.lock"
    - "vendor/**"
    - "generated/**"           # excluded before the size check — don't count toward the budget

diff:
  max_kb: 100                  # if the filtered diff is over this, review the first N files
                               # and post a comment noting what was skipped

rate_limiting:
  retries: 3
  backoff_seconds: 5
```

You can mix providers — summary on OpenAI, inline on Anthropic, or whatever combination makes sense for your budget.

---

## MCP tools for Claude Code

The MCP server exposes the same review logic as slash commands you can call from inside a Claude Code session.

**Setup:**

```bash
cp .env.example .env
# fill in GITHUB_TOKEN and ANTHROPIC_API_KEY (or OPENAI_API_KEY)

cargo build -p ai-pr-mcp
```

Add to your Claude Code MCP config:

```json
{
  "mcpServers": {
    "ai-pr-review": {
      "command": "/path/to/ai-pr-mcp",
      "env": {
        "GITHUB_TOKEN": "ghp_...",
        "ANTHROPIC_API_KEY": "sk-ant-..."
      }
    }
  }
}
```

**`/review-pr <pr-number>`**

Runs the two-pass review on any PR. Findings render inline in Claude Code grouped by severity — high, medium, low. Also writes `.ai-reviews/<branch>-review.md` so you have a local copy.

```
/review-pr 142

## Summary
The auth middleware change looks correct but there's a subtle
session fixation risk worth addressing before merge.

### High
src/auth/middleware.rs  line 84
Session ID should be regenerated after privilege escalation.

### Medium
src/auth/middleware.rs  line 102
Error path leaks whether the user exists via timing difference.
```

**`/apply-review`**

Reads the review findings from `/review-pr`, builds an actionable plan, and presents it. Asks before touching anything — confirm to apply the changes and commit them to your branch.

```
/apply-review

Plan for PR #142 (3 actionable changes):
  1. src/auth/middleware.rs  line 84 — regenerate session ID after privilege escalation
  2. src/auth/middleware.rs  line 102 — fix timing leak on error path
  3. src/cache.rs  line 42 — include locale in cache key

Apply these changes? [y/N]
```

---

## What gets posted to the PR

```
┌──────────────────────────────────────────────────────────────┐
│ 🤖 AI Review · reviewed abc1234                               │
│                                                               │
│ Overall the change looks solid. The new cache layer correctly │
│ invalidates on write. A few edge cases worth looking at.      │
│                                                               │
│ <!-- ai-pr-review --><!-- sha: abc1234 -->                    │
└──────────────────────────────────────────────────────────────┘

src/cache.rs  line 42
  Cache key doesn't account for the `locale` field — two users
  with different locales will share cached responses.

src/cache.rs  line 87
  Consider entry() here to avoid the double lookup.
```

The summary comment is updated in place on each push. The inline review is dismissed and reposted fresh. No comment spam.

---

## Development

```bash
cargo test                    # all tests
cargo test -p core            # core crate only
cargo clippy -- -D warnings
```

Specs are in `docs/specs/` — one `requirements.md` per feature. `docs/specs/testing.md` covers test structure and naming conventions.
