# Design 3: The Central Banker (The Governor)

## Purpose
"The Central Banker" controls the "Money Supply" (Complexity Budget) of the project. It enforces a strict cap on total complexity. If you add complexity in one file, you *must* reduce it in another, or the build fails (or a warning is issued). It manages the "Inflation" of the codebase.

## Loop Structure
1. **Baseline Assessment:**
   - Calculate Total System Complexity (The "GDP").
   - Set a "Debt Ceiling".
2. **Transaction Monitoring (Pre-Commit/CI):**
   - specific shell command runs on changed files.
   - If Net Complexity > 0 and Debt > Ceiling:
     - Reject change (or Warning).
     - Suggest "Deflationary Measures" (files to refactor).
3. **Quantitative Easing:**
   - If team velocity slows, the Banker might "lower interest rates" (relax linting rules temporarily) to boost output, but records this as "National Debt".
4. **Crisis Management:**
   - If Debt explodes, it declares "Austerity Measures" (Block new features, only bug fixes allowed).

## Tool Usage
- **grep/shell:** Real-time complexity calculation of diffs.
- **filesystem:** Lock files or status flags (`AUSTERITY_MODE`).
- **memory:** Track the global Debt Ceiling and historic trends.
- **web:** Search for "Economic Policy" metaphors to generate witty error messages.

## Memory Architecture
- **Global State:** `TotalDebt`, `DebtCeiling`, `InflationRate`.
- **Policy:** Rules for when to trigger Austerity.

## Failure Modes
- **Deadlock:** Cannot merge critical fix because it adds complexity.
- **Revolt:** Developers disable the agent.
- **Recovery:** "Bailout" command (admin override to raise ceiling).

## Human Touchpoints
- **Governance:** Team leads vote to raise the Debt Ceiling.
- **Policy:** Humans define what counts as "Complexity".
