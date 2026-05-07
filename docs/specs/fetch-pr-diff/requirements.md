# Feature 1 — Fetch PR Diff

## Overview

Retrieve the diff for a given PR from the GitHub API. On first run fetch the full diff. On subsequent runs fetch only the delta between the stored SHA and HEAD using the GitHub compare API. Apply file exclusion filters before checking the diff size limit.

## Requirements

### Requirement 1: Full Diff on First Run

**User Story:** As the review system, I want to fetch the full diff on the first run so that the entire PR is reviewed when no prior review exists.

#### Acceptance Criteria

1. WHEN no stored SHA is present in the summary comment, the system SHALL fetch the full diff for the PR using the GitHub pulls API.
2. WHEN the full diff is fetched successfully, the system SHALL return the raw diff text for further processing.
3. WHEN the GitHub API returns an error, the system SHALL return a descriptive error including the HTTP status and PR number.

---

### Requirement 2: Delta Diff on Subsequent Runs

**User Story:** As the review system, I want to fetch only the delta since the last reviewed SHA so that re-reviews are efficient and focused on new changes.

#### Acceptance Criteria

1. WHEN a stored SHA is present in the summary comment, the system SHALL fetch the diff using the GitHub compare API (`/compare/{base}...{head}`).
2. WHEN the compare API returns an empty diff (no new commits), the system SHALL return an empty result and skip review.
3. WHEN the compare API returns an error, the system SHALL return a descriptive error including the base SHA, head SHA, and HTTP status.

---

### Requirement 3: File Exclusion Filters

**User Story:** As the review system, I want to exclude files matching configured glob patterns before size checking so that generated and vendored files don't consume the diff budget.

#### Acceptance Criteria

1. WHEN file exclusion patterns are configured, the system SHALL remove all files matching any pattern from the diff before size limit evaluation.
2. WHEN a file matches multiple exclusion patterns, the system SHALL exclude it (any match is sufficient).
3. WHEN no exclusion patterns are configured, the system SHALL apply the default patterns: `["**/*.lock", "vendor/**", "generated/**"]`.
4. WHEN all files are excluded, the system SHALL return an empty diff and skip review.

---

### Requirement 4: Diff Size Limit

**User Story:** As the review system, I want to enforce a configurable size limit on the filtered diff so that oversized PRs are handled gracefully without incurring excessive API costs.

#### Acceptance Criteria

1. WHEN the filtered diff exceeds `diff.max_kb` kilobytes, the system SHALL truncate to the first N files that fit within the limit.
2. WHEN the diff is truncated, the system SHALL post a PR comment noting that the diff exceeded the size limit and listing how many files were included vs. total.
3. WHEN the filtered diff is within the size limit, the system SHALL return the full filtered diff without truncation.
4. WHEN `diff.max_kb` is not configured, the system SHALL apply a default limit of 100KB.
