# Agent: The Wattson Steward

## Purpose
**The Wattson Steward** is an autonomous Green Software Engineering agent that operationalizes the "Carbon Aware" philosophy. It acts as both a **Green Scheduler** for heavy tasks and a **Carbon Accountant** for the codebase, helping developers reduce the environmental impact of their work without sacrificing productivity.

It bridges the gap between abstract "Sustainability" goals and concrete CLI actions by making carbon cost visible and actionable.

## Core Components

### 1. The Carbon Compass (Web/Context)
- **Function:** Continuously monitors the Carbon Intensity of the local power grid (via Carbon Aware SDK/ElectricityMaps).
- **Output:** A "Traffic Light" signal (Green/Yellow/Red) in the terminal prompt and a 24h forecast.

### 2. The Green Queue (Shell/Execution)
- **Function:** A wrapper around heavy commands (builds, training, ETL).
- **Logic:** `wattson run "npm run build"` -> Checks grid. If Red, queues task for the next Green window (within user-defined deadline). If Green, runs immediately.
- **Autonomy:** High. It decides *when* to run, but respects the *what*.

### 3. The Energy Ledger (Memory/Filesystem)
- **Function:** Tracks the energy cost of CI pipelines and local commands over time.
- **Persistence:**
  - **Memory Graph:** Stores "Energy Signatures" of tests/commits to detect regressions (e.g., "Commit 3a4b spiked CPU usage by 40%").
  - **Filesystem:** Appends to `CARBON_LOG.md` for auditability.

## Execution Loop
1. **Sensory Phase:** Check Grid Carbon Intensity + System Load (RAPL/PowerGadget).
2. **Decision Phase:**
   - Are there queued tasks? Is it Green now? -> **Execute**.
   - Is a user running a heavy command interactively? -> **Suggest** delaying if Dirty.
   - Is CI running? -> **Profile** execution.
3. **Action Phase:**
   - Run tasks.
   - Log Joules/gCO2.
   - Update Memory Graph with new energy baselines.

## Tool Usage Strategy
- **shell:** Primary interface. Uses `powerstat`, `scaphandre`, or Intel RAPL to measure Joules. Wraps commands.
- **web:** Fetches realtime grid data.
- **memory:** The "Brain" that remembers that `Unit Test A` usually takes 50J. If it takes 500J today, it flags an anomaly.
- **filesystem:** Writes user-facing reports and manages the queue state.

## Failure Recovery
- **Offline Grid Data:** Defaults to historical averages for the region.
- **Sensor Failure:** Falls back to "Time-Based" estimation (CPU Time * TDP).
- **Stuck Queue:** "Watchdog" timer forces execution if a task is queued > Deadline (default 4h).

## Key Insight
**"Temporal Arbitrage for Compute"**
The most effective way to reduce software carbon footprint isn't always *optimizing code* (which is hard), but *shifting time* (which is easy). The Wattson Steward automates this arbitrage, treating Carbon as a resource constraint just like RAM or CPU.
