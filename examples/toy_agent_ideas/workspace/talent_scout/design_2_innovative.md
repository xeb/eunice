# Design 2: The Headhunter (Active Outreach)

## Purpose
To actively recruit open source developers by automatically identifying their expertise and drafting highly personalized, context-aware outreach messages.

## Loop Structure
1.  **Problem Detection:** Scans local issue tracker for "hard" bugs (e.g., tagged "performance", "memory-leak").
2.  **Expertise Matching:** Scans the dependency tree for libraries related to the bug (e.g., an image bug -> `sharp`, `libvips`).
3.  **Commit Analysis:** Uses Web Search to find developers who have recently merged PRs in those libraries related to the specific bug keywords.
4.  **Drafting:** Generates an email draft: "Hi [Name], I saw your PR #[ID] in [Lib] fixing [Issue]. We are using [Lib] and hitting a similar edge case. Would you be interested in consulting?"
5.  **Notification:** Pings the user to review the draft.

## Tool Usage
*   **grep:** Search local codebase for error logs/issues.
*   **web:** Search GitHub commits/PRs for matching keywords.
*   **memory:** Track "Outreach Candidates" to avoid contacting the same person twice.
*   **filesystem:** Write draft emails to `drafts/`.

## Memory Architecture
*   **Entities:** `Candidate`, `OutreachAttempt`, `BugContext`.
*   **Relations:** `IS_EXPERT_IN`, `CONTACTED_ON`, `REJECTED`.

## Failure Modes
*   **Hallucination:** Misinterpreting a commit's relevance (e.g., a typo fix vs. a logic fix).
*   **Spam Risk:** Sending too many messages. *Mitigation:* Strict "1 outreach per week" limit encoded in the loop.

## Human Touchpoints
*   **Approval:** Mandatory human review before any message is sent. The agent only *drafts*.
