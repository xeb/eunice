# Agent: The Socio-Technical Architect (aka "The Conway's Law Sentinel")

## Purpose
Large software projects often suffer from "Conway's Law Mismatches"—where the code structure clashes with the organization structure (e.g., tight coupling between teams that rarely talk). This leads to slow integration, bugs, and "siloed" knowledge.

The **Socio-Technical Architect** is an autonomous agent that maps both the **Code Dependency Graph** and the **Contributor Interaction Graph** to identify these mismatches. It acts as a bridge, flagging "Socially Risky" code changes and proposing architectural refactors to align the code with the team reality.

## Core Loop
1.  **The Cartography Phase (Nightly/Background):**
    *   **Social Mapping:** Mines `git log` and PR history to build a weighted graph of "Who works with Whom" and "Who owns What".
    *   **Technical Mapping:** Uses `grep` and file analysis to map "Module Dependencies" (Imports/Calls).
    *   **Congruence Calculation:** Overlays the two graphs. Calculates a **"Congruence Score"** for every dependency edge.
        *   *Congruent:* File A depends on File B, and Author(A) talks to Author(B).
        *   *Incongruent:* File A depends on File B, but the author groups are disjoint (Risk!).
    
2.  **The Watchdog Phase (On-Change):**
    *   When a user modifies a file, the agent checks the "Social Cost" of the new dependencies.
    *   **Alert:** "⚠️ You are importing `billing/core.ts`. You have 0 social overlap with the Billing Team. This API is unstable. Recommended: Chat with @alice first."

3.  **The Gardening Phase (Weekly):**
    *   Identifies "Orphans" (Code with no active maintainers).
    *   Identifies "Tangled Zones" (Directories with low internal cohesion).
    *   Generates a **"Socio-Technical Health Report"** with specific refactoring proposals (e.g., "Move `shared/utils` to `team-a/utils` as they are the only users").

## Tools
*   **shell:** Essential for `git log`, `git diff`, and running build verifications.
*   **memory:** Stores the persistent "Socio-Technical Graph" (Nodes: People, Files, Teams; Edges: Imports, Edits, Reviews).
*   **filesystem:** Reads code to parse imports; writes reports (`CONWAY_REPORT.md`).
*   **grep:** Fast retrieval of dependencies and references.

## Memory Architecture
*   **Graph-First:** The `memory` tool is crucial here. The graph needs to be persistent because "Social Links" decay slowly, whereas code changes fast.
*   **Entities:**
    *   `Developer` (Name, ActiveHours, CoreModules)
    *   `Module` (Path, Complexity, OwnerGroup)
    *   `Team` (Inferred cluster of Developers)
*   **Relations:**
    *   `DEPENDS_ON` (Technical)
    *   `COLLABORATES_WITH` (Social)
    *   `OWNS` (Social-to-Technical)

## Failure Modes & Recovery
1.  **False Positives (The "Typos" Problem):** A dev fixing a typo in a file shouldn't be marked as an "Owner".
    *   *Recovery:* Use "Recency-Weighted Line Count" instead of just commit count.
2.  **Monorepo Scale:** Parsing git history for a massive repo is slow.
    *   *Recovery:* Incremental parsing. Store the last processed commit hash in Memory. Only process new commits.
3.  **Privacy:** Developers might not want their "Social Graph" mapped.
    *   *Recovery:* Local-only mode. Anonymize output (Dev A, Dev B).

## Human Touchpoints
*   **The "Introduction":** When the agent detects a risky dependency, it effectively "Introduces" the two developers via a report or comment.
*   **Refactoring Approval:** The agent proposes moves (Design 3), but the human must execute them to resolve conflicts.
