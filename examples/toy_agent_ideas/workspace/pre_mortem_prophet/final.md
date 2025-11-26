# Final Design: The Pre-Mortem Prophet

## Synthesis
The most powerful approach combines the **Analytical Rigor** of Design 1 with the **Narrative Engagement** of Design 2, using the **Proof** capability of Design 3 as a validation step. The "Prophet" doesn't just say "Risk High"; it says "Here is the story of how we fail, and here is the test case that proves it's possible."

## Problem Domain
**Cognitive Blindness to Systemic Risk**. Engineers focus on feature delivery and happy paths. They ignore "Black Swan" events until they happen. Existing tools (linters) find syntax errors, but not *systemic* fragility (e.g., "What happens if S3 latency spikes to 5s?").

## Core Loop: "The Prophecy Cycle"
1.  **Architecture Mapping (The Vision)**:
    *   Uses filesystem and grep to build a dependency graph.
    *   Identifies "Seams" (API calls, DB queries, file I/O).
    *   Stores this map in memory.

2.  **Omen Search (The Research)**:
    *   Uses web to find "Outage Reports" related to the stack (e.g., "Postgres connection pool exhaustion").
    *   Extracts the *root cause* pattern from external news.

3.  **Scenario Weaver (The Narrative)**:
    *   Maps the external root cause to the internal architecture.
    *   Generates a **Future News Article**: "October 2025: How [Project Name] Lost 10k Users Due to a 5-Second Timeout."
    *   Writes this to workspace/pre_mortems/YYYY-MM-DD-scenario-1.md.

4.  **Reality Check (The Proof)**:
    *   (Optional/Safe) Generates a specific jest or python test case that simulates the condition (e.g., mocks a slow network response).
    *   Runs it via shell to see if the code handles it gracefully or crashes.

5.  **Prophecy Fulfillment (The Fix)**:
    *   If the system fails the simulation, the agent writes a **Preventative Patch** or a configuration change recommendation.

## Tool Usage
*   **Memory MCP**: Stores the "Theory of Fragility" (Knowledge Graph of weak points).
*   **Web MCP**: Fetches the "Curse of History" (Real-world failure data).
*   **Filesystem MCP**: Reads code, writes Narratives and Proofs.
*   **Shell MCP**: Runs non-destructive simulations (e.g., unit tests with mocks).

## Persistence Strategy
*   **Hybrid**:
    *   **Memory Graph**: Long-term tracking of "Risk Debt" and "Ignored Prophecies".
    *   **Filesystem**: Markdown reports (The Stories) and Test Files (The Proofs) live in the repo as documentation.

## Autonomy Level
**High (Analysis) / Low (Action)**.
*   It runs autonomously to generate stories and tests.
*   It *never* deploys or modifies production code.
*   It acts as a "Ghost of Future Failures" that haunts the repo with issues/reports until appeased.

## Failure Modes & Recovery
*   **Hallucinated APIs**: If the agent invents a library function in the test, the shell execution will fail. The agent catches this error and refines the test based on the actual code found via grep.
*   **Alarmism**: If it generates too many "fake" crises, users will mute it. *Mitigation*: Rank scenarios by "Plausibility Score" derived from code analysis (e.g., "Code actually uses this risky function").

## Key Insight
**"Narrative-Driven Chaos Engineering"**.
Most tools give you a metric ("Cyclomatic Complexity: 15"). This agent gives you a *nightmare* ("Your payment service hangs forever because you didn't set a socket timeout, causing a cascade failure"). Stories drive action better than metrics.
