# Design 3: The Semantic Enforcer (Radical)

## Purpose
To autonomously enforce "Semantic Purity". It treats the Codebase as a projection of the Domain Model. If the Domain Model changes (via an update to the Memory Graph), the Agent autonomously refactors the entire codebase to match.

## Core Loop
1.  **Directive:** Human updates the `Domain Definition` (e.g., "We are renaming 'User' to 'Pilot'").
2.  **Impact Analysis:**
    *   Agent scans all code, comments, config files, and DB schemas using `grep` and `filesystem`.
    *   Builds a dependency graph of where `User` is used.
3.  **Execution:**
    *   Uses `text-editor` to perform massive, multi-file refactoring.
    *   Updates variable names (`user_id` -> `pilot_id`).
    *   Updates comments ("Returns the user" -> "Returns the pilot").
    *   Updates filenames (`UserFactory.ts` -> `PilotFactory.ts`).
4.  **Verification:** Runs project tests. If fail, attempts to fix.
5.  **Commit:** Pushes a PR: "Refactor: Enforce Domain Terminology (User -> Pilot)".

## Tool Usage
*   `memory`: The "Single Source of Truth" for the Domain.
*   `grep_advanced_search`: Locating all semantic references (case-insensitive, comments included).
*   `filesystem_move_file`: Renaming files.
*   `text-editor`: Applying patches.
*   `shell`: Running tests.

## Memory Architecture
*   **Authoritative Graph:** The Memory Graph is not a reflection of the code; the **Code is a reflection of the Graph**.
*   **Relations:** `Entity -> ImplementedBy (File)`, `Term -> EnforcedPattern (Regex)`.

## Failure Modes
*   **Catastrophic Refactor:** Breaking string literals or external APIs that *look* like the term but aren't.
    *   *Recovery:* Strict scope limiting (e.g., "Only refactor inside `src/domain/`").
*   **Merge Conflicts:** Touching every file in the repo.
    *   *Recovery:* Atomic, batched changes.

## Human Touchpoints
*   **Trigger:** Human initiates the change.
*   **Review:** Massive PRs require careful review.

## Pros/Cons
*   **Pros:** True semantic consistency, enables "Renaming" as a strategic maneuver.
*   **Cons:** High risk, "Scary" to run, requires perfect tests to be safe.
