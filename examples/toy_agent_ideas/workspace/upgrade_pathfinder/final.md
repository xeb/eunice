# The Upgrade Pathfinder (Synthesized Design)

## Core Concept
**"Just-in-Time Technical Debt Repayment"**
The Upgrade Pathfinder is a background agent that monitors your active development context (which files you are editing) and cross-references them with a "Debt Graph" of outdated dependencies. When you touch a file that depends on a "High Interest" library, the agent proactively attempts an upgrade in a shadow environment. If successful, it offers a "One-Click Upgrade" patch immediately, allowing you to pay down debt while you are already working on the code.

## Architecture

### 1. The Surveillance Loop (Low Cost)
- **Role:** Monitor filesystem and maintain the Debt Graph.
- **Tools:** `filesystem`, `grep`, `memory`.
- **Action:** 
  - Periodically scans `package.json` / `requirements.txt`.
  - Uses `web` to fetch latest versions/CVEs.
  - Updates `memory` graph: `Library(name, current_v, latest_v, risk_score)`.
  - Watches for file edits.

### 2. The Shadow Proving Ground (High Cost, On-Demand)
- **Role:** Verify upgrades safely.
- **Trigger:** User edits `src/utils/date_formatter.js` (which uses `moment.js`).
- **Tools:** `shell` (cp, npm install, npm test), `text-editor` (apply heuristic fixes).
- **Action:**
  - Clones project to `/tmp/shadow_workspace`.
  - Attempts `npm install moment@latest`.
  - Runs tests relevant to `src/utils/date_formatter.js` (using `grep` to find tests importing this module).
  - If tests pass: Marks upgrade as "Ready".
  - If tests fail: Analyzes error, searches web for "moment migration guide", attempts simple regex replacements, retries.

### 3. The Interaction Layer (The Nudge)
- **Role:** Deliver value without annoyance.
- **Action:**
  - If an upgrade is "Ready" and the user is editing a dependent file, creates a non-intrusive notification (e.g., a `PATHFINDER.md` file in the directory or a terminal log if attached).
  - "Since you are editing `date_formatter.js`, I have verified that upgrading `moment.js` is safe. Run `./scripts/apply_upgrade_moment.sh` to apply."

## Memory Graph Structure
- **Nodes:**
  - `Library` (Attributes: name, version, deprecation_status)
  - `File` (Attributes: path, last_edited)
  - `Blocker` (Attributes: error_message, hash)
- **Edges:**
  - `File --imports--> Library`
  - `Library --has_upgrade--> Version`
  - `Version --blocked_by--> Blocker`
  - `Blocker --found_in--> File`

## Key Insight
Most technical debt tools fail because they are "out of band" (dashboards no one looks at). By coupling debt repayment to *active feature work*, the cost of context switching is minimized. The agent acts as a "scout" that runs ahead on the upgrade path while you walk the feature path.

## Tools Required
- **filesystem:** For monitoring and reading code.
- **grep:** For identifying dependencies and test files.
- **shell:** For running the shadow build/test process.
- **web:** For fetching changelogs and debugging build errors.
- **memory:** For persisting the graph of what upgrades are possible/blocked.
- **text-editor:** For applying migration patches.

## Autonomy Level
**Checkpoint-Based High Autonomy.**
The agent autonomously identifies, researches, and tests upgrades. It stops short of committing to the main branch, instead providing a "pre-validated offering" to the user.
