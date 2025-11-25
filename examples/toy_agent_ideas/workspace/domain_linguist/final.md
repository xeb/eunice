# Agent: The Domain Linguist

## Purpose
To eliminate "Semantic Drift" in long-lived software projects. As codebases age, the language used in code often diverges from the language used by business experts and in documentation (e.g., Code uses `User`, Business says `Subscriber`, Docs say `Client`). The Domain Linguist acts as a **Semantic Consistency Daemon**, ensuring that the "Ubiquitous Language" of the domain is rigorously reflected in the codebase, comments, and documentation.

## Key Insight
Most "naming conventions" are enforced superficially (camelCase vs snake_case). The Domain Linguist enforces **Semantic Correctness**. It maintains a persistent **Ontology Graph** of the business domain and treats the codebase as a projection of this graph. If the graph changes (a term is renamed), the agent offers to refactor the code. If the code changes (a new term appears), the agent asks to update the graph.

## Core Components
1.  **The Ontology (Memory):** A knowledge graph storing terms, definitions, synonyms, and relationships (e.g., `Order` *has_many* `LineItems`).
2.  **The Scanner (Grep):** A background process that indexes the codebase to find usages of terms.
3.  **The Librarian (Filesystem):** Maintains a human-readable `GLOSSARY.md` that is always in sync with the code.
4.  **The Editor (Text-Editor):** A refactoring engine that can safely rename terms across files.

## Execution Loop
1.  **Surveillance (The Watchdog):**
    *   Continuously monitors file changes via `filesystem` or git hooks.
    *   Parses new/modified identifiers (Classes, Functions, DB Tables).
2.  **Drift Detection:**
    *   Compares found identifiers against the **Ontology Graph**.
    *   **Case A (New Concept):** If a new term (e.g., `class Reseller`) appears that isn't in the graph, it prompts the human: "Is 'Reseller' a new concept, or a synonym for existing term 'Distributor'?"
    *   **Case B (Violation):** If a blacklisted or deprecated term (e.g., `Customer` when `Client` is preferred) is found, it flags it.
3.  **Reconciliation (The Gardener):**
    *   **Forward Sync:** If the human confirms "Reseller" is new, the agent adds it to the Memory Graph and updates `GLOSSARY.md` with a placeholder definition.
    *   **Backward Sync:** If the human renames a term in the Graph (e.g., "Change 'User' to 'Pilot'"), the agent generates a **Refactoring Plan** (a massive multi-file patch) for review.
4.  **Reporting:**
    *   Updates a `semantic_health.json` dashboard showing "Term Coverage" (what % of code uses defined terms).

## Tool Usage
*   **memory:** Stores the `Term` nodes and `Definition` properties. This is the "Source of Truth".
*   **grep_advanced_search:** Used to find all occurrences of a term, including in comments and strings (which IDE refactors often miss).
*   **text-editor:** Applies precise patches for renaming.
*   **filesystem:** Reads/writes the markdown Glossary and source files.
*   **web_brave_web_search:** (Optional) Can search the web for industry-standard definitions of terms to suggest to the user.

## Failure Modes & Recovery
*   **Context Confusion:** Renaming `User` to `Pilot` might accidentally rename `UserInterface` to `PilotInterface`.
    *   *Mitigation:* The agent uses "Smart Grep" (AST-aware or regex boundaries) and presents a **Interactive Diff** before applying.
*   **Definition Rot:** The Glossary exists but no one reads it.
    *   *Mitigation:* The agent injects definitions into **IDE Tooltips** (via generating a specific definition file the IDE reads, e.g., `JSDoc` or `.d.ts`).

## Human Touchpoints
*   **The "Naming Ceremony":** When the agent asks for clarification on a new term.
*   **Refactoring Approval:** The agent never refactors without explicit sign-off on the generated plan.

## Why This Matters
In Domain-Driven Design (DDD), the "Ubiquitous Language" is the primary tool for complexity management. Currently, this language lives in people's heads or stale wikis. The Domain Linguist **reifies** this language into an active, code-aware agent, bridging the gap between "The Business" and "The Code".
