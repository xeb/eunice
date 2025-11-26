# Design 2: The Feasibility Engine (Innovative)

## Purpose
To actively test the *technical viability* of ideas in the backlog. Instead of just reading about an idea, this agent attempts to build a "Tracer Bullet" — a minimal script that proves the core dependency works as expected.

## Loop Structure
1. **Hypothesis Generation**: Read an idea and formulate a technical hypothesis (e.g., "Library X can parse File Format Y").
2. **Scaffold**: Create a temporary directory `prototypes/<idea_id>/`.
3. **Code Gen**: Write a small Python/Node script to test the hypothesis (e.g., `import library_x; library_x.parse(sample)`).
4. **Execute**: Run the script using `shell`.
5. **Analyze**: Capture stdout/stderr.
    - **Success**: Mark idea as "Technically Feasible". Update Memory Graph.
    - **Failure**: Log the error. Search web for fixes. If fixable, retry. If blocked by missing feature, mark as "Blocked by <Missing_Feature>".
6. **Watch**: If blocked, store the "Missing Feature" in Memory. Periodically search the web (Weekly) to see if a new version of the library was released that solves it.

## Tool Usage
- **shell**: Execute proof-of-concept scripts, install dependencies (`npm install`, `pip install`).
- **filesystem**: Create prototype files.
- **memory**: Store the "Blocker" nodes.
- **web_brave_web_search**: Find documentation and release notes.

## Memory Architecture
- **State Machine**: Ideas move from `Proposed` -> `Hypothesis` -> `Prototyping` -> `Feasible` or `Blocked`.
- **Dependency Graph**: `Idea A` -> `DEPENDS_ON` -> `Library X (v1.2)`.
- **Blocker Tracking**: `Blocker Y` -> `BLOCKS` -> `Idea A`.

## Failure Modes
- **Resource Exhaustion**: Infinite loops or massive downloads in prototypes. *Recovery*: Strict timeouts (10s) and containerization.
- **Side Effects**: Scripts modifying external systems. *Recovery*: Sandboxing (only allow network/file access to whitelisted zones).

## Human Touchpoints
- **Approval**: Human must approve network requests or extensive package installations.
- **Result**: Human receives a "Feasibility Report" (Green/Red light) before starting actual work.
