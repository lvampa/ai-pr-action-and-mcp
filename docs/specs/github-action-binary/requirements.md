# Feature 6 — GitHub Action Binary

## Overview

A single compiled Linux binary (`ai-pr-action`) committed to the repo. Reads API keys from environment variables. Reads all other configuration from `ai-pr-review.yml`. Invoked directly by `action.yml` using `github.action_path`.

## Requirements

### Requirement 1: Environment Variable Inputs

**User Story:** As a GitHub Actions user, I want the binary to read API keys from environment variables so that secrets are never written to disk or config files.

#### Acceptance Criteria

1. The binary SHALL read `INPUT_ANTHROPIC_API_KEY` for the Anthropic API key.
2. The binary SHALL read `INPUT_OPENAI_API_KEY` for the OpenAI API key.
3. The binary SHALL read `INPUT_GITHUB_TOKEN` for authenticating GitHub API calls.
4. The binary SHALL read `INPUT_PROVIDER` to override the default provider (optional).
5. The binary SHALL read `INPUT_MODEL` to override the default model (optional).
6. WHEN `INPUT_GITHUB_TOKEN` is absent or empty, the binary SHALL exit with a non-zero status and a descriptive error message.
7. WHEN the required API key for the configured provider is absent, the binary SHALL exit with a non-zero status and a descriptive error message before making any API calls.

---

### Requirement 2: Config File Integration

**User Story:** As a GitHub Actions user, I want the binary to read `ai-pr-review.yml` from the repo root so that review behaviour is configurable per repository.

#### Acceptance Criteria

1. The binary SHALL load `ai-pr-review.yml` from the current working directory (the checked-out repo root).
2. WHEN `ai-pr-review.yml` is absent, the binary SHALL proceed with default configuration values.
3. WHEN `INPUT_PROVIDER` or `INPUT_MODEL` env vars are set, they SHALL override the corresponding values from `ai-pr-review.yml`.

---

### Requirement 3: GitHub Actions Context

**User Story:** As the binary, I want to read PR context from the GitHub Actions environment so that I know which PR to review without requiring explicit inputs.

#### Acceptance Criteria

1. The binary SHALL read the PR number from the `GITHUB_EVENT_PATH` JSON payload (`pull_request.number`).
2. The binary SHALL read the repository owner and name from `GITHUB_REPOSITORY` (format: `owner/repo`).
3. The binary SHALL read the current HEAD SHA from `GITHUB_SHA`.
4. WHEN any required GitHub Actions context variable is absent, the binary SHALL exit with a non-zero status and a descriptive error message.

---

### Requirement 4: Exit Codes

**User Story:** As a GitHub Actions workflow, I want the binary to use standard exit codes so that the workflow step fails correctly on error.

#### Acceptance Criteria

1. WHEN the review completes successfully, the binary SHALL exit with code `0`.
2. WHEN any unrecoverable error occurs (missing token, API failure after retries, etc.), the binary SHALL exit with a non-zero code and print the error to stderr.
3. WHEN Pass 1 succeeds but Pass 2 fails, the binary SHALL exit with a non-zero code (partial failure is still a failure).

---

### Requirement 5: Project Documentation

**User Story:** As a contributor or AI tool, I want a concise entry-point document at the repo root so that I can orient quickly without reading every spec.

#### Acceptance Criteria

1. A `CLAUDE.md` file SHALL exist at the repo root.
2. `CLAUDE.md` SHALL link to `docs/specs/` for feature specs, `docs/specs/testing.md` for test conventions, and describe the workspace layout and release process in brief.
3. `CLAUDE.md` SHALL NOT duplicate content from the spec files — it points to them, not replaces them.
