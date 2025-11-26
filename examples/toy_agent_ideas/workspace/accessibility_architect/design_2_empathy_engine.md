# Design 2: The Empathy Engine

## Purpose
An innovative agent that shifts focus from **compliance** (checking rules) to **usability** (simulating users). It builds a "Barrier Graph" that models how different user capabilities interact with the UI, identifying paths that are blocked for specific personas (e.g., "Keyboard Only User", "Screen Reader User").

## Loop Structure
1.  **Ingest:** Reads the codebase to build a graph of UI transitions (Button A -> Opens Modal B).
2.  **Persona Simulation:** Instantiates "Virtual Users" with specific capability constraints (e.g., `vision: none`, `mouse: false`).
3.  **Traversal:** Attempts to "traverse" the UI graph using only the allowed capabilities.
    *   *Example:* Can the "Keyboard User" reach the "Submit" button? If the button is a `div` without `tabindex`, the path is blocked.
4.  **Barrier Detection:** Identifies "Severed Edges" in the graph where a persona cannot proceed.
5.  **Story Generation:** Generates a "User Story" narrative explaining the failure: *"User Alex (Blind) tried to checkout but got stuck at the 'Address' field because it lacked a label."*

## Tool Usage
*   **memory (Graph):** The core engine. Stores the `UI_Graph` (Components, Transitions) and the `Capability_Graph` (Personas, Constraints).
*   **filesystem:** Reads code to infer the UI structure (e.g., detecting `onClick` handlers or `React Router` paths).
*   **web:** Searches for remediation strategies for specific barriers found.

## Memory Architecture
*   **Entities:** `Persona`, `Capability` (e.g., Sight, PrecisionPointer), `Component`, `Barrier`.
*   **Relations:** `Persona LACKS Capability`, `Component REQUIRES Capability`, `Barrier BLOCKS Persona`.
*   **Key Insight:** Accessibility is a **Graph Traversal Problem**. If `Component.requirements \not\subseteq Persona.capabilities`, the edge is impassable.

## Failure Modes
*   **Inference Failure:** Static analysis cannot perfectly predict dynamic UI flows (e.g., complex state management).
    *   *Recovery:* Agent flags "Uncertain Paths" and asks humans to verify if a transition is possible.
*   **Complexity:** The graph can explode in size for large apps.
    *   *Mitigation:* Limits analysis to "Critical Paths" defined by the user (e.g., Checkout, Login).

## Human Touchpoints
*   **Path Definition:** Humans define the "Critical Paths" (Goals) the agent should test.
*   **Narrative Review:** Developers read the generated user stories to understand the *impact* of bugs.
