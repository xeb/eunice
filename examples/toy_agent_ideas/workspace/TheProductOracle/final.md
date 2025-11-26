# Agent Design: The Product Oracle

## 1. High-Level Concept
**The Product Oracle** is an autonomous "Chief Alignment Officer" that connects high-level business strategy with low-level engineering execution. Unlike standard PM tools that manage *lists* of tasks, the Oracle manages the *logic* of value. It uses a **Strategic Knowledge Graph** to validate new backlog items, estimate their implementation cost by analyzing the codebase, and proactively identify "Dead Strategic Ends"—features that exist in code but no longer serve a business goal.

**Key Insight:** **"Strategic Garbage Collection"** — Just as languages have GC for memory that has no references, this agent provides GC for code that has no *business* references (active OKRs).

## 2. Core Toolset
| Tool | Purpose |
|------|---------|
| **Memory** | Stores the "Strategy Graph" (Company Goals -> OKRs -> Initiatives). |
| **Filesystem** | Reads Backlog items (Markdown) and Codebase structure. |
| **Grep** | Analyzes code complexity to estimate "Cost" and finds orphan features. |
| **Web (Brave)** | Validates market assumptions and tracks competitor feature sets. |

## 3. Architecture & Logic Loop

### A. The Inbound Gate (Alignment Check)
1. **Trigger**: A new file is added to `backlog/incoming/`.
2. **Analysis**:
   - The agent reads the feature request.
   - It searches the **Memory Graph** for a relevant `KeyResult` or `StrategicPillar`.
   - *Example*: "Add Dark Mode" -> Matches Strategy "Enterprise Readiness".
3. **Action**:
   - **Aligned**: Moves file to `backlog/refined/` and injects a "Strategy Context" section into the header.
   - **Misaligned**: Comments on the file: "No active OKR supports this. Please link to Strategy or tag as 'Experiment'."

### B. The Cost Estimator (Feasibility Check)
1. **Trigger**: A feature enters `backlog/refined/`.
2. **Analysis**:
   - Agent extracts keywords (e.g., "SAML", "Auth").
   - Runs **Grep** and **Filesystem** checks to map the relevant code surface area.
   - Calculates a "Complexity Proxy" (Cyclomatic complexity of touched files).
3. **Action**:
   - Updates the Backlog item with: `Estimated Complexity: High (Touches critical Auth modules)`.
   - If (Value == Medium) AND (Cost == High) -> Recommends **"Kill"**.

### C. The Outbound GC (Strategic Pruning)
1. **Trigger**: Weekly scheduled job.
2. **Analysis**:
   - Agent identifies **Memory Nodes** marked as `Status: Deprecated` (e.g., "Pivot away from B2C").
   - It searches the codebase for features tagged with that strategy.
3. **Action**:
   - Creates a "Technical Debt" ticket: "Remove B2C Login Flow (Strategy 'B2C' is deprecated)."

## 4. Persistence Strategy (Hybrid)
- **Memory Graph**: The "Brain". Stores the Strategy Ontology (Goals, KPIs, Personas) and their relationships. This is the Source of Truth for *Why* we do things.
- **Filesystem**: The "Body". Stores the Artifacts (Backlog items, PRDs, Code). This is the Source of Truth for *What* we are doing.

## 5. Failure Modes & Recoverability
- **The "Innovator's Dilemma"**: The agent might reject novel ideas because they don't fit the *past* strategy.
  - *Fix*: Allow a "Wildcard" budget or `type: experiment` override.
- **False Positive Complexity**: Grep might overestimate cost for simple changes in complex files.
  - *Fix*: Human engineers must validate the "Complexity Score" during Sprint Planning.

## 6. Novelty
Most agents act as "Doers" (writing code) or "Organizers" (sorting lists). The Product Oracle acts as a **"Thinker"**, applying a semantic filter to work before it even reaches the engineers, potentially saving thousands of wasted hours.
