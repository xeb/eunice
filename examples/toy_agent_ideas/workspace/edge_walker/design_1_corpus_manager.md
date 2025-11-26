# Design 1: The Corpus Manager (Conservative)

## Purpose
A background agent that manages and optimizes the "corpus" of inputs used for testing applications. It wraps existing fuzzing tools (like `libFuzzer`, `AFL`, or even simple property-based testing scripts) to ensure they are running effectively and that their findings are persisted and analyzed over time.

## Core Loop
1. **Inventory**: Scans the filesystem for existing test inputs (seeds) and fuzz targets.
2. **Execution**: Runs a fuzzing job via `shell` for a fixed time budget.
3. **Analysis**:
   - specific crash inputs are hashed and stored in `memory`.
   - stack traces are parsed; unique crash types are identified.
4. **Curation**: 
   - Minimizes the corpus (removes redundant inputs that don't increase coverage).
   - Commits new "interesting" inputs to a `test/corpus/` directory in the repo.
5. **Reporting**: Updates a markdown dashboard with "Time since last crash", "Current Coverage", and "Top 5 Risky Areas".

## Tool Usage
- **shell**: To run `cargo fuzz`, `go test -fuzz`, or python `atheris`.
- **filesystem** (simulated via shell): To read the corpus files and write new findings.
- **memory**: To track the history of crashes (deduplication) so the agent remembers a bug even if the code changes (regression tracking).
- **grep**: To parse logs and stack traces for keywords.

## Memory Architecture
- **Entities**: `FuzzTarget`, `Crash`, `InputHash`, `StackFrame`.
- **Relations**: `caused_by`, `fixed_in_commit`, `similar_to`.
- **Persistence**: Hybrid. High-volume data (inputs) on filesystem, Metadata (relationships/history) in graph.

## Failure Modes
- **Resource Hog**: Agent uses too much CPU/RAM. *Mitigation:* `shell` commands run with `nice` and `ulimit`.
- **Disk Fill**: Fuzzer generates millions of files. *Mitigation:* Strict corpus minimization step in the loop.
- **Flaky Crashes**: Agent reports bugs that can't be reproduced. *Mitigation:* Agent must successfully reproduce a crash 3 times before logging it.

## Human Touchpoints
- **Review**: Humans see new files in `test/corpus/` as PRs.
- **Configuration**: `.edge-walker.config` determines which targets to prioritize.
