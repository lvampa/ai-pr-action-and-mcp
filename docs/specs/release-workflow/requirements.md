# Feature 7 — Release Workflow

## Overview

A GitHub Actions workflow in the `ai-pr-review` repo. On tag push, compiles the Rust binary for Linux and commits it to the repo. Consuming repos reference the action with `uses: your-org/ai-pr-review@v1` — no Rust toolchain or extra steps needed on the consumer side.

## Requirements

### Requirement 1: Trigger on Tag Push

**User Story:** As a maintainer, I want the release workflow to trigger automatically on version tag pushes so that releases are consistent and repeatable.

#### Acceptance Criteria

1. The workflow SHALL trigger on pushes to tags matching the pattern `v*` (e.g., `v1`, `v1.0`, `v1.2.3`).
2. The workflow SHALL NOT trigger on branch pushes or pull requests.

---

### Requirement 2: Compile Linux Binary

**User Story:** As a maintainer, I want the workflow to compile a Linux x86_64 binary so that it runs on standard GitHub Actions runners without any toolchain setup by consumers.

#### Acceptance Criteria

1. The workflow SHALL compile the `ai-pr-action` binary targeting `x86_64-unknown-linux-gnu`.
2. The workflow SHALL build in release mode (`cargo build --release`).
3. WHEN the build fails, the workflow SHALL fail the job and not proceed to commit.
4. The compiled binary SHALL be stripped of debug symbols to minimise file size.

---

### Requirement 3: Commit Binary to Repo

**User Story:** As a maintainer, I want the compiled binary committed to the repo at the tag so that consumers can reference it directly without a build step.

#### Acceptance Criteria

1. The workflow SHALL commit the compiled binary to the repository at a well-known path (e.g., `bin/ai-pr-action`).
2. The commit SHALL be pushed to the same tag ref that triggered the workflow (or a new commit on the tag).
3. WHEN the binary has not changed since the last release, the workflow SHALL still complete successfully (idempotent).
4. The workflow SHALL use a bot identity (e.g., `github-actions[bot]`) for the commit author.

---

### Requirement 4: Consumer Experience

**User Story:** As a consuming repo, I want to reference the action with a simple `uses:` line so that I don't need to install Rust or run any build steps.

#### Acceptance Criteria

1. After a successful release, consuming repos SHALL be able to use the action with `uses: your-org/ai-pr-review@{tag}` and no additional setup steps.
2. The `action.yml` SHALL invoke the committed binary directly using `${{ github.action_path }}/bin/ai-pr-action`.
3. The binary SHALL be executable (file permissions set correctly) after being committed.
