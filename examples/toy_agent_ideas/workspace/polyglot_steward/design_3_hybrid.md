# Design 3: The Full-Cycle Localization Daemon (Hybrid)

## Purpose
A comprehensive, background daemon that acts as the "Steward" of the localization lifecycle. It detects new strings in code, extracts them, checks the Memory Graph for existing consistent translations, and flags potential cultural issues before they reach a human translator.

## Core Loop
1.  **Watch & Trigger**: Monitors the filesystem for `git commit` or file save events.
2.  **Differential Scan**: When a file changes, it runs `grep` to find *new* string literals.
3.  **Graph Lookup**:
    *   Query `memory` for similar strings (e.g., "Confirm" vs "OK").
    *   If a preferred term exists in the graph, it suggests using the existing key.
4.  **Auto-Extraction**: If it's a new concept:
    *   Extracts string to `en.json`.
    *   Generates a key name based on context (e.g., `auth.login.button_label`).
    *   Creates a `ToDo` entity in Memory for other languages.
5.  **Context Injection**: Adds metadata to the translation file:
    *   "Found in `LoginComponent.tsx` line 45 near 'password field'."
    *   "Similar to existing key `auth.signup.button_label`."

## Tool Usage
*   **grep**: Identification of strings.
*   **filesystem**: Reading code, updating JSON/PO files.
*   **memory**: Maintaining the "Glossary of Terms" and "Translation Memory" (Concept -> Translations).
*   **web**: Optional fallback for initial machine translation or cultural checks.

## Memory Architecture
*   **Nodes**: `Concept` (abstract idea), `Key` (technical ID), `String` (literal text), `Context` (code location).
*   **Edges**: `Key` *implements* `Concept`. `Context` *uses* `Key`. `Concept` *has_translation* `String` (with property `lang="es"`).
*   **Benefit**: If "Submit" changes to "Send" in English, the graph knows which other languages need updates because they are linked to the *Concept*, not just the string.

## Failure Modes
*   **Key Collision**: Generating a key that already exists but means something else.
*   **Code Breakage**: Auto-replacing code with variable calls might break syntax if not parsed correctly (AST vs Regex).

## Human Touchpoints
*   **Approval Queue**: The agent creates a "Pull Request" or a separate "diff" file. It never overwrites source code directly without a staging step.
