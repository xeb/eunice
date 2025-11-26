# Design 1: The Falsificationist

**Theme:** Safe, rigorous deductive reasoning for debugging.

## Purpose
To autonomously narrow down the root cause of software defects by systematically attempting to **falsify** hypotheses using non-destructive observations. It treats the codebase as an immutable artifact to be studied, not tinkered with until the problem is isolated.

## Core Loop: The Popperian Cycle
1.  **Observation:** Ingests error logs, stack traces, or ticket descriptions.
2.  **Hypothesis Generation:** Queries `memory` for potential causes based on symptoms (e.g., "If 500 error, maybe DB connection lost").
3.  **Experiment Design (Safe):** Generates a shell command or grep pattern that would **disprove** the hypothesis.
    *   *Example:* Hypothesis: "DB is down." -> Experiment: `curl -I localhost:5432` (If connection accepted, hypothesis falsified).
4.  **Execution:** Runs the command via `shell`.
5.  **Analysis:**
    *   **Refutation:** If hypothesis is falsified, mark as FALSE in `memory` and move to next.
    *   **Corroboration:** If experiment supports hypothesis, increase confidence and generate more specific sub-hypotheses.

## Tool Usage
*   **memory:** Stores the "Investigation Tree". Nodes are Hypotheses, Edges are logical dependencies. Attributes: `status` (open, falsified, corroborated), `confidence_score`.
*   **shell:** Restricted to read-only commands (`ls`, `cat`, `grep`, `curl`, `netstat`).
*   **grep:** Used to trace variable flow static analysis without running code.

## Memory Architecture
*   **Entities:** `Symptom`, `Hypothesis`, `Evidence`, `File`.
*   **Relations:**
    *   `Hypothesis EXPLAINS Symptom`
    *   `Evidence REFUTES Hypothesis`
    *   `Evidence SUPPORTS Hypothesis`

## Failure Modes
*   **Tunnel Vision:** Getting stuck on a branch of hypotheses that are all wrong because the root cause is outside the current mental model.
*   **False Negatives:** An experiment might incorrectly falsify a true hypothesis due to transient environment issues.

## Human Touchpoints
*   **Initial Trigger:** Human provides the bug report.
*   **Confirmation:** Human verifies the final "Suspect" before any code changes are proposed.
