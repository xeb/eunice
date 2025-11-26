# Design 2: The Pre-Mortem Prophet (Innovative)

## Purpose
To simulate project failure *before* it happens. This agent adopts a "Murphy's Law" mindset. It assumes a specific catastrophic failure has occurred in the future and works backward to find the "root cause" in the current codebase, serving as an automated risk assessment engine.

## Core Loop
1.  **Scenario Generation:** The agent uses `web_search` to find "Post-Mortems" of similar projects or technologies (e.g., "Redis cache stampede post-mortem", "React memory leak causes").
2.  **Hypothesis Formulation:** It constructs a "Failure Hypothesis": "The application fails because the Redis cache evicts keys too early."
3.  **Evidence Hunting:** It uses `grep` and `filesystem` analysis to find code that *prevents* or *enables* this scenario.
    *   *Search:* "ttl", "expiration", "eviction policy" configurations.
4.  **Prophecy:** If no preventative code is found, or if risky patterns are found, it generates a "Prophecy" file: `prophecies/2025-11-25_redis_stampede.md`.
    *   *Content:* "I predict a Redis Stampede because I found no 'jitter' in your TTL setting (Line 50, `cache.js`)."
5.  **Validation (Optional):** It generates a small `shell` script (test case) to try and trigger the failure (e.g., a load test script).

## Tool Usage
*   **web:** Gather real-world failure stories to ground the hypotheses in reality.
*   **filesystem & grep:** Deep code analysis to prove/disprove the hypothesis.
*   **shell:** Execute "Proof of Concept" exploit scripts (safely).
*   **memory:** Store "Prophecies" and their status (Refuted/Ignored/Fixed).

## Memory Architecture
*   **Nodes:** `FailureScenario`, `RiskFactor`, `CodeComponent`.
*   **Relations:** `CodeComponent HAS_RISK RiskFactor`, `RiskFactor CAUSES FailureScenario`.
*   **Insight:** The graph maps the "Risk Surface" of the application.

## Failure Modes
*   **Hallucination:** Predicting failures that are impossible due to external constraints (e.g., predicting Network Partition on a localhost demo).
    *   *Recovery:* Human feedback "Refute" marks the Scenario as invalid for this context.
*   **Destructive Testing:** The optional validation step could crash the dev environment.
    *   *Recovery:* Strict "Safe Mode" by default (static analysis only).

## Human Touchpoints
*   **Prophecy Review:** User reads the predictions.
*   **Mitigation:** User fixes the code, then asks the agent to "Re-evaluate Prophecy".
