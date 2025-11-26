# Agent: The Compliance Cartographer

## Purpose
An autonomous legal guardian that treats software license compliance not as a checklist, but as a **Viral Graph Problem**. It maps the flow of intellectual property rights through the dependency tree, identifies "Infection Vectors" (e.g., a GPL library deep in the stack), and proactively generates remediation strategies or isolation boundaries.

## Core Toolset
- **memory:** Stores the "Legal Knowledge Graph" (Package -> License -> Viral Risk).
- **filesystem:** Scans manifests, reads/writes `NOTICES`, creates graph visualizations.
- **web:** Searches for missing license text, verified alternatives, and maintainer reputation.
- **grep:** Analyzes code for deep linking (static vs dynamic) to determine viral risk.

## Architecture

### 1. The Legal Graph (Memory)
The agent maintains a persistent graph where:
- **Nodes:** `Package`, `License`, `File`, `Maintainer`.
- **Edges:**
  - `DEPENDS_ON` (with metadata: `type=dev|prod`, `linkage=static|dynamic`)
  - `HAS_LICENSE`
  - `VIRAL_RISK` (Weighted edge: 0.0=MIT to 1.0=AGPL)
  
This graph allows the agent to answer questions like: *"Which production features depend on the GPL library 'foo'?"* or *"What is the shortest path to remove the AGPL risk?"*

### 2. The Autonomy Loop (Background Daemon)
1. **Watch & Scan:** Monitors `package.json`, `go.mod`, etc. On change, it updates the Graph.
2. **Risk Analysis:**
   - Detects **"License Drift"** (a patch update changing a license).
   - Identifies **"Viral Contamination"** (Proprietary code statically linking GPL).
3. **Strategic Remediation:**
   - If a violation is found, it does *not* just break the build.
   - It **searches** for alternatives (Design 2).
   - It **drafts** an "Isolation Wrapper" proposal (e.g., "Move this logic to a separate service").
   - It **generates** the `NOTICES` file automatically.
4. **Reporting:** Updates a `LEGAL_HEALTH.md` dashboard in the repo root.

## Human Touchpoints
- **Policy Config:** User defines `policy.yaml` (e.g., "Allow GPL if usage is Test-Only").
- **Review:** The agent opens Issues/PRs with "Legal Risk Alerts" requiring human sign-off.
- **Override:** Humans can tag a dependency as `exempt` in the configuration.

## Failure Modes & Recovery
- **Unknown Licenses:** If a license text is custom or unrecognized, the agent flags it as "Review Required" and provides a link to the text.
- **Network Failure:** Falls back to the local Memory Graph (cached data).
- **False Positives:** The agent uses `grep` to check *how* the library is used (import vs exec) to refine risk scores (e.g., LGPL is fine if not modified/linked statically).

## Key Insight
**Compliance as Topology**: Most tools view dependencies as a list. This agent views them as a **Network of Rights**. By modeling the *linkage type* (static/dynamic/RPC) in the graph, it can distinguish between a "hard violation" (GPL static link) and a "safe usage" (GPL over API), providing nuanced, actionable advice rather than binary failures.
