# Agent: The Teleology Engine

## Purpose
The **Teleology Engine** is an autonomous "Intent Assurance" agent designed to bridge the gap between **Requirement Management** (Jira/Linear/Issues) and **Software Implementation** (Git/Code). 

Unlike standard traceability tools that simply link IDs (e.g., "Ticket-123 is in Commit-ABC"), the Teleology Engine uses **Semantic Triangulation** to verify that the code *actually satisfies* the business intent. It proactively injects this context back into the codebase, turning the source code into a self-documenting "Living Specification".

## Core Loop: The "Intent-Implementation-Verification" Cycle

1.  **Ingest (The Mandate):** 
    - Agent monitors the Issue Tracker (via `web`/`fetch`) for Active items.
    - It extracts "Semantic Constraints" (e.g., "Must handle retries", "GDPR compliance") and stores them in **Memory**.

2.  **Observe (The Reality):**
    - Agent monitors `git` activity (via `filesystem`/`shell`).
    - It analyzes the `diff` of active PRs.

3.  **Triangulate (The Check):**
    - **Keyword Verification:** Does the code contain evidence of the constraints? (e.g., looking for `retry_policy` or `anonymize(user)`).
    - **Negative Space Analysis:** Does the code *delete* safeguards required by the ticket?

4.  **Act (The Injection):**
    - **@Intent Tagging:** The Agent uses `text-editor` to insert structured comments into the modified code, permanently linking it to the Requirement:
      ```python
      # @intent: [PROJ-123] Enforce 2FA for admin users
      # @constraint: MUST NOT allow bypass via API
      def authenticate_admin(user): ...
      ```

5.  **Audit (The Report):**
    - Generates a "Drift Report" showing Tickets with no corresponding Code evidence, and Code with no clear Ticket intent.

## Tool Usage
- **memory:** Stores the **Intent Graph** (Requirement Node <-> Semantic Constraint <-> Code File).
- **web:** Fetches "The Truth" (Requirements from Jira/Linear).
- **filesystem / shell:** Navigates the "Reality" (Git history, file contents).
- **grep:** Searches for semantic evidence in the codebase.
- **text-editor:** Injects `@intent` tags into source files.

## Memory Architecture
The agent maintains a **Teleological Graph**:
- **Nodes:** `Requirement` (The Why), `Mechanism` (The How), `Artifact` (The Code).
- **Edges:** 
    - `IMPLEMENTS_INTENT` (Code -> Requirement)
    - `VIOLATES_CONSTRAINT` (Code -> Constraint)
    - `LACKS_EVIDENCE` (Requirement -> Code)

## Failure Modes & Recovery
- **Semantic Mismatch:** Agent flags "Missing Cache" because dev used "Memoize".
    - *Recovery:* Dev replies to Agent comment: "@agent define Cache = Memoize". Agent updates **Memory** ontology.
- **Tag Rot:** Code changes, tags stay.
    - *Recovery:* "Drift GC" mode scans files; if code below an `@intent` tag has changed >50%, Agent marks tag as `[STALE]` and requests re-verification.

## Human Touchpoints
- **PR Reviewer:** Agent appears as a bot in Pull Requests.
- **Tag Ratification:** Humans merge the `@intent` tags, accepting them as part of the codebase.
- **Ontology Training:** Humans teach the agent domain-specific synonyms.
