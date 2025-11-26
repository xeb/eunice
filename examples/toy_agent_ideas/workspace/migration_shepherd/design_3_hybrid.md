# Design 3: The Test-Driven Strangler (Hybrid)

## Purpose
To facilitate large-scale refactoring by generating ephemeral "safety net" tests from the live application's behavior before touching the code.

## Loop Structure
1. **Record:** Agent uses `shell` (curl/scripts) to hit the application with a corpus of inputs (from logs or fuzzed).
2. **Freeze:** It records the *exact* outputs (JSON, console text) into a "Golden Master" test suite on the `filesystem`.
3. **Refactor:** Agent applies changes (or allows User to apply changes).
4. **Replay:** Agent runs the Golden Master suite against the modified code.
5. **Score:** It calculates a "Behavioral Parity Score".
6. **Iterate:** If score < 100%, it reverts the change or patches it.
7. **Cleanup:** Once the refactor is committed, the Golden Master tests are deleted (they are brittle and short-term).

## Tool Usage
- **shell:** Running the app, capturing stdout/stderr.
- **filesystem:** Storing the snapshots.
- **grep:** Finding input points to fuzz.
- **text-editor:** Generating the test scripts.

## Memory Architecture
- **State:** File-based snapshots (JSON/Text).
- **Persistence:** Low. This is a transactional agent.

## Failure Modes
- **Nondeterminism:** Timestamps, UUIDs, or random seeds cause false negatives.
- **Recovery:** Agent tries to "scrub" variable fields using Regex (e.g., replace dates with `<DATE>`) before comparing.

## Human Touchpoints
- User defines the "Input Corpus".
- User decides when to delete the Golden Master tests.
