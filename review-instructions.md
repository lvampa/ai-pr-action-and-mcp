# Review Instructions

This is a Rust workspace (`ai-pr-core`, `ai-pr-action`, `ai-pr-mcp`) that implements an AI-powered GitHub PR review tool. When reviewing changes, focus on:

## Correctness

- Async functions awaited correctly; no accidental blocking calls on the tokio runtime
- GitHub API responses parsed defensively — missing fields should produce descriptive errors, not panics
- `anyhow::Result` error chains preserve context at every `?` site (`with_context` / `context`)
- `serde` deserialization handles missing optional fields via `#[serde(default)]`; no silent data loss

## Security

- No secrets (tokens, API keys) logged or included in error messages returned to callers
- GitHub tokens read only from environment variables, never hardcoded or config-file sourced
- MCP tool inputs validated before any filesystem or network operation

## Rust idioms

- Prefer `?` over `unwrap` / `expect` outside of tests and `main`
- `clone()` on large strings or `Vec` inside hot loops warrants a note
- Trait objects (`Box<dyn ModelProvider>`) used correctly — `Send + Sync` bounds present where needed
- New public API items (`pub fn`, `pub struct`) should follow the existing naming convention

## Tests

- New behaviour should have unit tests in the same file or an integration test in `crates/ai-pr-core/tests/`
- Test names follow `{feature}_{requirement}_{condition}` (see `docs/specs/testing.md`)
- Mockito mocks registered in FIFO order; `expect(1)` used to exhaust a mock after one hit
- Avoid `unwrap()` in tests where a descriptive failure message would help

## MCP server

- Tool descriptions should be clear and action-oriented (Claude Code surfaces these to users)
- Tool errors returned as `ErrorData::invalid_params` for bad input, `ErrorData::internal_error` for runtime failures
- No blocking I/O (`std::fs`, `std::process::Command`) called directly on the async tokio thread without `spawn_blocking`

## GitHub Action binary

- Environment variable reads use `.ok().filter(|s| !s.is_empty())` pattern — no silent empty-string tokens
- Exit code 1 on any error (handled by `main` calling `std::process::exit(1)` on `Err`)

## Severity guide

- **High** — likely to cause a bug, security issue, or data loss in production
- **Medium** — worth fixing before merge, but won't cause immediate breakage
- **Low** — improvement or suggestion, fine to address in a follow-up
