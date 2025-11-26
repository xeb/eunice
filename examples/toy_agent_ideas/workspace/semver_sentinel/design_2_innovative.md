# Design 2: The "Semantic Impact Graph" (Innovative)

## Purpose
An agent that *ignores* commit messages and instead analyzes the *code itself* to determine the semantic version bump. It builds a graph of the "Public API" (exported functions, classes, types) and detects breaking changes, additions, or internal-only fixes mathematically.

## Loop Structure
1.  **Snapshot Graph:** On every commit, parse the codebase (using `grep` or language-specific AST tools via shell) to extract "Exported Nodes" (Public API).
2.  **Graph Diff:** Compare the current Public API Graph with the previous version's Graph.
    *   *Node Missing?* -> **MAJOR** (Breaking Change)
    *   *Node Signature Changed?* -> **MAJOR** (Breaking Change)
    *   *New Node Added?* -> **MINOR** (Feature)
    *   *Graph Identical (but file hash changed)?* -> **PATCH** (Internal Fix)
3.  **Auto-Draft Release:** Generate a `CHANGELOG.md` entry that describes *exactly* what changed in the API (e.g., "Function `fetchUser` removed argument `id`").
4.  **Propose Tag:** Suggest the new version number to the user.

## Tool Usage
*   **Grep/Shell:** To parse code and extract function signatures/exports (e.g., `grep "export function"` or `ctags`).
*   **Memory:** Store the "Public API Graph" of the *previous* release to enable fast comparison without re-parsing old code.
*   **Filesystem:** Write the detailed, accurate `CHANGELOG.md`.

## Memory Architecture
*   **API Graph:** Nodes = Exported Functions/Classes. Edges = Dependencies (optional, but good for impact analysis).
*   **Version History:** A linked list of Graphs for every version.

## Failure Modes
*   **Dynamic Exports:** Languages like Python/JS allow dynamic exports (`module.exports = ...`) which are hard to statically analyze.
*   **False Negatives:** A change in *behavior* (logic bug) that doesn't change the *signature* (types) might look like a Patch but break consumers.

## Human Touchpoints
*   **Confirmation:** The agent says "I detected a Breaking Change in `auth.ts`. Bumping to 2.0.0. Proceed?"
*   **Overrides:** Humans can force a version bump if logic changed invisibly.
