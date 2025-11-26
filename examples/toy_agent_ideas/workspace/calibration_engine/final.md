# The Calibration Engine (aka "The Reality Check")

## Purpose
To eliminate "Optimism Bias" in software estimation by acting as a silent, evidence-based feedback loop that learns the user's personal "Velocity Signature" and autonomously corrects future estimates.

## Core Philosophy
Developers are consistently inconsistent. Instead of asking them to "estimate better" (which fails), this agent accepts their flawed estimates as "Raw Signals" and applies a learned "Correction Factor" based on the task type (UI, Backend, Docs) and historical performance.

## The Architecture

### 1. The Observation Loop (Passive)
- **Frequency:** Runs on every `post-commit` hook or hourly.
- **Action:**
  - Scans `git log` to detect "Work Sessions" (clusters of commits < 4h apart).
  - Associates these sessions with active tasks in `TODO.md` (via Regex matching like `#123`).
  - Calculates **Actual Duration** (Time of Last Commit - Time of First Commit + Session Buffers).

### 2. The Learning Engine (Memory)
- **Data Structure:** A Knowledge Graph storing:
  - `Entity(TaskType)`: "Frontend", "Refactor", "Bugfix".
  - `Entity(Complexity)`: "Small (<50 LOC)", "Medium", "Large".
  - `Observation(Bias)`: A ratio of `Actual / Estimated`.
- **Model Update:**
  - If User estimated "2h" for a "Frontend/Small" task, but it took "4h", the agent records a bias factor of `2.0`.
  - Over time, it computes a Moving Average for this category (e.g., "Frontend tasks are usually underestimated by 1.8x").

### 3. The Feedback Loop (Active)
- **Intervention:** When the user writes a new task in `TODO.md`:
  - `[ ] Refactor Auth #backend (Est: 4h)`
- **Agent Reaction:** The agent *edits the file* to append its prediction:
  - `[ ] Refactor Auth #backend (Est: 4h) <!-- Reality Check: ~7.2h (Bias: 1.8x) -->`
- **Reporting:** Weekly "Velocity Report" showing trends: "Your estimation accuracy improved by 10% this week."

## Tool Usage
- **shell:** `git log`, `git diff --stat` (for LOC metrics).
- **filesystem:** Reading/Editing `TODO.md`, writing `velocity_report.md`.
- **memory:** Storing the **Calibration Matrix** (TaskType x Complexity -> BiasFactor).
- **grep:** Categorizing tasks based on keywords (e.g., "css" -> Frontend).

## Failure Modes & Recovery
- **The "Rabbit Hole" Problem:** A task that takes 10x longer due to unforeseen blockers ruins the average.
  - *Recovery:* Outlier detection (Z-score) excludes extreme anomalies from the training set.
- **Task Switching:** Working on 2 tasks simultaneously messes up the time tracking.
  - *Mitigation:* Agent checks for "interleaved commits" and splits the time proportionally based on LOC churn.

## Future Composability
- **Integration with "The Product Oracle":** Can feed realistic estimates into the roadmap to automatically push low-priority features when time runs out.
- **Integration with "The Refactoring Steward":** Can flag codebases that consistently cause estimation errors (High Volatility) as prime candidates for refactoring.
