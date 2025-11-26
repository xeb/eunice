# Design 1: The Coherence Linter (Conservative)

## Purpose
To automatically detect and report technical inconsistencies between code, comments, and documentation using static analysis techniques, acting as a "semantic linter" that runs alongside standard syntax linters.

## Core Philosophy
"If it isn't tested or mechanically verified, it is likely wrong." This agent focuses on low-hanging fruit: outdated TODOs, parameter mismatches, and dead links, avoiding high-risk semantic guessing.

## Loop Structure
1.  **Trigger**: Runs on a schedule or git hook (simulated via shell loop).
2.  **Scan Phase**:
    *   Iterates through the codebase using `filesystem`.
    *   Uses `grep` to find Javadoc/Docstring patterns and function signatures.
3.  **Analysis Phase**:
    *   Parses comment params (`@param x`) and compares with function signature (`fn(x, y)`).
    *   Checks for `TODO` comments with dates older than X months.
    *   Validates hyperlinks in Markdown files using `fetch` or `grep` for local anchors.
4.  **Reporting Phase**:
    *   Generates a `COHERENCE_REPORT.md` file in the root.
    *   Does NOT modify code.

## Tool Usage
*   **filesystem**: Read code and markdown files. Write report.
*   **grep**: Regex-based extraction of comments and code structures.
*   **shell**: Execute existing linters or build tools to gather standard metadata.

## Memory Architecture
*   **Stateless**: This variant does not maintain a persistent graph. It re-scans each time to ensure it reflects the exact current state, similar to a compiler.
*   **Configuration**: Reads a `.coherence.json` config file for ignore patterns.

## Failure Modes
*   **False Positives**: Might flag a mismatch where the comment implies a broad type (e.g., "User ID") and code uses specific type (`UUID`).
*   **Recovery**: User adds suppression comment `// coherence-ignore`.

## Human Touchpoints
*   **Report Review**: The user reads the generated Markdown report.
*   **Configuration**: User edits the config file to tune sensitivity.
