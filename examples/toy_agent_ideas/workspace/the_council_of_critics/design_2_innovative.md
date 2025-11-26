# Design 2: The Living Electorate (Political Persistence)

## Purpose
An autonomous background daemon that maintains a "Social Graph" of synthetic stakeholders (The Council). These critics don't just review code; they *remember* how you treated their previous feedback. If you consistently ignore accessibility, the "Accessibility Advocate" loses trust in you and becomes more vocal/obstructionist.

## Loop Structure
1. **Observation**: Agent watches for file saves (`filesystem` polling or git hooks).
2. **Council Session**:
    *   Agent queries `memory` for active Personas and their current "Trust Score" with the user.
    *   Relevant Personas (based on file type) "discuss" the change in an internal scratchpad.
    *   They vote on severity: `Notice`, `Warning`, or `Blocker`.
3. **Intervention**:
    *   **Low Severity**: Adds a comment to a `council_log.md`.
    *   **High Severity**: Directly injects a comment into the code: `// COUNCIL BLOCK (Security): You ignored the SQL injection warning 3 times. We are not letting this pass.`
4. **Reconciliation**:
    *   When the user fixes the issue, they run `council resolve <id>`.
    *   The Agent verifies the fix.
    *   If fixed, Trust Score increases. If deleted without fix, Trust Score plummets.

## Tool Usage
*   **memory**: Stores the "Social Graph".
    *   *Nodes*: `Persona:Security`, `Persona:UX`, `User`.
    *   *Edges*: `trusts`, `distrusts`, `has_outstanding_issue`.
    *   *Observations*: "User ignored warning #451 on 2025-11-25".
*   **web**: The Personas search the web to "radicalize" themselves or get new arguments.
    *   *Example*: The "Performance Critic" searches "React 19 performance pitfalls" to find new things to complain about.
*   **filesystem**: Read code, inject comments.

## Memory Architecture (The Graph)
*   **Trust Dynamics**: Trust decays over time if ignored. Trust is gained by "active listening" (making changes that align with persona goals).
*   **Coalitions**: Critics can form edges between themselves (`Security` allies with `Legal` against `User`).

## Failure Modes
*   **Mutiny**: The Council becomes so distrustful that they block everything.
*   **Recovery**: User must "bribe" the council (reset memory or perform a dedicated "cleanup task" to regain trust).

## Human Touchpoints
*   User negotiates with the council via `council_log.md` (writing replies to the bot).
*   User must actively "maintain relationships" with these synthetic entities.
