# Design 3: The Contextual Anthropologist (Hybrid)

## Purpose
A research-heavy agent that enriches the codebase with external context. Unlike the Cartographer (internal structure) or Sandboxer (modernization), the Anthropologist explains *why* the code looks the way it does by correlating it with historical web data.

## Core Toolset
* **web**: For searching historical documentation, deprecation notices, and community discussions (StackOverflow, GitHub Issues).
* **fetch**: For retrieving full text of documentation or blog posts.
* **memory**: For storing the "Knowledge Layer" on top of the code graph.
* **filesystem**: For reading code manifests (package.json, pom.xml).

## Loop Structure
1. **Artifact Dating**:
   - Scans dependency manifests to identify library versions.
   - Estimates the "Carbon Date" of the code (e.g., "This module was likely written in mid-2018").
2. **Excavation**:
   - Identifies obscure function calls or patterns.
   - Searches the web: "Why use X instead of Y in [Year]?" or "Library Z v1.2 vulnerability".
3. **Annotation**:
   - Adds "Context Nodes" to the memory graph.
   - Example: `Node(AuthService) -> CONTEXT -> "Uses deprecated OAuth flow (2019 security advisory)"`.
4. **Publication**:
   - Generates "Retroactive Architecture Decision Records (ADRs)".
   - Creates a "Risk Heatmap" based on external CVEs and deprecation status.

## Memory Architecture
* **Entities**: `ExternalLibrary`, `Vulnerability`, `HistoricalTrend`, `StackOverflowThread`.
* **Relations**: `AFFECTS` (Vulnerability -> CodeFile), `EXPLAINS` (WebResource -> CodePattern).
* **Graph Overlay**: The code structure graph (from Design 1) is enriched with these external entities.

## Failure Modes & Recovery
* **Hallucination**: AI might invent reasons for code decisions. *Mitigation*: Strictly link to source URLs for every claim.
* **Link Rot**: External documentation might be gone. *Mitigation*: Archive key snippets into the `memory` observation store.

## Human Touchpoints
* **Validation**: Humans review the "Retroactive ADRs" to confirm they align with internal memory.
* **Search Direction**: Humans can point the agent to specific documentation wikis or internal Confluence pages.

