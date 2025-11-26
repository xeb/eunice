# Agent: The Hive-Mind Steward (formerly Cross-Pollinator)

## 1. System Overview
**The Hive-Mind Steward** is a background daemon that treats multiple local repositories as a single "distributed monolith." Its goal is to identify **Convergent Evolution** (duplicated logic) and **Configuration Drift** (inconsistent tooling) to reduce the maintenance overhead of poly-repo architectures.

It does not enforce standards top-down. Instead, it uses **Evolutionary Metrics** to visualize "Winning" patterns and encourages the "Laggard" projects to align with the majority.

## 2. Problem Domain
*   **Siloed Knowledge**: Team A writes a `date_formatter` that Team B needs but doesn't know exists.
*   **Config Drift**: 5 repos use `node:18`, 2 use `node:16`, 1 uses `node:20`. This causes CI failures.
*   **Wheel Reinvention**: Multiple implementations of the same utility function across projects.

## 3. Core Toolset
| Tool | Purpose |
|------|---------|
| `filesystem` | Traversing directories (`list_directory`) and reading code (`read_text_file`). |
| `grep` | High-speed pattern census (finding imports, config values, function definitions). |
| `memory` | Storing the "State of the Ecosystem" graph (Nodes=Projects, Edges=SharedPatterns). |
| `shell` | Running `diff`, `md5sum`, and generating patch files. |

## 4. Architectural Loop

### Phase 1: The Census (Discovery)
1.  Agent wakes up (cron or event).
2.  Reads `~/.hive-mind-config` for a list of watched directories.
3.  **Config Scanner**: Computes hashes of standard files (`package.json`, `tsconfig.json`, `.eslintrc`, `Makefile`, `Dockerfile`).
4.  **Pattern Scanner**: Uses `grep` to count usage of specific libraries (e.g., `import * from 'lodash'`, `import * from 'axios'`).

### Phase 2: The Graph Update (Memory)
1.  Updates the Knowledge Graph:
    *   `Project(A) --HAS_CONFIG_HASH--> Hash(123)`
    *   `Project(B) --HAS_CONFIG_HASH--> Hash(123)`
    *   `Project(C) --HAS_CONFIG_HASH--> Hash(999)` (Outlier)
2.  Detects **Consensus**: "Hash(123) is present in 80% of projects."

### Phase 3: The Proposal (Action)
1.  **For Drift**: If Project C is an outlier for a high-consensus config, the agent creates a **Migration Proposal** in `workspace/inbox/ProjectC_fix_config.md`.
    *   Content: "Project C is using an old `tsconfig.json`. 80% of your projects use the standard version. Run this command to update: `cp ...`"
2.  **For Logic**: If two projects have identical file hashes for a utility file (e.g., `utils/math.ts`), the agent suggests moving it to a shared library.

### Phase 4: The Dashboard (Visibility)
1.  Updates `workspace/HIVE_MIND_STATUS.md`.
2.  Visualizes the "Fragmentation Score" of the ecosystem.
    *   "Config Consistency: 85%"
    *   "Dependency Alignment: 60%"

## 5. Persistence Strategy
*   **Memory Graph**: Stores the *relationships* and *trends* (e.g., "React usage is increasing").
*   **Filesystem**: Stores the *reports* and *proposals*. The agent uses the filesystem as its UI (User Interface), dropping files into an Inbox for the human to review.

## 6. Autonomy & Human-in-the-Loop
*   **Autonomy**: High. The agent is read-only on the code but write-active on the "Meta-Layer" (Reports/Proposals).
*   **Safety**: It never modifies project code directly. It only proposes changes via Markdown files or Shell Scripts that the user must manually execute.
*   **Feedback**: The user can add a `.hiveignore` file to a project to tell the agent "This deviation is intentional."

## 7. Key Insight
**"Evolutionary Architecture as a Service."**
Instead of a strict linter that says "You are wrong," the agent says "You are lonely." It leverages social pressure (even in a single-user environment) by highlighting that a project is falling behind the user's own established norms.
