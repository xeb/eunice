# Agent: The Litmus Agent

## Abstract
The Litmus Agent is a background maintenance daemon that treats project documentation (READMEs, tutorials, Wikis) as "executable specifications." It fights documentation rot by periodically selecting code snippets from Markdown files, executing them in a sandboxed environment, and autonomously patching them if they fail due to API changes or broken links. It bridges the gap between the codebase (truth) and documentation (often a lie).

## Core Toolset
- **shell:** To execute code snippets and capture stderr/stdout.
- **filesystem:** To parse Markdown and apply patches.
- **memory:** To maintain a "Freshness Graph" of documentation reliability and store successful repair strategies.
- **grep:** To cross-reference documentation code against the actual codebase (checking for renamed functions or moved modules).

## Problem Domain
Technical documentation inevitably drifts from the codebase.
-   "Works on my machine" syndrome in tutorials.
-   Deprecated API calls remaining in READMEs.
-   Broken URLs or `pip install` commands.
This erodes user trust and increases support burden.

## Architecture

### 1. The Verification Loop (The "Litmus Test")
The agent runs in a continuous, low-intensity background loop:
1.  **Sampling:** Selects a documentation file based on its "Rot Score" (time since last check / frequency of codebase changes).
2.  **Extraction:** Parses the Markdown to identify code blocks (e.g., ```bash`, ```python`).
3.  **Static Analysis (Fast Path):** Uses **grep** to verify that imported modules and called functions actually exist in the current codebase.
    *   *If function `foo.bar()` is in docs but not in code:* Flag as BROKEN immediately.
4.  **Dynamic Analysis (Slow Path):** Executes the snippet in a temporary directory/sandbox.
    *   *If success:* Updates Memory (`last_verified: <now>`, `status: PASS`).
    *   *If failure:* Enters **Repair Mode**.

### 2. Autonomous Repair Mode
When a snippet fails, the agent attempts to fix it:
1.  **Error Analysis:** Parses stderr (e.g., "ImportError: cannot import name 'OldFunc'").
2.  **Search Strategy:**
    *   **Internal:** searches the codebase for the missing symbol to see if it was renamed or moved.
    *   **External (Optional):** Uses Web Search to find migration guides if it's a third-party dependency.
3.  **Patch Generation:** Creates a candidate replacement snippet.
4.  **Verification:** Runs the *new* snippet.
5.  **Commit:** If successful, edits the Markdown file and pushes a commit/PR: "docs: Fix broken example in README (renamed OldFunc -> NewFunc)."

### 3. Memory Architecture
The agent builds a persistent **Knowledge Graph**:
-   **Entities:** `Document`, `Snippet`, `ErrorPattern`.
-   **Relations:**
    -   `Document CONTAINS Snippet`
    -   `Snippet DEPENDS_ON Library`
    -   `ErrorPattern RESOLVED_BY FixStrategy`
-   **Inference:** If `numpy` updates break one tutorial, the agent queries the graph for *all other snippets* using `numpy` and proactively re-tests them.

## Autonomy & Safety
-   **Bounded Autonomy:** The agent can directly commit changes to documentation files but is restricted from modifying source code.
-   **Sandboxing:** All execution happens in ephemeral directories to prevent file system pollution.
-   **Human-in-the-Loop:** Complex repairs (where multiple fixes are possible) are escalated to a "Draft PR" for human review rather than auto-committed.

## Key Insight
**"Probabilistic Documentation Testing"**: Instead of running *every* test on *every* commit (which is slow/expensive), the Litmus Agent treats documentation verification as a stochastic process, sampling snippets over time. It uses the Memory Graph to prioritize "hot" paths (frequently read docs) or "at-risk" paths (docs referencing recently changed code).

