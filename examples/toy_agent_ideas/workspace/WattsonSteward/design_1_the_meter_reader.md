# Design 1: The Meter Reader (Conservative)

## Purpose
A passive observability agent that monitors local and CI/CD energy consumption, generating "Carbon Invoices" to make the invisible cost of compute visible to developers.

## Loop Structure
1. **Monitor:** Periodically queries system power metrics (RAPL on Linux/Intel, PowerGadget on Mac) and container stats.
2. **Contextualize:** Correlates power spikes with specific processes, git commits, or test runs.
3. **Report:** Generates a daily markdown report in `docs/sustainability/daily_carbon.md` and a weekly summary.

## Tool Usage
- **shell:** Executes power measurement tools (e.g., `scaphandre`, `perf`).
- **filesystem:** Reads logs, writes markdown reports.
- **memory:** Stores baseline energy consumption for specific tests/tasks to detect regressions.
- **web:** Fetches current grid carbon intensity (gCO2/kWh) to convert Joules to Carbon.

## Memory Architecture
- **Nodes:** `Task`, `Commit`, `EnergyProfile`.
- **Edges:** `Task --CONSUMED--> EnergyProfile`, `Commit --CAUSED--> EnergyProfile`.
- **Logic:** "If commit X increased energy usage of Test Suite Y by >10%, flag it."

## Failure Modes
- **Sensor unavailability:** Falls back to estimated models (TDP * Usage) if hardware sensors are blocked.
- **API Failure:** Uses cached grid intensity averages if the live API is down.

## Human Touchpoints
- **Passive:** Developers read the reports.
- **Alerts:** Slack/Email notification on significant regression.
