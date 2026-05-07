# Testing Guide

## Structure

```
crates/
  core/
    src/
      lib.rs
    tests/
      config_loader.rs      # Feature 3 acceptance criteria
      fetch_pr_diff.rs      # Feature 1 acceptance criteria
      model_interface.rs    # Feature 2 acceptance criteria
      two_pass_review.rs    # Feature 4 acceptance criteria
      post_comments.rs      # Feature 5 acceptance criteria
  ai-pr-action/
    src/
      main.rs
    tests/
      binary.rs             # Feature 6 acceptance criteria
  ai-pr-mcp/
    src/
      main.rs
    tests/
      mcp_server.rs         # Feature 8 acceptance criteria
      review_pr.rs          # Feature 9 acceptance criteria
      apply_review.rs       # Feature 10 acceptance criteria
tests/
  integration/
    full_review_flow.rs     # End-to-end: diff fetch → two-pass review → post comments
```

## Test Naming Convention

Test names map directly to spec acceptance criteria:

```
{feature}_{requirement}_{condition}
```

Examples:
- `config_loader_diff_size_zero_returns_error`
- `fetch_pr_diff_no_stored_sha_fetches_full_diff`
- `two_pass_review_bot_re_review_clears_sha`

Each WHEN/SHALL in a spec requirement should have a corresponding test.

## Unit Tests (in `src/`)

Fast, no I/O, no network. Use inline `#[cfg(test)]` modules for testing pure logic:
- Config parsing and validation
- Glob pattern matching
- Diff size calculation
- JSON findings parsing
- SHA extraction from comment body

## Integration Tests (in `tests/`)

Test a full module boundary with real file I/O or mocked HTTP. Use `mockito` or `wiremock` to mock GitHub and AI provider APIs.

Each integration test file corresponds to one spec feature.

## Property-Based Tests

Use `proptest` for requirements that must hold across a range of inputs:
- Config round-trip: serialise → deserialise → equal
- Diff size limit: any diff over max_kb is always truncated
- Filter application: excluded files never appear in filtered diff
- SHA embed/extract: any valid SHA survives a comment round-trip

Add proptest to `[dev-dependencies]` in the relevant crate's `Cargo.toml`.

## Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p core

# Specific test
cargo test config_loader_diff_size_zero

# With output
cargo test -- --nocapture
```

## Verifying Spec Coverage

For each `requirements.md`, every acceptance criterion should map to at least one test.
Work through them in order — if a criterion has no test, the feature is not verified complete.

To check coverage manually:
1. Open the relevant `requirements.md`
2. For each WHEN/SHALL line, find the corresponding test by name
3. If none exists, add it before marking the feature done
