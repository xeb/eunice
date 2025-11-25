# Design 2: The Ontology Gardener (Balanced)

## Purpose
To act as a "Living Dictionary" that grows with the codebase. Instead of just blocking bad names, it actively maintains a `GLOSSARY.md` file and proactively asks clarifying questions when new terms appear, keeping code and documentation in sync.

## Core Loop
1.  **Monitor:** Watches for file changes (filesystem events).
2.  **Analysis:** When a new Noun/Entity appears (e.g., `class Subscriber`):
    *   Checks the **Memory Graph**.
    *   If unknown, asks the user (CLI/Inbox): "Is 'Subscriber' a new concept or a synonym for 'Customer'?"
3.  **Sync:**
    *   If 'Synonym': Suggests refactoring `Subscriber` -> `Customer`.
    *   If 'New Concept': Adds to Memory Graph and appends definition to `GLOSSARY.md`.
4.  **Audit:** Periodically scans `docs/` folder. If docs say "Users log in" but code says `Customer.login()`, it flags the inconsistency.

## Tool Usage
*   `memory`: Stores the "Ubiquitous Language" graph (Synonyms, Relationships, Definitions).
*   `filesystem`: Reads/Writes `GLOSSARY.md` and source code.
*   `grep`: Finds usage contexts to infer meaning.
*   `text-editor`: Inserts DocStrings or updates Markdown files.

## Memory Architecture
*   **Graph:**
    *   Nodes: `Term` (e.g., "Order"), `Context` (e.g., "Billing").
    *   Edges: `is_synonym_of`, `deprecated_by`, `related_to`.
*   **Persistence:** Serializes the graph to a local JSON file for portability, but loads into Memory server for query efficiency.

## Failure Modes
*   **Nagging:** Asking about every temp variable.
    *   *Recovery:* Heuristics to only care about *Public Interfaces* (Classes, Public Functions, DB Tables).
*   **Drift:** Graph gets out of sync with code if agent is turned off.
    *   *Recovery:* "Full Re-indexing" mode on startup.

## Human Touchpoints
*   **Clarification:** Answering "New vs Synonym" questions.
*   **Approval:** Reviewing auto-generated `GLOSSARY.md` updates.

## Pros/Cons
*   **Pros:** Keeps documentation alive, educates developers, flexible.
*   **Cons:** Requires user interaction, complex parsing logic.
