# Design 1: The String Detective (Conservative)

## Purpose
A purely analytical agent focused on code hygiene. It identifies hardcoded strings in source code that need localization and verifies that existing keys in i18n files are actually used. It acts as a specialized linter that prepares the ground for human translators.

## Core Loop
1.  **Scan**: Periodically (or on demand) iterate through source code files (JS/TS, Python, etc.) using `grep` patterns to identify string literals (e.g., text inside `<div>`, strings passed to alert functions).
2.  **Verify**: Cross-reference found strings against existing localization files (JSON/YAML) to see if they are already handled.
3.  **Report**: Generate a "Localization Debt" report:
    *   Hardcoded strings needing extraction.
    *   Unused keys in translation files (zombie keys).
    *   Inconsistent naming conventions for keys.
4.  **Wait**: Pause until the next scheduled run or user trigger.

## Tool Usage
*   **grep**: The primary sensor. Uses regex to find strings like `label="[^"]+"` or `>([^<]+)<`.
*   **filesystem**: Reading source files and writing the report.
*   **memory**: (Minimal) Stores the "Ignore List" of strings that shouldn't be translated (e.g., technical IDs, CSS classes).

## Memory Architecture
*   **Filesystem-based**: Uses a `.i18nignore` file to track false positives.
*   **Memory Graph**: Not strictly necessary for this variant, keeping it lightweight.

## Failure Modes
*   **Regex Fragility**: Misses complex string interpolations or matches non-text data (e.g., regex patterns strings).
*   **Context Blindness**: Cannot determine if "Save" is a verb or a noun without deeper parsing.

## Human Touchpoints
*   **Review**: Human must manually extract the reported strings. The agent is read-only regarding code.
