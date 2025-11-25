# Agent Design: The Software Immunologist

## Executive Summary
The **Software Immunologist** is an autonomous background agent that acts as an immune system for software projects. Unlike traditional dependency bots (Dependabot/Renovate) that mechanically open PRs, the Immunologist **learns** from every update attempt. It uses a persistent memory graph to track the stability of packages, remembers compile errors and their fixes ("antibodies"), and proactively inoculates the codebase against known software supply chain risks.

## Core Toolset
- **filesystem:** To read manifests (`package.json`, `Cargo.toml`) and apply patches.
- **shell:** To execute builds, tests, and git operations in an isolated sandbox (branches).
- **web (Brave):** To research changelogs, CVEs, and migration guides when errors occur.
- **memory:** To persist the "Health Graph"—a reputation database of packages, versions, and compatibility scores.

## Architecture & Loop

### 1. Sampling (Detection)
The agent periodically scans the filesystem to identify outdated dependencies. It queries the `memory` graph to check the "Stability Score" of available updates.
- **Low Risk:** High stability score in memory + no known CVEs (Web). -> **Proceed to Testing.**
- **High Risk:** Known breakage in other local projects or low score. -> **Defer/Block.**

### 2. Clinical Trial (Isolation & Testing)
The agent creates a dedicated branch (e.g., `immunity/upgrade-react-19`).
- Runs the update command via `shell`.
- Executes the project's test suite.

### 3. Diagnosis (Reasoning)
If tests fail, the agent captures the error logs (STDERR).
- **Memory Check:** "Have I seen this error before for this library?"
- **Web Research:** If the error is new, search: `"LibName vX.Y.Z error [Error String]"`.
- **Hypothesis Generation:** The agent formulates a fix (e.g., "Rename import `foo` to `bar`").

### 4. Immune Response (Self-Healing)
- **Antibody Application:** The agent uses `text-editor` or `filesystem` to apply the fix.
- **Re-Test:** Runs tests again.
- **Success:** Records the fix strategy in `memory` (creating an "Antibody" node linked to this upgrade path). Commits and pushes the PR.
- **Failure:** If unfixable, reverts changes, deletes branch, and marks the version as "Toxic" in `memory`.

### 5. Memory Persistence (Long-term Immunity)
The memory graph becomes a valuable asset.
- **Nodes:** `Package`, `Version`, `ErrorPattern`, `FixStrategy`.
- **Relations:**
  - `Version A` -> `BREAKS` -> `Test Suite`
  - `Fix Strategy B` -> `RESOLVES` -> `Error Pattern C`
- **Benefit:** When the agent encounters the same upgrade in a *different* project, it instantly knows how to fix it by retrieving the "Antibody" from memory.

## Use Case Example
**Scenario:** A breaking change in `axios` v2.0 renames a method.
1. **Project A:** Agent tries to update. Tests fail. Agent searches web, finds the migration guide. Applies the rename. Tests pass. Records: "Axios v1->v2 requires rename X->Y".
2. **Project B:** Agent detects `axios` update. Checks memory. Sees the "Antibody". Applies the update AND the rename immediately. Tests pass first try.

## Failure Modes & Recovery
- **Runaway Updates:** Agent might try to update a broken package endlessly.
  - *Fix:* "Cool-down" period in memory. If an update fails, don't retry for 7 days unless a new version is released.
- **Bad Fixes:** Agent "fixes" the code by deleting the failing test.
  - *Fix:* Strict rule: net reduction in test count is forbidden.

## Deployment Strategy
Run as a daily cron job or CI pipeline step. It requires access to the repo (filesystem) and a persistent store (memory graph file or database).

