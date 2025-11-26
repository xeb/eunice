# The Empirical Engineer

**Core Tools:** `shell`, `memory`, `filesystem`, `grep`
**Secondary Tools:** `web` (for Research)
**Persistence:** Memory Graph (Hypotheses, Evidence, Causal Models) + Filesystem (Test Scripts, Logs)

## Problem Domain
Software systems often exhibit "emergent defects" — bugs that arise from complex interactions rather than simple logic errors. Human engineers solve these by forming hypotheses and testing them. The **Empirical Engineer** automates this scientific method, transforming debugging from a chaotic hunt into a rigorous, documented process of falsification and discovery.

## Key Insight
**"Debugging as Science"**: Instead of "trying fixes" until something works, this agent treats a bug as a phenomenon to be explained. It formally logs every hypothesis, the experiment run to test it, and the result (Observation). This prevents "looping" (trying the same thing twice) and builds a permanent "Knowledge Graph" of the system's behavior that persists after the bug is fixed.

## Architecture

### 1. The Investigation Loop (Autonomous)
This is the default operating mode, inspired by the **Falsificationist** design.
1.  **Symptom Intake:** Agent receives a failure report (log, issue).
2.  **Hypothesis Generation:**
    *   *Internal:* Query `memory` for similar past cases.
    *   *External:* Use `web` to find standard causes for these symptoms.
3.  **Experiment Formulation:**
    *   Agent writes a specific, isolated shell script (e.g., `check_db_latency.sh`) to test the hypothesis.
    *   **Constraint:** The script must be *read-only* or *side-effect free* initially.
4.  **Execution & Observation:** Run script, capture stdout/stderr/exit code.
5.  **Conclusion:**
    *   **Falsified:** Mark hypothesis as FALSE. Prune this branch.
    *   **Corroborated:** Mark as PLAUSIBLE. Generate deeper sub-hypotheses.
6.  **Persistence:** Store the entire chain (Symptom -> Hypothesis -> Experiment -> Result) in the Knowledge Graph.

### 2. The Escalation Protocols
If the passive loop fails to isolate the cause:

*   **Mode A: Comparative Anatomy (Research)**
    *   Triggered when no internal hypotheses remain.
    *   Agent searches the web for architectural patterns matching the local codebase structure (using `grep` to map local structure).
    *   Goal: Find "structural analogies" to known classes of bugs.

*   **Mode B: Chaos Cartography (Active Testing)**
    *   **Requires Human Approval.**
    *   Agent proposes "Active Experiments" (e.g., "Block port 80", "Corrupt config file").
    *   Goal: Reproduce the bug by forcing the system into edge states.

## Memory Graph Structure (The "Lab Notebook")

*   **Entities:**
    *   `Symptom`: "High Latency", "500 Error"
    *   `Hypothesis`: "Database Connection Pool Exhausted"
    *   `Experiment`: "Run netstat count on port 5432"
    *   `Observation`: "200 connections ESTABLISHED"
    *   `Conclusion`: "Hypothesis PLAUSIBLE"

*   **Relations:**
    *   `Hypothesis TESTED_BY Experiment`
    *   `Experiment YIELDED Observation`
    *   `Observation REFUTES/SUPPORTS Hypothesis`

## Usage Scenario
**User:** "The API is timing out intermittently."
**Agent:**
1.  Creates `Entity: Symptom(API Timeout)`.
2.  Generates `Hypothesis: Network Saturation`, `Hypothesis: Slow DB Query`.
3.  Writes `exp_01_check_bandwidth.sh` and `exp_02_log_slow_queries.sh`.
4.  Runs `exp_01`. Result: Bandwidth normal. **Result:** Falsified.
5.  Runs `exp_02`. Result: Several queries > 5s. **Result:** Supported.
6.  Refines: `Hypothesis: Missing Index on Users Table`.
7.  ...Converges on root cause.

## Safety & Recovery
*   **ReadOnly by Default:** Agent starts with `grep`, `ls`, `cat`, `curl`.
*   **Sandbox Mode:** Destructive experiments (Chaos Mode) are only allowed if the user flags the environment as "Staging/Test".
*   **Journaling:** Every command is logged to a filesystem markdown file (`investigation_log.md`) so humans can audit the "Science".
