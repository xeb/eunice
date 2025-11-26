# Design 3: The Code Ecologist (Hybrid/Deep)

## Purpose
A static and dynamic analysis agent that identifies inefficient code patterns (busy loops, uncompressed data transfer, excessive re-renders) that disproportionately consume energy, treating them as "Carbon Smells".

## Loop Structure
1. **Scan:** Greps codebase for known energy anti-patterns (e.g., `while(true)` without sleep).
2. **Profile:** Runs specific unit tests under a power profiler to identify "Hot Methods".
3. **Refactor:** Generates patches to replace inefficient patterns (e.g., replacing polling with event listeners).

## Tool Usage
- **grep:** Finds static anti-patterns.
- **shell:** Runs micro-benchmarks with power instrumentation.
- **text-editor:** Reads code, applies refactoring patches.
- **memory:** Stores the "Energy Efficiency" score of modules.

## Memory Architecture
- **Nodes:** `Function`, `EnergyCost`, `OptimizationPattern`.
- **Edges:** `Function --HAS_COST--> EnergyCost`.
- **Logic:** "If Function A is Hot and matches Pattern B, propose Refactor C."

## Failure Modes
- **False Positives:** Optimizing for energy might hurt latency or readability.
- **Heisenbugs:** Profiling overhead might obscure the actual energy cost.

## Human Touchpoints
- **Review:** All changes are submitted as PRs/Patches.
- **Configuration:** Users define "Critical Paths" where latency > energy.
