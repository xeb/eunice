# Design 2: The Friction Engineer (Innovative)

## Purpose
To actively shape behavior by adding "Friction" to undesirable actions and "Lubrication" to desirable ones. Based on Fogg Behavior Model (Behavior = Motivation + Ability + Trigger). This agent manipulates "Ability".

## Loop Structure
1. **Event Monitoring**: Watch for file system events or git hooks.
2. **Context Evaluation**:
   - Is the user trying to commit to `main` without tests? -> **High Friction**.
   - Is the user writing documentation? -> **Zero Friction** (Auto-formatting, Auto-linking).
3. **Dynamic Intervention**:
   - **Friction**: The agent *injects* a pre-commit hook temporarily that asks a reflective question: "This module has high complexity. Have you updated the tests?" (Requires 'Y' to proceed).
   - **Lubrication**: If the user creates a new function, the agent *immediately* scaffolds the unit test file next to it, removing the "activation energy" required to start testing.
   - **Visual Nudge**: If technical debt is high, the agent renames the folder `src` to `src_needs_refactor` (radical!) or simply adds a `DEBT.md` in the root that grows in size.

## Tool Usage
- **filesystem**: Rename files, create hooks, scaffold boilerplate.
- **shell**: Intercept git commands (via aliases or hooks).
- **memory**: Track "Friction Budget" (don't annoy the user too much).

## Memory Architecture
- **Graph**: `Action` -> `DesirabilityScore`.
- **State**: `UserFrustrationLevel` (inferred from `git reset` or rapid deletions).
- **Logic**: If `UserFrustration` is high, disable all Friction interventions.

## Failure Modes
- **Blocking Critical Work**: Preventing a hotfix deploy because of "process friction."
  - *Recovery*: "Emergency Brake" - a specific commit message tag (e.g., `!hotfix`) bypasses all agent interventions.
- **Rage Quit**: User disables the agent.
  - *Recovery*: Agent operates in "Silent Mode" first, logging what it *would* have done.

## Human Touchpoints
- High. The agent is an active participant in the workflow, effectively "gamifying" the constraints.
