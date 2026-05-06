# CI Setup

## Quick start

Add to `.github/workflows/review.yml` in any repo you want reviewed:

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

No checkout step needed. The action reads PR context from the GitHub environment.

## Secrets

Add these to your repo under **Settings → Secrets and variables → Actions**:

- `ANTHROPIC_API_KEY` — or `OPENAI_API_KEY` if using OpenAI
- `GITHUB_TOKEN` is provided automatically by GitHub Actions — no setup needed

## Action inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `github_token` | Yes | — | GitHub token for posting comments |
| `anthropic_api_key` | If using Anthropic | — | Anthropic API key |
| `openai_api_key` | If using OpenAI | — | OpenAI API key |
| `provider` | No | `anthropic` | Override provider (`anthropic` or `openai`) |
| `model` | No | From config | Override model name |

## Configuration

Drop `ai-pr-review.yml` in the root of the repo being reviewed to customise behaviour. All fields are optional — defaults apply if the file is absent or a field is omitted.

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

File exclusions are applied before the size check — excluded files don't count toward the `max_kb` budget.

## How it works

Every PR push triggers two passes:

**Pass 1 — fast model**
Reviews only the commits since the last run (delta from stored SHA to HEAD). Updates the summary comment in place — identified by a hidden `<!-- ai-pr-review -->` marker. Stores the reviewed SHA in the comment body as `<!-- sha: {sha} -->`.

**Pass 2 — better model**
Reviews the same delta. Dismisses the previous inline review ("Superseded by updated review") and posts fresh inline comments for changed lines only.

**Incremental reviews**
On the first run, the full PR diff is reviewed. On subsequent pushes, only new commits are sent to the model. This keeps costs low and feedback focused on what actually changed.

**Force a full re-review**
Comment `@bot re-review` on the PR. The next run ignores the stored SHA and reviews the entire PR from scratch.

**Pass failures**
If Pass 1 fails, a comment is posted explaining why. If Pass 2 fails, the Pass 1 summary remains intact and a separate failure comment is posted. A partial review is better than no review.

**Diff too large**
If the filtered diff exceeds `max_kb`, the first N files that fit within the limit are reviewed and a comment notes how many files were skipped.
