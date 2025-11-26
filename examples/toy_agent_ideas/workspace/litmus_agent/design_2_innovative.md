# Design 2: The Self-Healing Tutorial (Innovative)

## Purpose
An active repair agent that not only finds broken documentation but attempts to fix it. It treats documentation errors as bugs to be solved using codebase search and web research (e.g., finding updated API calls).

## Core Toolset
- **shell:** To run snippets and capturing errors.
- **grep:** To search the local codebase for correct function signatures/usage when a doc snippet fails.
- **web:** To search online documentation/StackOverflow if local search fails.
- **filesystem:** To patch the Markdown files with corrected code.
- **memory:** To build a knowledge base of "Common Fixes" and "Deprecated -> New" mappings.

## Architecture

### 1. Loop Structure
1.  **Monitor:** Watches for file changes or runs on schedule.
2.  **Execute & Catch:** Runs snippets. If a snippet fails (non-zero exit code):
    *   **Analyze Error:** Captures stderr (e.g., "ModuleNotFoundError", "AttributeError").
    *   **Local Search (Grep):** Searches the codebase for the symbol or package causing the error. "Is this function renamed?"
    *   **Web Search:** If local search fails, searches online for the error message + library name.
    *   **Hypothesize Fix:** Generates a candidate fix (e.g., change `pip install old-lib` to `pip install new-lib`).
    *   **Verify Fix:** Runs the *modified* snippet in a sandbox.
3.  **Apply Patch:** If the fix works, it edits the Markdown file directly.
4.  **Notify:** Logs the change: "Updated README.md: Replaced deprecated API call 'foo()' with 'bar()'."

### 2. Memory Architecture
-   **Knowledge Graph:** Stores mapped relationships between *Error Patterns* and *Successful Fixes*.
-   **Learning:** Over time, it learns that "Error X usually means Dependency Y is missing," speeding up future repairs.

### 3. Safety & Failure Modes
-   **Git Integration:** Always creates a new Git branch (e.g., `docs/auto-fix-123`) before applying changes.
-   **Revertibility:** If a fix causes *subsequent* snippets to fail (dependency chain), it rolls back.
-   **Hallucination Check:** Uses `grep` to ensure the "fixed" code actually utilizes existing project artifacts.

### 4. Human Touchpoints
-   **PR Review:** The agent submits a Pull Request with its fixes, requiring human approval to merge.
-   **Guidance:** Humans can provide "Hints" in memory if the agent gets stuck on a specific error.

## Pros/Cons
-   **Pros:** Actively reduces maintenance burden; keeps docs in sync with code evolution.
-   **Cons:** Higher risk of generating "working but incorrect" code (e.g., bypasses security checks to make it run). Complex implementation.
