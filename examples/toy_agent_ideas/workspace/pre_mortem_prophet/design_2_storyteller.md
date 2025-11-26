# Design 2: The Chaos Storyteller (Innovative/Narrative)

## Purpose
Humans are bad at probability but good at stories. This agent doesn't just list risks; it *writes the history of the future*. It generates a detailed, plausible "Press Release" or "Internal Post-Mortem" dated 6 months in the future, describing a catastrophic failure of the current system. This narrative approach bypasses "alert fatigue" and engages the team's imagination.

## Loop Structure
1.  **Architectural Inference**: Read code to understand the "Critical Path" (e.g., Checkout -> Payment -> Database).
2.  **Scenario Selection**: Pick a "Theme" (e.g., "The Leap Year Bug", "The DDOS", "The Third-Party API Deprecation").
3.  **Narrative Generation**:
    *   Draft a "Timeline of Events" leading to the failure.
    *   Cite specific filenames and functions that "failed" (grounded in actual code).
    *   Invent realistic error logs based on the tech stack.
4.  **Provocation**: Present the story to the user: "This is how we go down in November. Do you want to stop it?"
5.  **Mitigation Plan**: If the user says "Yes", generate the ticket/code stub to prevent it.

## Tool Usage
*   **filesystem**: Deep read of critical path files to cite real function names in the story.
*   **memory**: Store "Potential Timelines".
*   **web**: Research "Post-mortems of similar companies" to make the story realistic (style transfer).

## Memory Architecture
*   **Entities**: Scenario, NarrativeElement, Vulnerability.
*   **Observations**: "User X ignored warning Y", "Module Z has no timeout config".
*   **Simulation**: The memory graph acts as the "World State" for the simulation.

## Failure Modes
*   **Hallucination**: Citing files that don't exist (mitigated by filesystem validation).
*   **Melodrama**: Generating scenarios so catastrophic they are unbelievable (e.g., "Datacenter meteor strike").

## Human Touchpoints
*   **Interactive Fiction**: The user can "interrogate" the future outcome. "Did the backup fail?" "Yes, because of config X."
