# Agent: The Polyglot Steward

## Core Tools
`memory`, `web`, `grep`, `filesystem`

## Problem Domain
Software Internationalization (i18n) and Localization (l10n) are often fragmented. Developers hardcode strings; translators lack context; inconsistencies creep in (e.g., using "Sign In" vs "Log In" randomly). Traditional tools are stateless dictionaries.

## Key Insight
**"Context-Aware Translation Memory as a Graph"**.
Instead of a flat key-value pair (e.g., `login_btn: "Log In"`), the agent maintains a **Semantic Knowledge Graph** of the application's language. It maps:
*   **Terms** to **Concepts** (e.g., "Log In" -> *Authentication Entry*).
*   **Concepts** to **Code Contexts** (e.g., *Authentication Entry* is used in `Header.tsx` and `LoginForm.js`).
*   **Concepts** to **Cultural Nuances** (e.g., "Avoid 'Abort' in accessible UIs").

This allows the agent to enforce consistency ("You used 'Sign In' here, but the rest of the app uses 'Log In'") and provide translators with deep context ("This string appears next to a destructive action").

## Architecture

### 1. The Discovery Loop (grep + filesystem)
*   **Trigger**: Weekly schedule or Pre-commit hook.
*   **Action**: Scans codebase for string literals.
*   **Logic**:
    *   If string matches an existing **Concept** in Memory (e.g., "Cancel"), suggest the existing key.
    *   If new, analyze surrounding code (using `grep -C`) to infer context (button? error message?).

### 2. The Validation Loop (web + memory)
*   **Trigger**: New keys added to `en.json`.
*   **Action**:
    *   Checks the **Memory Graph** for "Banned Terms" or "Style Guide Violations".
    *   Uses **Web Search** to validate cultural safety for target locales (e.g., "Is this phrase idiomatic in Brazilian Portuguese?").
*   **Output**: Annotates the translation file with "Steward Notes" (e.g., "Context: Button. Tone: Formal. Warning: Do not translate as 'Cancelar' in this context, use 'Voltar'.").

### 3. The Synchronization Loop (filesystem)
*   **Action**: Updates the `glossary.md` file in the repo to reflect the current state of the Memory Graph, ensuring humans have a readable reference.

## Persistence Strategy
*   **Primary**: Memory Graph for the complex relationships (Term <-> Context <-> Cultural Rule).
*   **Secondary**: Filesystem for the actual code and i18n files (JSON/YAML) and a generated `GLOSSARY.md`.

## Autonomy Level
**Checkpoint-based**.
The agent actively monitors and prepares work (extracts strings, adds notes, flags issues), but strictly **does not commit changes** to source code without human ratification (e.g., via a Pull Request comment or a generated report file).

## Failure Recovery
*   **Ambiguity**: If it can't decide between "Sign In" and "Log In", it creates a "Decision Request" observation in Memory and flags it for the human.
*   **Graph Drift**: If the codebase changes significantly, it rebuilds the graph by re-indexing the existing i18n files.
