# Design 3: The Dialectic Arena (Hybrid)

## Purpose
To facilitate a "Socratic Debate" about the code. The agent spawns multiple "Persona" sub-agents (in memory/context) that hold conflicting viewpoints (e.g., "The Performance Maximalist" vs. "The Readability Purist"). They analyze the code and argue, producing a synthesized "Consensus" report.

## Core Loop
1.  **Context Loading:** The agent reads a target feature/module.
2.  **Persona Selection:** It selects 2-3 relevant personas from `memory`.
    *   *Security Sam:* "How can I hack this?"
    *   *Maintainer Mary:* "Will I understand this in 6 months?"
    *   *Product Paul:* "Does this actually help the user?"
3.  **Round 1 - Critique:** Each persona generates specific critiques based on the code (using specialized `grep` patterns or web lookups).
    *   *Sam:* searches for `eval()`, `SQL injection`.
    *   *Mary:* counts cyclomatic complexity, comment density.
4.  **Round 2 - Rebuttal:** The agent attempts to "defend" the code against the critiques (e.g., "The eval is necessary because X, and it's sandboxed here").
5.  **Synthesis:** A final "Verdict" is written, highlighting the valid, undefended critiques.

## Tool Usage
*   **memory:** Stores Persona definitions (their biases, preferred tools, and past critiques).
*   **filesystem:** The shared "subject" of the debate.
*   **web:** Used by personas to find citations for their arguments (e.g., Sam searches CVE database).
*   **shell:** Used to run "metrics" (e.g., `cloc`, `complexity-report`) to support arguments.

## Memory Architecture
*   **Nodes:** `Persona`, `Argument`, `Rebuttal`, `Consensus`.
*   **Relations:** `Persona MAKES Argument`, `Argument ATTACKS File`, `Rebuttal DEFENDS File`.
*   **Persistence:** The "Debate History" is preserved. If you re-open the debate later, the personas remember their past points.

## Failure Modes
*   **Endless Debate:** The personas might get stuck in a loop of nitpicking.
    *   *Recovery:* Hard limit on "Turns" (e.g., 2 rounds).
*   **Vague Critiques:** "This code is ugly."
    *   *Recovery:* System prompt enforces "Citation Needed" - arguments must reference line numbers or external URLs.

## Human Touchpoints
*   **Moderator:** The human acts as the judge, deciding which arguments are valid.
*   **Persona Configuration:** User can create custom personas (e.g., "The CTO who hates Microservices").
