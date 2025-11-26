# Design 2: The Architect (Experimental)

## Purpose
A highly autonomous agent designed to tackle structural technical debt. It builds a persistent mental model of the codebase's architecture to identify circular dependencies, god-objects, and tight coupling, then plans and executes multi-file refactors.

## Core Loop
1. **Mapping**: Periodically scans the codebase to build/update a "Code Knowledge Graph" in the Memory MCP.
   - Nodes: Files, Classes, Functions, Modules.
   - Edges: Imports, Calls, Inherits, Instantiates.
2. **Analysis**: Queries the graph to find anti-patterns (e.g., "Node with >50 incoming edges", "Cycle detection").
3. **Planning**: Generates a refactoring plan (e.g., "Extract interface IUserService from UserService").
4. **Execution**:
   - Uses text-editor to move code, update imports, and create new files.
   - Uses shell to run intermediate tests.
5. **Validation**: Runs full integration test suite.
6. **Learning**: If a refactor fails tests, it records the failure pattern in Memory to avoid similar strategies.

## Tool Usage
*   **memory**: Heavily used. Stores the entire dependency graph and "Refactoring Plans" as entities.
*   **grep**: Searches for symbol usages across files to populate the graph.
*   **text-editor**: Performs complex, multi-line code movements and patching.
*   **shell**: Runs tests and git operations.

## Memory Architecture
*   **Graph-Based**: Persists the codebase structure.
*   **Entity Types**: CodeModule, Class, Function, DebtHotspot.
*   **Relations**: calls, imports, depends_on.
*   **Reasoning**: "If Class A and Class B are mutually dependent, mark as CircularDependency observation."

## Failure Modes
*   **Logic Regression**: Moving code might break subtle logic not covered by tests.
*   **Hallucinated Imports**: Might try to import things that don't exist.
*   **Infinite Refactor Loop**: Might endlessly toggle between two structures. Mitigated by max_attempts in plans.

## Human Touchpoints
*   **Plan Approval**: The Architect generates a "Refactoring Proposal" (Markdown). A human must approve it before execution starts.
