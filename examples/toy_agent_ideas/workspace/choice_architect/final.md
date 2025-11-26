# Final Design: The Choice Architect (Synthesized)

## Executive Summary
The **Choice Architect** is an autonomous background daemon that optimizes the "Path of Least Resistance" for developers. It combines the **Config Curator's** safety with the **Friction Engineer's** active scaffolding, wrapped in the **Ambient Mirror's** feedback loops. It does not nag; it *prepares the room* so that the right work is easier to do.

## Core Philosophy
**"Make the right thing the easy thing."**
1. **Pre-computation**: Don't ask the user to write boilerplate; have it ready before they ask.
2. **Dynamic Defaults**: Change default behaviors (git, linter, editor) based on context.
3. **Ambient Feedback**: Reflect state changes subtly in the environment.

## Architecture

### 1. The Context Loop (OODA)
*   **Observe**: Watch file modifications, git commits, and shell history.
*   **Orient**: Use **Memory Graph** to understand the user's current "Mode" (e.g., "Deep Refactoring", "Quick Hotfix", "Learning").
*   **Decide**: Select the appropriate "Nudge Strategy".
*   **Act**: Modify the environment.

### 2. Nudge Strategies (The Toolbelt)
The agent selects from a hierarchy of interventions:
*   **Level 1 (Ambient)**: Change terminal prompt color/suffix based on branch health (e.g., `(main|dirty|!coverage-low)`).
*   **Level 2 (Scaffolding)**: If user creates `feature.ts`, agent *immediately* creates `feature.test.ts` with imports pre-filled.
*   **Level 3 (Defaulting)**: If user is in "Refactoring Mode", agent switches VSCode `settings.json` to aggressive linting. If "Hotfix Mode", it relaxes rules.
*   **Level 4 (Friction)**: If trying to push with failing tests, agent adds a 3-second delay to the command prompt with a "Are you sure?" message (via shell alias injection).

### 3. The Memory Graph
*   **Nodes**: `UserMode`, `ProjectState`, `Nudge`, `Response`.
*   **Edges**: `Nudge` -> `TRIGGERED_BY` -> `ProjectState`. `User` -> `ACCEPTED` -> `Nudge`.
*   **Learning**: If the user constantly deletes the auto-scaffolded test files, the agent learns "User dislikes test scaffolding" and downgrades that intervention to a mere suggestion in a TODO file.

### 4. Technical Implementation
*   **Core**: Python script running as a daemon or cron job.
*   **MCP Integration**:
    *   `filesystem`: To inject `.editorconfig`, modify shell profiles, scaffold files.
    *   `grep`: To scan for "modes" (e.g., finding keywords in recent edits).
    *   `shell`: To execute git checks and system commands.
    *   `memory`: To persist user preferences (implicit) and state.

## Safety & Recovery
*   **The "Undo" Log**: Every environmental change is logged to `.choice_architect/history.log`.
*   **The "Kill Switch"**: A file `.choice_architect/OFF` disables all interventions immediately.
*   **Boundaries**: The agent never modifies *business logic code*, only *configuration, scaffolding, and environmental variables*.

## Example Scenario
1.  **Context**: Developer switches branch to `legacy-refactor`.
2.  **Agent Action**:
    *   Detects branch name.
    *   Updates `.git/config` to require signed commits (security).
    *   Updates terminal prompt to show "Refactor Mode 🧹".
    *   Pre-creates a `REFACTOR_PLAN.md` from a template if missing.
3.  **Result**: Developer feels "settled in" to the task without manual setup.
