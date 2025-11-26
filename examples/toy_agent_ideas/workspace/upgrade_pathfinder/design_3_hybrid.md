# Design 3: The Contextual Debt Negotiator (Hybrid)

## Purpose
To integrate technical debt management into the daily workflow. Instead of a separate "chore" week, the agent monitors active development and provides "Just-in-Time" upgrade paths for the files currently being modified.

## Loop Structure
1. **Monitor:** Watches the filesystem for file modification events (or polls git status).
2. **Contextualize:** When User edits auth_service.py:
   - Agent identifies dependencies of auth_service.py (e.g., flask-jwt).
   - Checks memory graph: Is flask-jwt flagged as "High Debt"?
3. **Evaluate:**
   - If High Debt: Performs a quick background check (lightweight version of Shadow Upgrader).
   - Can we upgrade flask-jwt easily?
4. **Nudge:**
   - If yes: Drops a NOTE.md alongside the file or sends a notification: "Since you're editing auth_service.py, here is a tested patch to upgrade flask-jwt. It passes tests."
   - If no: Updates the memory graph with the specific conflict found in this file.

## Tool Usage
- **memory:** The central brain. Stores the graph of File -> imports -> Library -> Risk Score.
- **grep/filesystem:** To parse imports and watch file changes.
- **shell:** To run isolated tests in the background.
- **text-editor:** To insert non-intrusive comments/notes if permitted.

## Memory Architecture
- **Graph:**
  - Nodes: File, Library, Author, RiskFactor.
  - Edges: EDITED_BY, DEPENDS_ON, BLOCKED_BY.
- **Optimization:** Queries the graph to find "High Leverage" files (files that, if fixed, unlock upgrades for many other files).

## Failure Modes
- **Annoyance:** Nudging too often. -> Recovery: "Cool-down" period per library (don't suggest same upgrade for 1 week).
- **Performance:** Slowing down the dev machine. -> Recovery: Run low-priority background threads (nice level).

## Human Touchpoints
- **Opt-in:** The agent suggests; the human accepts. It feels like a "Clippy" for Architecture, but helpful.
