# Design 1: The Causal Mapper (Passive)

## Purpose
The Causal Mapper is a background daemon that reads software documentation, architecture decision records (ADRs), and post-incident reviews to build a global 'Causal Map' of the system. It visualizes hidden feedback loops that might lead to instability.

## Tool Usage
*   **filesystem**: Reads .md, .txt, and code comments.
*   **memory**: Stores variables (nodes) and causal links (edges) with polarity (+/-) and delay.
*   **grep**: Finds keywords like 'leads to', 'increases', 'causes', 'results in', 'bottleneck'.
*   **shell**: Renders the graph using Graphviz (dot).

## Loop Structure
1.  **Scan**: Watch for changes in docs/ or post-mortems/.
2.  **Extract**: Use regex and NLP patterns to identify causal claims.
    *   *Example:* 'Increasing cache size [Cause] reduces DB load [Effect: -].'
3.  **Graph**: Update the Memory Graph with new nodes/edges.
4.  **Detect**: Run cycle detection algorithms to find loops.
    *   *Reinforcing Loop (R):* Even number of negative links.
    *   *Balancing Loop (B):* Odd number of negative links.
5.  **Report**: If a new *Reinforcing Loop* is detected (potential runaway effect), generate a PNG diagram and save it to reports/potential_loops/.

## Memory Architecture
*   **Nodes**: Variable (Name, Unit, Context).
*   **Relations**: CAUSES (To, Polarity, Delay, Confidence).
*   **Observations**: Source file snippet, Last Verified Date.

## Failure Modes
*   **Misinterpretation**: 'X does not cause Y' might be parsed as a causal link.
    *   *Recovery:* User can explicitly tag text <!-- ignore-causal --> or edit the memory graph.
*   **Sprawl**: Graph becomes a 'hairball'.
    *   *Recovery:* Prune low-confidence links or cluster by subsystem.

## Human Touchpoints
*   Passive. The human only interacts by reading the reports.
