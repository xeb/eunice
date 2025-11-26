# Agent: The Resilience Weaver

## Executive Summary
The Resilience Weaver is an autonomous maintenance agent that fortifies software against "Unknown Unknowns" by aggregating crowd-sourced failure data. It proactively audits your codebase not against a static rule set, but against the collective experience of the internet. It identifies external dependencies, researches their common production failure modes, and actively verifies if your code handles them—either by static analysis or by generating specific "Proof of Failure" unit tests.

## Core Loop
1.  **Cartography (Discovery):**
    *   Scans the codebase using `grep` and `filesystem` to map all "IO Boundaries" (HTTP calls, DB queries, File IO, Library calls).
    *   Builds a dependency map in the **Memory Graph**.

2.  **Forensics (Research):**
    *   For each boundary (e.g., `boto3.client('s3').get_object`), it performs targeted **Web Searches**:
        *   "boto3 get_object specific exceptions"
        *   "boto3 s3 connection pool exhausted"
        *   "requests.get hangs indefinitely python"
    *   It parses results to extract **Failure Scenarios** (e.g., "PartialReadError", "ConnectTimeout").

3.  **Cross-Examination (Audit):**
    *   It examines the local code context around the boundary.
    *   **Static Check:** Does a `try/except` block cover the discovered Exception Types?
    *   **Dynamic Check (Optional):** Generates a temporary `pytest` case using `unittest.mock` to simulate the failure and asserts that the application handles it (i.e., doesn't crash).

4.  **Weaving (Remediation):**
    *   **Report:** Creates a `resilience_audit.md` file detailing "Exposed Nerves" (unhandled failures).
    *   **Patch:** Proposes specific code patches (using `text-editor`) to wrap vulnerable calls in robust error handling, adding comments citing the source of the failure mode (e.g., "See StackOverflow #12345").

## Core Tools
*   **web_brave_web_search:** The primary sensor for "Failure Knowledge".
*   **memory:** Stores the "Failure Ontology" (Library -> FailureMode -> Exception).
*   **grep_advanced-search:** Precising locating of call sites and exception handlers.
*   **text-editor:** Injecting tests and patches.

## Persistence Strategy
*   **Memory Graph:** Holds the "Universal Failure Database" (e.g., knowing that `requests` can raise `ConnectTimeout` is a permanent fact).
*   **Filesystem:** Stores the project-specific Audit Reports and generated Test Suites.

## Autonomy Level
*   **Research & Audit:** Fully Autonomous (Background Daemon).
*   **Patching:** Checkpoint-based (User approves code changes).

## Key Insight: "Failure-First Development"
Most agents help you write code that *works*. The Resilience Weaver helps you write code that *breaks gracefully*. It leverages the fact that "how things fail" is often better documented in forums and issue trackers than in official documentation.

## Example Scenario
*   **Agent:** Notices usage of `stripe.Charge.create`.
*   **Research:** Finds that Stripe API can throw `RateLimitError` which needs exponential backoff, not just a retry.
*   **Audit:** Checks code, sees a generic `except Exception:` or no retry logic.
*   **Action:** Proposes a patch importing `tenacity` library and wrapping the call with `@retry(wait=wait_exponential(...))`.
