# Feature 3 — Config Loader

## Overview

Reads `ai-pr-review.yml` from the repo root. Drives model selection, provider, prompts, file filters, diff size limit, and rate limiting. Falls back to sensible defaults if not present. Shared library crate used by both `ai-pr-action` and `ai-pr-mcp`.

```yaml
summary:
  provider: anthropic
  model: claude-haiku-4-5
  prompt: |                    # optional — falls back to built-in default
    Review this diff and provide a concise summary...
inline:
  provider: anthropic
  model: claude-opus-4-5
  prompt: |                    # optional — falls back to built-in default
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

## Requirements

### Requirement 1: Load Configuration File

**User Story:** As a library consumer, I want the config loader to read `ai-pr-review.yml` from the repo root so that I can customise review behaviour without changing action inputs.

#### Acceptance Criteria

1. WHEN `ai-pr-review.yml` is present at the repo root, the system SHALL parse it into a typed `Config` struct.
2. WHEN `ai-pr-review.yml` is absent, the system SHALL return a `Config` struct populated entirely with default values.
3. WHEN `ai-pr-review.yml` contains a YAML syntax error, the system SHALL return a descriptive error identifying the file path and parse failure.
4. WHEN `ai-pr-review.yml` contains an unrecognised key, the system SHALL ignore it and continue loading remaining fields.

---

### Requirement 2: Summary and Inline Model Configuration

**User Story:** As a library consumer, I want to configure provider, model, and prompt separately for each pass so that I can use a cheap model for Pass 1 and a better model for Pass 2, with custom instructions for each.

#### Acceptance Criteria

1. WHEN `summary.provider` is absent, the system SHALL default to `anthropic`.
2. WHEN `summary.model` is absent, the system SHALL default to `claude-haiku-4-5`.
3. WHEN `summary.prompt` is present, the system SHALL use it as the prompt for Pass 1.
4. WHEN `summary.prompt` is absent, the system SHALL use the built-in default summary prompt.
5. WHEN `inline.provider` is absent, the system SHALL default to `anthropic`.
6. WHEN `inline.model` is absent, the system SHALL default to `claude-opus-4-5`.
7. WHEN `inline.prompt` is present, the system SHALL use it as the prompt for Pass 2.
8. WHEN `inline.prompt` is absent, the system SHALL use the built-in default inline prompt.
9. WHEN either `provider` field is set to a value other than `anthropic` or `openai`, the system SHALL return a descriptive validation error naming the invalid value.

The built-in default summary prompt SHALL instruct the model to produce a concise high-level summary of the changes, noting overall quality, key risks, and areas of concern.

The built-in default inline prompt SHALL instruct the model to produce a JSON array of findings with `file`, `line`, `severity` (`high`, `medium`, or `low`), and `comment` fields, covering correctness issues, security concerns, and code quality problems.

---

### Requirement 3: File Exclusion Filters

**User Story:** As a library consumer, I want to specify glob patterns for files to exclude from the diff so that lock files and generated files don't consume the diff budget.

#### Acceptance Criteria

1. WHEN `filters.exclude` is present, the system SHALL use the provided list of glob patterns.
2. WHEN `filters.exclude` is absent, the system SHALL default to `["**/*.lock", "vendor/**", "generated/**"]`.
3. WHEN `filters.exclude` is an empty list, the system SHALL apply no exclusions.

---

### Requirement 4: Diff Size Limit

**User Story:** As a library consumer, I want to set a maximum diff size in kilobytes so that oversized diffs are caught before incurring API costs.

#### Acceptance Criteria

1. WHEN `diff.max_kb` is present, the system SHALL use the specified value.
2. WHEN `diff.max_kb` is absent, the system SHALL default to `100`.
3. WHEN `diff.max_kb` is zero or negative, the system SHALL return a descriptive validation error.

---

### Requirement 5: Rate Limiting Configuration

**User Story:** As a library consumer, I want to configure retry count and backoff so that transient API errors are handled gracefully.

#### Acceptance Criteria

1. WHEN `rate_limiting.retries` is absent, the system SHALL default to `3`.
2. WHEN `rate_limiting.backoff_seconds` is absent, the system SHALL default to `5`.
3. WHEN either value is negative, the system SHALL return a descriptive validation error.

---

### Requirement 6: Public API

**User Story:** As a library consumer, I want a single entry-point function so that both binaries can load config with minimal boilerplate.

#### Acceptance Criteria

1. The system SHALL expose a `load_config(repo_root: &Path) -> Result<Config>` function that handles file discovery, parsing, defaults, and validation in one call.
2. WHEN `load_config` succeeds, it SHALL return a fully-resolved `Config` with all defaults applied.
3. WHEN `load_config` fails, it SHALL return an error identifying the source of the failure.
