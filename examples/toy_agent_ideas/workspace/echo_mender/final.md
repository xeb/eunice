# Agent: The Echo Mender

## Core Concept
**The Echo Mender** is a background "immune system" for your codebase. It operates on the principle that **a bug fix is rarely a singleton event**. When a developer fixes a bug, they are often identifying a *pattern of failure*. The Echo Mender captures this pattern and actively hunts for its clones, both immediately and in the future.

## Problem Domain
*   **Incomplete Refactoring:** Changing an API in 9 out of 10 places.
*   **Copy-Paste Bugs:** Fixing a logic error in one function but missing the copy-pasted version in another file.
*   **Regression:** Re-introducing a bad pattern that was banned/fixed months ago.

## Key Insight: "The Fix as a Query"
Most agents treat code as static text. The Echo Mender treats a **Git Commit** as a **Search Query**.
*   Input: `git diff` (Lines removed vs Lines added).
*   Abstraction: "We are replacing Pattern A with Pattern B".
*   Action: "SELECT * FROM codebase WHERE content MATCHES Pattern A".

## Architecture & Tools

### 1. The Pattern Extractor (Shell + Memory)
*   **Trigger:** Monitors `git log` for new commits.
*   **Logic:** parses the diff.
    *   If lines are removed, it generalizes them (replacing variables with wildcards).
    *   It stores this in **Memory** as a `VulnerabilitySignature`.
    *   *Example Memory:*
        *   `Entity: Pattern_123`
        *   `Observation: "Replaced md5() with sha256()"`
        *   `Observation: "Source Pattern: import md5"`

### 2. The Clone Hunter (Grep + Filesystem)
*   **Trigger:** After extraction, or on a schedule.
*   **Logic:**
    *   Retrieves active `VulnerabilitySignatures` from Memory.
    *   Uses `grep` (or `ripgrep`) to scan the *entire* codebase for these patterns.
    *   Filters out the file that was just fixed.

### 3. The Patcher (Text-Editor)
*   **Trigger:** When a match is found.
*   **Logic:**
    *   Reads the file content.
    *   Applies the transformation (Pattern A -> Pattern B) learned from the original commit.
    *   Runs local tests (via `shell`) to verify the fix doesn't break syntax.
    *   Generates a patch file or a new git branch.

## Memory Strategy: The "Immune Memory"
The agent builds a persistent graph of **"Banished Patterns"**.
*   **Nodes:** `Pattern`, `FixCommit`, `File`.
*   **Relations:**
    *   `(Pattern) -> DETECTED_IN -> (File)`
    *   `(FixCommit) -> DEFINED -> (Pattern)`
*   **Benefit:** If a junior dev unknowingly commits code using a "Banished Pattern" (e.g., using a deprecated auth function), the agent recognizes it *immediately* because it's in the memory graph, flagging it during CI.

## Failure Modes & Recovery
*   **False Positive Propagation:** The agent might try to apply a fix where it doesn't belong.
    *   *Recovery:* The agent *never* commits directly. It generates a "Proposal" (branch/patch). If the human rejects it (closes PR), the agent adds a `REJECTED_CONTEXT` observation to the memory to refine future matches.
*   **Context Sensitivity:** Simple grep might miss semantic nuances.
    *   *Recovery:* Start with strict literal matching. Evolve to AST-based matching if simple matching fails too often.

## Loop Example
1.  **Dev:** Commits "Fix: Replace hardcoded http with https".
2.  **Agent:** "I see you removed `http://`. I will remember that `http://` is bad." (Updates Memory).
3.  **Agent:** "Scanning project..." -> Found `http://` in `old_config.yaml`.
4.  **Agent:** "Proposed Fix: Update `old_config.yaml` to `https://`".
5.  **Agent:** (6 months later) Dev commits new file with `http://`.
6.  **Agent:** "Alert: You are re-introducing a Banished Pattern (defined in commit 5f3a1)."

## Human Interface
*   **Console:** Simple CLI for status.
*   **Filesystem:** Writes `workspace/echo_mender/proposals/` folders containing diffs.
*   **Git:** Pushes branches like `auto-fix/pattern-123`.
