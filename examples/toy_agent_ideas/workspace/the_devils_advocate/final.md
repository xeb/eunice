# Agent: The Pre-Mortem Prophet

## Purpose
The Pre-Mortem Prophet is an autonomous risk assessment agent that "remembers the future failure" of your project. It combines **Persona-based Scenario Generation** (from Design 3) with **Evidence-Based Prophecy** (from Design 2) to identify structural, security, and product risks before they manifest.

Instead of generic linting, it creates specific, grounded narratives of failure: *"I predict this API will fail under load because the retry logic lacks jitter, as seen in the 2023 Redis outage."*

## System Architecture

### 1. The Persona Council (Memory)
The agent maintains a set of "Failure Archetypes" in the Memory Graph:
*   **The Black Hat:** Generates security breach scenarios.
*   **The Scale Skeptic:** Generates performance bottleneck scenarios.
*   **The Confused User:** Generates UX friction scenarios.
*   **The Compliance Officer:** Generates regulatory violation scenarios.

### 2. The Prophecy Loop (Execution)
1.  **Context Loading:** The agent scans the codebase using `filesystem_directory_tree` and reads key architectural files.
2.  **Scenario Genesis:** A selected Persona generates a "Future Failure" based on current code structures + external web research.
    *   *Input:* "We are using JWTs for session management."
    *   *Web Search:* "JWT revocation pitfalls 2024".
    *   *Scenario:* "Users cannot be logged out immediately upon ban."
3.  **Evidence Hunting:** The agent uses `grep` and `filesystem` to validate the scenario.
    *   *Action:* Grep for "blacklist", "revocation", "redis_check" in auth middleware.
4.  **Verdict:**
    *   **Prophecy Confirmed:** No mitigation found. A "Prophecy" is written to `prophecies/`.
    *   **Prophecy Averted:** Mitigation found. A "Defense" is recorded in Memory.

### 3. The Prophecy Ledger (Persistence)
The Memory Graph tracks the lifecycle of risks:
*   Nodes: `RiskScenario`, `Evidence`, `Mitigation`.
*   Edges: `Codebase HAS_RISK Scenario`, `Mitigation BLOCK Scenario`.
*   This allows the agent to *re-verify* old risks when code changes (Regression Testing for Risks).

## Tool Usage
*   **memory:** Stores the "Risk Graph" and Persona definitions.
*   **web_brave_web_search:** Finds "Post-Mortems" and "CVEs" to ground scenarios in reality.
*   **grep_ripgrep:** The primary sensor for finding code evidence.
*   **filesystem:** Reading design docs and writing Prophecy Reports.
*   **shell:** (Optional) Running static analysis tools (e.g., `semgrep`) to support evidence hunting.

## Failure Modes & Recovery
*   **Hallucinated Risks:** Predicting a database deadlock in a stateless app.
    *   *Recovery:* Human "Refutation" feedback is stored in Memory: "We don't use a DB." The agent learns to suppress this class of risk.
*   **Alarm Fatigue:** Generating too many minor warnings.
    *   *Recovery:* "Severity Scoring" based on web search result counts (e.g., if a bug is famous, it's high severity).

## Human Touchpoints
*   **The Daily Prophecy:** A morning report of "New Risks Detected".
*   **The Defense:** Developer adds a comment or code fix and asks the agent to "Verify Mitigation".
