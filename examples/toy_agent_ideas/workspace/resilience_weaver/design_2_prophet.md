# Design 2: The Chaos Prophet (Experimental)

## Purpose
To actively *prove* fragility in code by automatically generating and running tests that simulate specific, researched failure scenarios.

## Loop Structure
1.  **Target Selection:** Identify high-value targets (IO-bound functions, public API endpoints).
2.  **Failure Scouting:** Use Web Search to find *how* these targets fail (e.g., "how to reproduce connection reset peer python requests").
3.  **Harness Generation:** The agent uses `text-editor` to create a temporary test file (`probe_test.py`).
    *   This test uses mocking libraries (like `unittest.mock`) to force the identified exception when the target function is called.
4.  **Execution & Observation:** Run the test via `shell`.
    *   **Pass (Crash):** If the application crashes with a traceback, the vulnerability is confirmed.
    *   **Fail (Handled):** If the application degrades gracefully (returns 503, logs error), the code is robust.
5.  **Patch Proposal:** If a crash occurs, the agent generates a patch to wrap the code in a `try/except` block that handles that specific scenario.

## Tool Usage
*   **shell:** Execute tests, install mock libraries.
*   **text-editor:** Write test harnesses and apply patches.
*   **web_brave_web_search:** Find specific mocking strategies for libraries (e.g., "mock boto3 s3 down").
*   **grep:** Find call sites to target.

## Memory Architecture
*   No persistent graph needed; this is an ephemeral "Attack & Patch" loop.
*   Uses filesystem for test artifacts.

## Failure Modes
*   **Destructive Testing:** Mocks might leak or side-effects might occur if not perfectly isolated. (Mitigation: Run in a container/sandbox only).
*   **False Confidence:** Passing a mocked test doesn't guarantee real-world resilience.

## Human Touchpoints
*   **Permission to Patch:** User must approve the application of the fix.
*   **Test Review:** User reviews the generated "Proof of Failure" test case.
