# The Phoenix Scribe: Executable Institutional Memory

**The Phoenix Scribe** transforms the static, often-ignored artifacts of failure (Incident Post-Mortems) into active, autonomous agents of resilience. It ensures that "we will fix this so it never happens again" is not just a promise, but an executable guarantee.

## Core Philosophy
Organizations suffer from "Resilience Amnesia". Incident reports are written, filed, and forgotten. New developers repeat old mistakes because the knowledge of past failures is locked in Markdown files, not in the CI/CD pipeline. The Phoenix Scribe treats **Post-Mortems as Code**.

## Architecture: The "Ouroboros" Loop

### 1. The Ingestion Engine (Knowledge)
*   **Trigger**: New Post-Mortem (PM) merged to `docs/incidents/`.
*   **Action**: 
    *   Parses the PM using LLM to extract the "Resilience Tuple": `{Component, FailureMode, Trigger, MitigatingPattern}`.
    *   **Memory Update**: Adds nodes to the `memory` graph.
        *   `Entity("Service:Auth") --has_weakness--> Observation("RateLimiting")`
        *   `Observation("RateLimiting") --reproduced_by--> Observation("Chaos:TrafficSpike")`

### 2. The Chaos Composer (Verification)
*   **Trigger**: A "Fix" is claimed in the PM, or a regression test is needed.
*   **Action**: 
    *   Converts the text description of the failure (e.g., "DB fell over under load") into a **Chaos Definition** (e.g., a `k6` load script + `pumba` container pause).
    *   Executes this Chaos Test in a sandboxed ephemeral environment.
    *   **Verification**: 
        *   *Before Fix*: System fails (confirms reproduction).
        *   *After Fix*: System survives (confirms mitigation).
    *   **Artifact**: Saves the successful Chaos Definition as a permanent Regression Test in `tests/resilience/`.

### 3. The Contextual Guardian (Prevention)
*   **Trigger**: Developer edits code.
*   **Action**: 
    *   Scans the diff.
    *   Queries `memory` graph: "Has this component failed before?"
    *   **Intervention**: If the developer is adding a retry loop to a service that previously caused a cascade failure, the Agent comments:
        > "⚠️ **Resilience Warning**: You are modifying `PaymentGateway`. In Incident #88 (2024), unjittered retries here caused a Thundering Herd. Please ensure you use `BackoffStrategy`. [Link to executable reproduction]."

## Tool Implementation

| Function | Tool | Usage |
| :--- | :--- | :--- |
| **Knowledge Base** | `memory` | Graph of Services, Incidents, and Failure Patterns. |
| **Orchestration** | `shell` | Running `docker`, `kubectl`, `toxiproxy`, `k6`. |
| **Research** | `web_brave` | Finding standard chaos patterns for specific tech stacks (e.g., "how to delay Postgres packets"). |
| **Observation** | `filesystem` | Reading logs, parsing PMs, writing new test files. |
| **Pattern Match** | `grep` | Locating code implicated in past incidents. |

## Failure & Recovery
*   **Runaway Chaos**: Agent enforces strict timeouts (30s) and CPU limits on all generated tests. If a test hangs, it `kill -9`s the container and marks the test as "Dangerous/Manual Only".
*   **False Correlation**: If the Agent flags irrelevant incidents, developers can reply "Not Relevant", and the Agent updates the memory edge weight to degrade the association.

## The Evolution
As the agent runs, it builds a **Resilience Topology** of the system—a map not of how the system *works*, but of how it *breaks*. This map becomes the ultimate onboarding guide for SREs.
