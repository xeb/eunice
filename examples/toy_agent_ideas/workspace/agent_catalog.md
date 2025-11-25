# Agent Catalog

A journal of long-running agent designs ideated by the Toy Agent Ideation Engine.

---

## [2025-11-25 SEED] Agent: Knowledge Gardener
**Core Tools:** memory, web, fetch, filesystem
**Problem Domain:** Personal knowledge management and curation
**Key Insight:** Treating knowledge like a garden—facts need tending, topics can go stale, contradictions are weeds to address
**Persistence Strategy:** Hybrid (memory graph for knowledge, filesystem for human collaboration via inbox/outbox)
**Autonomy Level:** Semi-autonomous with mandatory human approval for fact confirmation
**Link:** workspace/knowledge_gardener/final.md

---

## [2025-11-25 SEED] Agent: Codebase Archaeologist
**Core Tools:** grep, filesystem, memory, shell
**Problem Domain:** Codebase understanding, documentation, and maintenance
**Key Insight:** Multi-modal operation (survey/conversational/patrol) allows the same knowledge base to serve background exploration, real-time Q&A, and proactive issue detection
**Persistence Strategy:** Hybrid (memory graph for knowledge, filesystem for reports and artifacts)
**Autonomy Level:** Variable by mode—Survey is autonomous, Conversational is interactive, Patrol generates reports for human review
**Link:** workspace/codebase_archaeologist/final.md

---


## [2025-11-25 14:32] Agent: The Software Immunologist
**Core Tools:** memory, web, shell, filesystem
**Problem Domain:** Automated dependency maintenance and self-healing security
**Key Insight:** Using a persistent "immune memory" to track package stability and apply learned fixes ("antibodies") across multiple projects, preventing the same bad update from breaking multiple repos.
**Persistence Strategy:** Hybrid (memory graph for reputation/fixes, filesystem for code changes)
**Autonomy Level:** Fully Autonomous (updates, tests, and fixes) with Human Review (PR merge)
**Link:** workspace/software_immunologist/final.md

## [2025-11-25 14:45] Agent: The Narrative Loom
**Core Tools:** memory, web, fetch, filesystem
**Problem Domain:** News synthesis, narrative tracking, and intelligence analysis
**Key Insight:** Modeling news not as a stream of updates but as a **Temporal Knowledge Graph** of evolving events, allowing the detection of narrative shifts and causal forks.
**Persistence Strategy:** Hybrid (Memory Graph for the "World Model", Filesystem for Reports)
**Autonomy Level:** High Autonomy (Continuous monitoring) with Human Seeding (Topic selection)
**Link:** workspace/narrative_loom/final.md

## [2025-11-25 14:39] Agent: The Interface Cartographer
**Core Tools:** shell, memory, web, grep
**Problem Domain:** Automated documentation verification and tool exploration
**Key Insight:** Empirically verifying "Digital Physics" by running controlled experiments on CLI tools to build a guaranteed-correct "User Manual" graph.
**Persistence Strategy:** Memory Graph (Ontology of Commands)
**Autonomy Level:** Semi-autonomous (Bounded Sandbox Exploration)
**Link:** workspace/interface_cartographer/final.md

## [2025-11-25 14:55] Agent: The Digital Customer
**Core Tools:** memory, shell, web, filesystem
**Problem Domain:** Automated quantification of Developer Experience (DX) and Usability
**Key Insight:** evaluating software not just on "does it work" (functional correctness) but on "how hard is it to use" (frustration metrics, error recovery, help lookups), effectively automating User Acceptance Testing.
**Persistence Strategy:** Hybrid (Memory Graph for the "Mental Model" of the tool, Filesystem for Reports)
**Autonomy Level:** High (Given a goal, it explores until success or frustration threshold)
**Link:** workspace/digital_customer/final.md

## [2025-11-25 14:42] Agent: Legacy System Archaeologist
**Core Tools:** memory, grep, filesystem, web
**Problem Domain:** Recovering "lost tribal knowledge" in legacy codebases and documenting the "why" behind historical technical decisions.
**Key Insight:** Combining static analysis (code graph) with "historical web context" (finding old docs/forums) to explain obscure code.
**Persistence Strategy:** hybrid (Memory Graph for relations + Filesystem for reports)
**Autonomy Level:** Fully autonomous (background daemon for scanning/researching)
**Link:** workspace/LegacyArcheologist/final.md

## [2025-11-25 14:50] Agent: The World-Smith
**Core Tools:** memory, filesystem, grep, web
**Problem Domain:** Creative Writing & Narrative Consistency
**Key Insight:** Twin-State Persistence: A Memory Graph for logic validation and a bi-directional Markdown Wiki for human interaction/editing.
**Persistence Strategy:** Hybrid (Graph Logic + Filesystem Interface)
**Autonomy Level:** Background Daemon (Autonomous Analysis & Wiki Gen, Passive Reporting)
**Link:** workspace/world_smith/final.md

## [2025-11-25 14:58] Agent: The Refactoring Steward
**Core Tools:** memory, filesystem, shell, grep, text-editor
**Problem Domain:** Continuous, Safe Technical Debt Reduction
**Key Insight:** Reifying Technical Debt as "Inbox Proposals" (Markdown files) that humans approve/reject, creating a high-trust negotiation loop for dangerous code changes.
**Persistence Strategy:** Hybrid (Memory Graph for Health Metrics, Filesystem for Approval Workflow)
**Autonomy Level:** High (Autonomous Surveillance & Execution) with Mandatory Human Checkpoint (Proposal Approval)
**Link:** workspace/RefactoringSteward/final.md

## [2025-11-25 15:05] Agent: The API Diplomat
**Core Tools:** fetch, web, filesystem, memory, shell
**Problem Domain:** External API drift, integration maintenance, and breaking changes
**Key Insight:** Treating API integrations as "Diplomatic Relations" that require proactive intelligence gathering (reading changelogs) and continuous treaty negotiation (updating contracts/SDKs) rather than just reactive testing.
**Persistence Strategy:** Hybrid (Memory Graph for Relations/Stability, Filesystem for Contracts/Code)
**Autonomy Level:** High (Autonomous Surveillance & Drafting) with Human Ratification (PR Merge)
**Link:** workspace/api_diplomat/final.md

## [2025-11-25 14:50] Agent: The Process Symbiont
**Core Tools:** shell, memory, web, grep
**Problem Domain:** Autonomous reliability and self-healing of local development environments.
**Key Insight:** Combining a "Process Supervisor" with a "Web-Enabled Debugger" to autonomously fix common dev environment issues (zombie ports, missing deps) by learning from past crashes.
**Persistence Strategy:** Memory Graph for learned fixes + Filesystem for logs
**Autonomy Level:** High (Autonomous "Safe" Fixes, Permission for "Risky" Fixes)
**Link:** workspace/process_symbiont/final.md

## [2025-11-25 14:52] Agent: The Domain Linguist
**Core Tools:** memory, grep, filesystem, text-editor
**Problem Domain:** Domain-Driven Design (DDD) enforcement and Semantic Drift prevention
**Key Insight:** Treating the Codebase as a projection of a "Living Ontology" graph; if the graph changes, the code is refactored; if code changes, the graph is updated.
**Persistence Strategy:** Hybrid (Memory Graph for Ontology, Filesystem for Glossary/Code)
**Autonomy Level:** Semi-Autonomous (Watchdog & Gardener) with Human Approval for Refactoring
**Link:** workspace/domain_linguist/final.md
