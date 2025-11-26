# Agent: The Shadow Pioneer
### *The Counterfactual Navigation Engine*

## Purpose
The Shadow Pioneer is a background daemon that continuously explores the "adjacent possible" of your codebase. While you work on features, it works on *futures*. It creates shadow branches to speculatively apply upgrades, refactors, and migrations, verifying them with tests/benchmarks, and populating a "Possibility Graph" in memory.

When you ask "Can we upgrade to Node 20?", it doesn't guess—it *knows*, because it tried it 4 hours ago and logged the specific 3 files that failed.

## Core Loop
1. **Frontier Scanning:**
   - Observes `package.json`, `.nvmrc`, and code patterns (using `grep`).
   - Queries Web Search for "newer versions", "migration guides", or "performance idioms".
2. **Speculative Execution (The "Shadow Realm"):**
   - Spawns a detached process (low priority).
   - Creates a temporary git worktree (to avoid locking the user's working directory).
   - Applies the change (e.g., `npm install react@latest`).
3. **Verification & Metric Collection:**
   - Runs:
     - **Unit Tests** (Correctness)
     - **Build Size** (Performance)
     - **Linter** (Style)
   - Captures the *delta* (e.g., "Tests Passed, Bundle Size -15%").
4. **Graph Update:**
   - Nodes: `CommitHash` (Current) -> `SpeculativeState` (Node 20 upgrade).
   - Edge: Contains the `patch` required to get there and the `verification_report`.
5. **Notification:**
   - If a high-value state (e.g., Security Fix + Tests Pass) is reached, it alerts the user: "Path Verified: Upgrade OpenSSL via PR #42."

## Tool Usage
- **shell:** `git worktree`, `npm/cargo/pip`, `make test`.
- **memory:** Stores the "Multiverse Graph" (Nodes = Code States, Edges = Transformations).
- **web:** Brave Search for changelogs, breaking changes, and codemods.
- **filesystem:** Reading config, applying patches.

## Persistence Strategy
- **Memory Graph:** Holds the topology of possible futures (The "Map").
- **Filesystem:** Stores the actual patch files/diffs for the edges in `.shadow_pioneer/patches/`.

## Autonomy Level
**Background Daemon.** It runs completely autonomously in the background. It only asks for attention when it finds a "Gold" state (Verified Improvement).

## Key Insight: "Counterfactual Intelligence"
Most tools tell you what your code *is*. This agent tells you what your code *could be*. By doing the dirty work of "trying and failing" in the background, it removes the fear of experimentation.

## Failure Modes & Recovery
- **Resource Hog:** Compiling in background slows down user. *Fix:* Detect system load or run only during user inactivity (screensaver mode).
- **Flaky Tests:** False negatives in verification. *Fix:* Retry loop; flag tests that fail in *both* main and shadow branches as "Baseline Noise".
- **Disk Usage:** Too many worktrees. *Fix:* Aggressive cleanup; ephemeral worktrees only live for the duration of the test.

## Example Scenario
User is working on `main`.
Shadow Pioneer sees `lodash` is used.
Shadow Pioneer searches: "Is lodash dead?". Result: "Prefer native Array methods."
Shadow Pioneer branches, runs `npx lodash-to-native`, runs tests.
Tests Pass. Bundle size drops 40KB.
Shadow Pioneer records this path in Memory.
User types: `@pioneer status`
Agent responds: "I found 3 optimizations. 1. Drop Lodash (-40KB, Auto-fix available). 2. Upgrade React (Blocked by Test A). 3. Fix Typo (Ready)."
