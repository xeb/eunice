# Design 1: The Continuity Auditor

## Purpose
A conservative, read-only analysis agent that ensures long-form fiction maintains internal consistency. It acts as a "compiler for fiction," checking for errors in continuity, timeline, and character details without ever modifying the creative text itself.

## Core Toolset
- **filesystem**: Read manuscript files (markdown/txt) and write reports.
- **memory**: Store the "Canonical Truth" of the world (Character traits, Timeline events).
- **grep**: Fast searching for entity mentions across large projects.

## Loop Structure
1.  **Scan Phase**: Iterate through manuscript files in order (e.g., `Chapter_01.md`, `Chapter_02.md`).
2.  **Extraction**: Use LLM to extract entities (Person, Location, Item) and facts from the text.
3.  **Validation**:
    -   Check new facts against the Memory Graph.
    -   *Conflict Detection*: "Chapter 1 says Alice has blue eyes. Chapter 5 says Alice has green eyes."
    -   *Timeline Check*: "Event B happens before Event A, but Event A is referenced in Chapter 2."
4.  **Reporting**: Generate a `Continuity_Report_[Timestamp].md` detailing potential errors, flagged with confidence levels.

## Memory Architecture
-   **Entity Nodes**: Characters, Locations, Objects.
-   **Observations**: Facts extracted from text, tagged with Source Chapter.
    -   Example: `Alice` -> `eye_color: blue` (Source: Ch1, Line 40).
-   **Relations**: `visited`, `met`, `killed`, `owns`.

## Failure Modes
-   **False Positives**: Metaphor detection failure (e.g., "Her eyes were cold as ice" interpreted as literal temperature).
-   **Context Loss**: Missing implied context between chapters.
-   **Recovery**: The user can whitelist/ignore specific warnings in a config file (`.auditor_ignore`).

## Human Touchpoints
-   **Review**: The agent only produces reports. The human must decide if the inconsistency is an error or a plot point.
-   **Configuration**: User defines the "Truth" if the agent is confused (e.g., "Alice is *supposed* to have green eyes, update the graph").
