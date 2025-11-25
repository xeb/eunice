# Final Design: The Legacy Intelligence Platform

## Executive Summary
This agent architecture combines the structural analysis of the **Code Cartographer** with the external contextual awareness of the **Contextual Anthropologist**. It creates a living knowledge graph that maps *what* the code does, *how* it does it, and *why* it was written that way. It serves as an interactive "Expert System" for onboarding new developers or planning modernization.

## Architecture

### 1. The Knowledge Core (Memory + Filesystem)
The heart of the agent is a graph database (MCP Memory) that unifies three layers of data:
*   **Code Layer**: Files, Classes, Functions, Imports (from Static Analysis).
*   **Context Layer**: Library versions, CVEs, documentation links (from Web Search).
*   **Evolution Layer**: Git history, author churn, "hotspots" of change.

### 2. The Agent Loop
The agent runs as a background daemon with three distinct phases:

#### Phase A: Cartography (Nightly)
1.  **Scan**: Walk the filesystem to identify changes.
2.  **Parse**: Use `grep` and language-specific parsers to update the Code Layer nodes.
3.  **Link**: Re-verify internal dependencies (imports/calls).

#### Phase B: Archaeology (Weekly/On-Demand)
1.  **Analyze**: Identify "Mystery Nodes" (functions with high complexity, low documentation, old dates).
2.  **Research**: Use `web_brave_web_search` to investigate:
    *   "What is library 'legacy-utils v0.1'?"
    *   "Why was pattern X used in 2016?"
3.  **Enrich**: Add observations and "Context Edges" to the graph.

#### Phase C: Advisory (Interactive)
1.  **Query**: Developer asks "How does the billing system work?"
2.  **Synthesize**: Agent traverses the graph (Code -> Context) to explain:
    *   "The billing system uses Stripe v2 (deprecated)."
    *   "It was implemented in 2017 to handle X."
    *   "Warning: It relies on a vulnerability in 'async-lib'."

### 3. Tool Utilization
*   **memory**: The primary storage for the synthesized world model.
*   **filesystem**: Read-only access for scanning; Write access for generating Markdown reports/ADRs.
*   **web**: To ground internal code artifacts in external reality (documentation, forums).
*   **grep**: High-speed symbol extraction.
*   **shell**: For running `git` commands to extract history (author/date metadata).

### 4. Safety & Persistence
*   **Read-Mostly**: The agent never modifies code directly. It only writes to its `memory` graph and generates `docs/` files.
*   **Graph-Based Persistence**: The state is preserved in the MCP Memory graph, allowing it to survive restarts and grow smarter over time.

### 5. Future Extensibility (The Sandbox Plugin)
*   **Refactoring**: Can be enabled to spin up a `sandbox/` (from Design 2) to test modernization hypotheses (e.g., "Can we upgrade to React 18?") and report results without touching main.

## Key Insight
**Code is not self-contained.**
Traditional tools analyze code in isolation. This agent treats code as an artifact of a specific time and technology ecosystem. By using the Web to fetch historical context, it bridges the gap between "reading the syntax" and "understanding the intent."

