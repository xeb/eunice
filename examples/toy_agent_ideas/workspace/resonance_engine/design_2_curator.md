# Design 2: The Curator (Autonomous)

## Purpose
An active background agent that fights digital entropy by **Contextual Resurfacing**. It aggressively finds connections between current work and old files, pushing "Exhibitions" to the user to keep the archive alive.

## Core Toolset
*   **filesystem:** Read access + Write (for creating "Exhibition" folders/symlinks).
*   **memory:** Stores User Context (recent activity) + File Graph.
*   **web (Brave):** Enriches file metadata (e.g., "This old PDF is cited by 2024 papers").
*   **shell:** To send system notifications (notify-send) or open files.

## Loop Structure
1.  **Monitor (Continuous):**
    *   Watch `~/.bash_history` or `Recent Documents` to see what user is working on (e.g., "Project X").
2.  **Resurface (Triggered):**
    *   Search Memory Graph for "Project X" + "Modified < 2020".
    *   If matches found: Calculate "Relevance Score" (Keyword overlap + External Web Relevance).
3.  **Exhibit (Daily/Weekly):**
    *   Create a folder `Desktop/Exhibition_[Date]`.
    *   Populate with Symlinks to the 5 most relevant "forgotten" files.
    *   Generate a `manifest.md` explaining *why* these were chosen (e.g., "Linked to your search for 'Crypto' yesterday").
    *   Notify user: "New Exhibition ready: 'Echoes of Crypto 2017'".

## Memory Architecture
*   **Nodes:** `UserInterest`, `File`, `ExternalEvent`.
*   **Edges:**
    *   `UserInterest -> sparked_by -> File`
    *   `File -> resonates_with -> ExternalEvent` (via Web Search)

## Failure Modes
*   **Annoyance:** User finds notifications distracting. (Fix: Feedback loop. If user deletes Exhibition without opening, reduce frequency).
*   **Hallucination:** Links unrelated files. (Fix: Strict keyword matching threshold).

## Human Touchpoints
*   **Feedback:** User can "Star" an exhibition item to reinforce the connection in Memory.
*   **Configuration:** "Mute" certain folders or topics.

## Pros/Cons
*   **Pros:** actively solves hoarding/entropy. Surprises the user.
*   **Cons:** High risk of being intrusive. Requires more system permissions.
