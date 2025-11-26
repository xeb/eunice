# Design 1: The Passive Auditor

## Purpose
"The Auditor" provides rigorous, quantitative financial reporting on codebase health. It treats technical debt not as a vague annoyance but as a balance sheet liability that accrues interest based on developer activity.

## Loop Structure
1. **Scan (Daily):**
   - Iterate through all source files.
   - Calculate `Complexity Score` (Principal) using indentation/length heuristics.
   - Calculate `Volatility Score` (Interest Rate) using git history (frequency of changes).
2. **Ledger Update:**
   - Store metrics in the Memory Graph: `(File) --[HAS_DEBT]-> (Amount)`.
   - Update `InterestRate` based on recent churn.
3. **Report Generation:**
   - Generate `DEBT_REPORT.md` in the root.
   - Highlight "Toxic Assets" (High Complexity + High Churn).
   - Project future costs if left unfixed (Compound Interest).

## Tool Usage
- **grep:** Count complexity indicators (nested loops, long functions, TODOs).
- **shell:** Run `git log` to calculate file volatility/churn.
- **memory:** Store the historical value of debt to track trends (Inflation/Deflation).
- **filesystem:** Write the markdown reports.

## Memory Architecture
- **Entities:** `File`, `Module`, `DebtMetric`.
- **Relations:**
  - `File -> HAS_METRIC -> DebtMetric`
  - `File -> BELONGS_TO -> Module`
- **Observations:** Daily snapshots of complexity and churn.

## Failure Modes
- **False Positives:** High complexity might be necessary (e.g., core algorithms).
- **Git History Noise:** Renames/Moves might reset volatility scores if not tracked carefully.
- **Recovery:** Re-scan entire codebase to rebuild baseline.

## Human Touchpoints
- **Read-Only:** Humans consume the reports.
- **Configuration:** Humans set "Exchange Rates" (e.g., 1 Cyclomatic Point = $50).
