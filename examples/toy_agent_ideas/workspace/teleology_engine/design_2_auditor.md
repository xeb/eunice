# Design 2: The Teleological Auditor (Innovative)

## Purpose
To detect **"Intent Drift"**—when the implemented code technically works but fails to satisfy the *spirit* or *specifics* of the original requirement (e.g., "The ticket asked for a *secure* login, but you added a *fast* login that skips auth").

## Loop Structure
1. **Ingest Intent:** Read the full text of a Ticket (Description, Acceptance Criteria, Comments). Extract key "Semantic Constraints" (e.g., "Must be encrypted", "Under 200ms", "User role X only").
2. **Ingest Implementation:** Read the `git diff` of the PR linked to the ticket.
3. **Semantic Verification:**
    - Use `grep` / pattern matching to look for evidence of the constraints.
    - *Example:* If Ticket says "Add caching", Agent greps for `Cache`, `Redis`, `TTL` in the diff.
    - If keywords are missing, flag as "Potential Drift".
4. **Drift Calculation:** Compute a "Teleology Score" (0-100%) representing how well the code keywords overlap with the requirement keywords.
5. **Interrogate:** If the score is low, the Agent posts a comment on the PR: "I see you linked Ticket-123 ('Add Caching'), but I see no cache invalidation logic in this diff. Please explain."

## Tool Usage
- **web:** Fetch detailed requirement text.
- **shell:** `git diff`, `git blame`.
- **grep:** Advanced pattern matching to find "evidence of intent" in code.
- **memory:** Store "Constraint Signatures" for common requirements (e.g., "If requirement mentions 'Privacy', code must import 'GDPR_Module'").

## Memory Architecture
- **Nodes:** `Requirement` (Text), `Constraint` (Concept), `Implementation` (Code snippet).
- **Edges:** `VIOLATES`, `SATISFIES`, `MISSING_EVIDENCE_OF`.
- **Learning:** Agent learns which keywords in tickets usually map to which libraries in code.

## Failure Modes
- **Semantic Gap:** The code uses "Memoization" but the ticket asked for "Caching" - Agent might miss the synonym (requires synonym graph).
- **False Confidence:** Agent approves code that contains the keywords but uses them incorrectly.
- **Recovery:** User replies "Memoization IS caching", Agent updates its synonym memory.

## Human Touchpoints
- **PR Comments:** Agent acts as a reviewer.
- **Debate:** Developer argues with Agent to teach it new terminology.
