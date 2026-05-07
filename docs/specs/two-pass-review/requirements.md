# Feature 4 — Two-Pass Review

## Overview

Pass 1 uses a cheap model to post or update a top-level summary comment. Pass 2 uses a better model to post fresh inline comments. Both passes operate on the delta diff. The reviewed SHA is stored in the summary comment body. `@bot re-review` forces a full re-review.

## Requirements

### Requirement 1: Pass 1 — Summary Comment

**User Story:** As a PR author, I want a top-level summary comment posted after Pass 1 so that I get quick high-level feedback immediately.

#### Acceptance Criteria

1. WHEN Pass 1 completes successfully, the system SHALL post or update a top-level PR comment containing the summary.
2. WHEN a summary comment already exists (identified by the `<!-- ai-pr-review -->` hidden marker), the system SHALL update it in place via PATCH rather than posting a new comment.
3. WHEN no summary comment exists, the system SHALL create a new comment containing the `<!-- ai-pr-review -->` marker and the reviewed HEAD SHA embedded in the body.
4. The summary comment body SHALL embed the reviewed HEAD SHA in a hidden HTML comment in the format `<!-- sha: {sha} -->`.
5. WHEN Pass 1 fails, the system SHALL post a comment identifying that Pass 1 failed and including the error message. The existing summary comment (if any) SHALL remain unchanged.

---

### Requirement 2: Pass 2 — Inline Comments

**User Story:** As a PR author, I want inline comments on specific lines so that I can see precise feedback in context.

#### Acceptance Criteria

1. WHEN Pass 2 completes successfully, the system SHALL dismiss any previous inline review with the reason "Superseded by updated review" and post a fresh review with new inline comments.
2. WHEN no previous inline review exists, the system SHALL post a new review without attempting dismissal.
3. Pass 2 inline comments SHALL only reference lines present in the current delta diff.
4. WHEN Pass 2 fails, the system SHALL post a comment identifying that Pass 2 failed and including the error message. The Pass 1 summary comment SHALL remain intact.

---

### Requirement 3: SHA-Based Incremental Review

**User Story:** As the review system, I want to track the last reviewed SHA so that only new commits are reviewed on each run.

#### Acceptance Criteria

1. WHEN a run starts, the system SHALL extract the stored SHA from the `<!-- sha: {sha} -->` marker in the existing summary comment.
2. WHEN no stored SHA is found, the system SHALL perform a full review of the entire PR diff.
3. WHEN a stored SHA is found, the system SHALL fetch only the delta between the stored SHA and the current HEAD.
4. WHEN the delta is empty (no new commits since last review), the system SHALL exit without posting any comments.
5. WHEN a pass completes successfully, the system SHALL update the `<!-- sha: {sha} -->` marker in the summary comment to the current HEAD SHA.

---

### Requirement 4: Force Re-Review

**User Story:** As a PR author, I want to trigger a full re-review by commenting `@bot re-review` so that I can get fresh feedback after addressing comments.

#### Acceptance Criteria

1. WHEN any PR comment contains the text `@bot re-review`, the system SHALL ignore the stored SHA and perform a full review of the entire PR diff.
2. WHEN a forced re-review completes, the system SHALL update the stored SHA to the current HEAD as normal.
3. WHEN multiple `@bot re-review` comments exist, the system SHALL treat it the same as a single occurrence.
