# Environment Variables

## GitHub Action binary (`ai-pr-action`)

These are set by the GitHub Actions runner from the `action.yml` inputs. You don't set them manually in CI — they're mapped automatically.

| Variable | Required | Description |
|----------|----------|-------------|
| `INPUT_GITHUB_TOKEN` | Yes | GitHub token for API calls. Use `${{ secrets.GITHUB_TOKEN }}` |
| `INPUT_ANTHROPIC_API_KEY` | If using Anthropic | Anthropic API key |
| `INPUT_OPENAI_API_KEY` | If using OpenAI | OpenAI API key |
| `INPUT_PROVIDER` | No | Override the provider from `ai-pr-review.yml` |
| `INPUT_MODEL` | No | Override the model from `ai-pr-review.yml` |
| `GITHUB_REPOSITORY` | Set by runner | `owner/repo` — identifies the repo being reviewed |
| `GITHUB_SHA` | Set by runner | HEAD commit SHA of the PR branch |
| `GITHUB_EVENT_PATH` | Set by runner | Path to the webhook event JSON (contains PR number) |

`GITHUB_REPOSITORY`, `GITHUB_SHA`, and `GITHUB_EVENT_PATH` are set automatically by the Actions runner. You never need to set these manually in CI.

## MCP server (`ai-pr-mcp`)

Set these in your shell or in the MCP server config's `env` block.

| Variable | Required | Description |
|----------|----------|-------------|
| `GITHUB_TOKEN` | Yes | GitHub personal access token with `repo` scope |
| `ANTHROPIC_API_KEY` | If using Anthropic | Anthropic API key |
| `OPENAI_API_KEY` | If using OpenAI | OpenAI API key |

Copy `.env.example` to `.env` for local development:

```bash
cp .env.example .env
```

`.env` is git-ignored. Never commit real keys.
