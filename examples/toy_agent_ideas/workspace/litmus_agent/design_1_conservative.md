# Design 1: The Docu-Linter (Conservative)

## Purpose
A specialized maintenance agent that treats documentation as a test suite. It periodically validates that code snippets in `README.md` and other documentation files are executable and produce expected outputs, preventing "documentation rot."

## Core Toolset
- **filesystem:** To find and read Markdown files.
- **shell:** To execute extracted code snippets in a controlled environment.
- **memory:** To track the "freshness" and pass/fail history of each snippet.

## Architecture

### 1. Loop Structure
The agent runs as a low-priority background daemon or a CI step:
1.  **Discovery:** Scans specific directories (allowed paths) for `.md` files.
2.  **Parsing:** Extracts code blocks marked with language tags (e.g., ```bash`, ```python`).
3.  **Filtration:** Ignores blocks marked `<!-- skip-test -->` or lacking specific tags.
4.  **Execution:** Runs the snippet in a temporary sub-shell or sandbox.
5.  **Recording:** Logs the result (exit code, stdout, stderr) to the Memory Graph.
6.  **Reporting:** Generates a "Freshness Report" highlighting broken snippets.

### 2. Memory Architecture
Uses the Memory Graph to store:
-   **Entities:** `Document` (file path), `Snippet` (hash of content).
-   **Relations:** `Document HAS Snippet`.
-   **Observations:**
    -   `last_verified_at`: Timestamp.
    -   `status`: PASS / FAIL.
    -   `error_log`: Output if failed.

### 3. Safety & Failure Modes
-   **Read-Only:** Does not modify documentation files. Only reports issues.
-   **Sandboxing:** Execution is limited to a non-privileged user or container to prevent destructive commands (e.g., `rm -rf /`).
-   **Timeout:** Long-running snippets are killed to prevent hangs.

### 4. Human Touchpoints
-   **Report Review:** Developers review the generated report.
-   **Whitelisting:** Humans add comments to docs to skip dangerous/interactive snippets.

## Pros/Cons
-   **Pros:** Safe, low risk of corrupting data, easy to integrate into existing CI.
-   **Cons:** Passive; requires human intervention to actually fix the docs.
