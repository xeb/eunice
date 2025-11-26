# Final Design: The System Dynamics Scout

## One-Line Summary
An autonomous agent that reads software documentation and incident reports to build and maintain a living **Causal Loop Diagram (CLD)** of the system, identifying hidden 'Vicious Cycles' and feedback loops that threaten stability.

## Problem Domain
*   **Emergent Failures:** Modern distributed systems fail in complex, non-linear ways (e.g., Thundering Herds, Retry Storms) that are not obvious from static code analysis.
*   **Siloed Mental Models:** The frontend team knows 'User Load -> Latency', the backend team knows 'Latency -> DB Locks', but nobody sees the full loop.

## Core Philosophy
**'Documentation is a Causal Claim'**. Every time an engineer writes 'We added a cache to reduce load,' they are defining a causal link (Cache Size --(+)--> Cache Hit Rate --(-)--> DB Load). This agent extracts, formalizes, and validates these claims.

## Architecture

### 1. The Causal Harvester (Tools: filesystem, grep, memory)
*   **Input:** Markdown files (docs/, ADRs/), Post-Mortems, Code Comments.
*   **Parsing Strategy:** Uses NLP (or regex heuristics) to find causal connectors:
    *   'leads to', 'affects', 'increases', 'decreases', 'results in', 'driven by'.
*   **Polarity Inference:** Determines if the relationship is Positive (Same direction) or Negative (Opposite direction).

### 2. The Loop Detector (Tools: memory, shell)
*   **Graph Traversal:** Periodically walks the Memory Graph to find closed cycles.
*   **Archetype Matching:** Compares detected loops against a library of **System Archetypes** (stored in Memory or fetched via Web).
*   **Output:** Generates a Graphviz (.dot) visualization of the loop.

### 3. The Reality Check (Tools: web, fetch)
*   **External Validation:** If a causal link is 'New Feature -> Viral Growth', the agent searches the web for similar case studies or benchmarks to validate the plausibility.

## The Agent Loop (Autonomy: Daemon)
1.  **Watch:** Trigger on file changes in docs/ or arch/.
2.  **Extract:** Update the Causal Graph in Memory.
3.  **Analyze:** Find cycles and calculate loop polarity.
4.  **Report:** If a **Reinforcing Loop** is found without a balancing factor, create an Issue.

## Memory Architecture (Graph)
*   **Nodes**: Variable, Archetype.
*   **Relations**: CAUSES (To, Polarity, Delay, Confidence).

## Why This is Novel
Most agents focus on *Code* (syntax/logic). This agent focuses on *System Dynamics* (time/feedback), bridging the gap between static architecture diagrams and runtime reality.
