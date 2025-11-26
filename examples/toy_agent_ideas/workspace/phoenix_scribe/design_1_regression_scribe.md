# Design 1: The Regression Scribe (Conservative)

## Purpose
To automatically convert textual Incident Post-Mortems (PMs) into executable regression tests, preventing the "fix it and forget it" cycle where bugs re-emerge.

## Loop Structure
1. **Monitor**: Watch a designated `post-mortems/` directory for new Markdown files.
2. **Parse**: Use an LLM to extract "Root Cause", "Triggering Input", and "Expected Behavior" from the PM.
3. **Generate**: Create a standard unit/integration test file (e.g., `test_incident_2025_03_12.py`) that attempts to reproduce the failure.
4. **Verify**: Run the test against the *current* codebase.
   - If it passes (bug fixed): Commit the test as a regression guard.
   - If it fails (bug still present): Open a high-priority Issue linked to the PM.
5. **Report**: Update the PM file with a link to the generated test.

## Tool Usage
- **filesystem**: Read `post-mortems/`, write `tests/regressions/`.
- **grep**: Search codebase to find relevant modules mentioned in the PM.
- **shell**: Execute `pytest` or `npm test` to verify the generated test.

## Memory Architecture
- **Stateless/Low-State**: Relies primarily on the filesystem. The "memory" is the suite of regression tests it builds up over time.

## Failure Modes
- **Hallucinated APIs**: The agent might generate tests for functions that don't exist. *Recovery:* It runs the test; if it fails to compile, it attempts to "repair" the test using the error log up to 3 times before giving up.
- **Flaky Tests**: The reproduction might rely on timing. *Recovery:* Mark tests as `@flaky` if they yield inconsistent results.

## Human Touchpoints
- **Review**: A human must merge the PR containing the new test.
- **Ambiguity**: If the PM is too vague ("system felt slow"), the agent tags the author asking for concrete metrics.
