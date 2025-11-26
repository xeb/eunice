# Final Design: The Invariant Hunter

## Executive Summary
**The Invariant Hunter** is an autonomous "Quality Assurance Archaeologist" that bridges the gap between *what we say* (documentation/comments) and *what we enforce* (tests/types). It uses a **"Scan-Synthesize-Verify"** loop to automatically generate Property-Based Tests (PBT) from natural language artifacts.

## Core Value Proposition
- **Automated Regression Defense**: Turns "comments" into "executable tests" that prevent future breakages.
- **Living Documentation**: Flags when docs drift from reality (e.g., "Docs say positive, but code allows -1").
- **Constraint Discovery**: Surfaces implicit assumptions that developers forgot to document.

## Architecture

### 1. The Mining Loop (Passive)
- **Tools**: `grep`, `filesystem`, `memory`
- **Action**: Continuously scans the codebase for "Constraint Signals":
    - **Comments**: "Must be unique", "Should be sorted", "Invariant:"
    - **Assertions**: `assert(x > 0)`, `if (!user) throw ...`
    - **Types**: `type Email = string` (weak constraint) vs `class Email { ... }` (strong constraint)
- **State**: Builds a **Constraint Graph** in `memory`:
    - Nodes: `Entity`, `Property`, `Constraint`
    - Edges: `DocumentedIn`, `EnforcedBy`
    - *Example*: `Entity("User") -> hasProp("email") -> Constraint("valid_regex") -> DocumentedIn("User.ts comment")`

### 2. The Synthesis Loop (Active)
- **Tools**: `filesystem` (read types), `web` (PBT library docs), `text-editor` (write tests)
- **Action**: For every "Phantom Constraint" (Documented but not Enforced):
    1. **Analyze Context**: Reads surrounding code to understand types/imports.
    2. **Draft Test**: Generates a Property-Based Test (using `fast-check` or `hypothesis`) that asserts the constraint holds for *all* valid inputs.
    3. **Sandbox Run**: Executes the test in a temporary file.
- **Outcome**:
    - **Pass**: The code adheres to the comment. **Action**: Propose adding the test to `tests/generated/`.
    - **Fail**: The code violates the comment. **Action**: Create a "Drift Report" (Bug or Outdated Doc).

### 3. The Persistence Layer
- **Memory Graph**: Stores the "Truth" of the system.
    - Prevents re-generating failed tests loop.
    - Tracks "Flaky" constraints.
- **Filesystem**:
    - `workspace/invariant_hunter/proposals/`: Generated tests waiting for review.
    - `workspace/invariant_hunter/reports/`: Markdown reports of drift.

## Safety & Autonomy
- **Bounded Autonomy**: The agent *never* edits production code directly. It only proposes *new* test files.
- **Safe Execution**: Tests are run in a distinct process.
- **Self-Correction**: If a generated test fails to compile (syntax error), the agent reads the error log and retries the generation (max 3 attempts).

## Implementation Roadmap
1. **Phase 1**: "The Loud Librarian" - Only scans and reports mismatches.
2. **Phase 2**: "The Ghost Writer" - Generates passing tests for simple constraints (numeric ranges, null checks).
3. **Phase 3**: "The Logic Master" - Handles complex multi-entity invariants (e.g., "Order total must equal sum of items").

## MCP Toolset
- **memory**: Graph database for the "Constraint Topology".
- **filesystem**: Reading code, writing proposals.
- **shell**: Running test runners (`npm test`, `pytest`).
- **grep**: Fast searching for keywords.
- **web**: Looking up PBT syntax and library updates.
