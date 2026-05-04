# Feature 5 — Post Inline PR Comments

## Overview

Parse structured JSON findings from the model and post each as an inline comment via the GitHub review API. The Pass 1 summary is posted as the top-level review body and updated in place on subsequent runs. The Pass 2 inline review is dismissed and reposted fresh on each run.

## Requirements

### Requirement 1: Parse Model Findings

**User Story:** As the review system, I want to parse structured JSON findings from the model response so that each finding can be posted as a precise inline comment.

#### Acceptance Criteria

1. The system SHALL parse model output as a JSON array of findings, each containing at minimum: `file`, `line`, `severity`, and `comment` fields.
2. WHEN the model response is not valid JSON, the system SHALL return a descriptive error and skip posting inline comments for that pass.
3. WHEN a finding references a file not present in the diff, the system SHALL skip that finding and log a warning.
4. WHEN a finding references a line number outside the diff hunk for that file, the system SHALL skip that finding and log a warning.
5. WHEN all findings are skipped due to invalid references, the system SHALL complete without error (no inline comments posted is a valid outcome).

---

### Requirement 2: Post Inline Comments via GitHub Review API

**User Story:** As a PR author, I want each finding posted as an inline comment on the relevant line so that feedback is shown in context in the GitHub UI.

#### Acceptance Criteria

1. The system SHALL submit all findings as a single GitHub pull request review using the `POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews` endpoint.
2. Each finding SHALL be submitted as a review comment with `path`, `line`, and `body` fields populated from the parsed finding.
3. WHEN the GitHub API returns an error for the review submission, the system SHALL return a descriptive error including the HTTP status.
4. The review SHALL be submitted with `event: "COMMENT"` (not APPROVE or REQUEST_CHANGES).

---

### Requirement 3: Dismiss Previous Inline Review

**User Story:** As a PR author, I want the previous inline review dismissed when a new one is posted so that stale comments don't clutter the PR.

#### Acceptance Criteria

1. WHEN a previous inline review exists (identified by the bot's user ID), the system SHALL dismiss it using `PUT /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/dismissals` before posting the new review.
2. The dismissal message SHALL be hardcoded to `"Superseded by updated review"`.
3. WHEN no previous inline review exists, the system SHALL skip the dismissal step without error.
4. WHEN the dismissal API call fails, the system SHALL log a warning and proceed to post the new review regardless.

---

### Requirement 4: Summary Comment Management

**User Story:** As a PR author, I want the summary comment updated in place on each run so that the PR timeline isn't cluttered with repeated top-level comments.

#### Acceptance Criteria

1. WHEN a summary comment with the `<!-- ai-pr-review -->` marker exists, the system SHALL update it via `PATCH /repos/{owner}/{repo}/issues/comments/{comment_id}`.
2. WHEN no summary comment exists, the system SHALL create one via `POST /repos/{owner}/{repo}/issues/{issue_number}/comments`.
3. The summary comment SHALL always contain the `<!-- ai-pr-review -->` marker and the `<!-- sha: {sha} -->` marker.
4. WHEN the PATCH or POST call fails, the system SHALL return a descriptive error including the HTTP status.
