# Agent: The Accessibility Architect

## 1. Problem Domain
**Web Accessibility & Inclusive Design Compliance**
Modern web apps are often inaccessible not due to malice, but complexity. Automated scanners (like Lighthouse) catch syntax errors but miss "usability barriers" (e.g., a modal that traps keyboard focus, or a workflow that requires color vision). Developers view accessibility as a checklist item rather than a user experience constraint.

## 2. Key Insight
**Graph-Based Empathy Simulation**
Instead of just regex-matching for missing attributes, this agent models the application as a **State Transition Graph** and simulates **Users as Constraint Sets** (Personas) traversing this graph.
*   *Traditional:* "Does image X have alt text?"
*   *Agentic:* "Can a blind user successfully navigate from 'Home' to 'Checkout'?"
This allows the agent to detect deep structural issues (broken flows) and explain them in terms of user impact.

## 3. Core Tools
*   **memory:** Stores the **Barrier Graph** (UI Nodes + User Capabilities).
*   **filesystem:** Reads code to infer graph structure and writes fixes.
*   **web:** Fetches WCAG remediation patterns and Aria practices.
*   **grep/text-editor:** Locates code and applies surgical refactors.

## 4. Execution Loop
1.  **Cartography (Mapping):**
    *   Scans the codebase to build a static graph of Components and Interactions (links, buttons, form inputs).
    *   Infers "Requirements" for each edge (e.g., "This button requires a Mouse Click event").
2.  **Simulation (Testing):**
    *   Instantiates "Personas" from Memory (e.g., *Persona: Alex (Blind, Screen Reader)*).
    *   Attempts to find paths through the graph for each persona.
    *   Identifies **Barriers**: Edges where `Edge.Requirement` exceeds `Persona.Capability`.
3.  **Remediation (fixing):**
    *   For each Barrier, it searches `web` for a semantic fix (e.g., "How to make div button accessible").
    *   Uses `text-editor` to apply the fix (adding `role="button"`, `tabIndex="0"`, `onKeyDown`).
4.  **Education (Reporting):**
    *   Updates the Memory Graph with the fix.
    *   Generates a Markdown report explaining the "User Journey" that was unblocked.

## 5. Persistence Strategy
**Hybrid Graph + Filesystem**
*   **Memory Graph:** Stores the "Accessibility Ontology" (Rules, Personas) and the "Application Map" (which component does what).
*   **Filesystem:** Stores the Code (Single Source of Truth) and human-readable Reports (`a11y_audit.md`).

## 6. Autonomy Level
**High (Autonomous Remediation with Human Gatekeeping)**
*   The agent runs in the background, autonomously simulating users and preparing fixes.
*   It **cannot** commit to `main`. It opens Pull Requests (or creates patches) for human review.
*   It learns from PR rejections (updating its Memory to avoid that pattern).

## 7. Failure Modes & Recovery
*   **Visual Regression:** A fix for accessibility might break layout (e.g., focus outlines).
    *   *Recovery:* The agent tags fixes as "CSS-Impact: High" for closer human review.
*   **Dynamic Content:** It cannot see content loaded via API.
    *   *Mitigation:* It inserts "Runtime Observers" (optional code snippets) that log missing accessibility attributes in the browser console for the developer.

## 8. Example Scenario
The agent notices a `<div onClick={submit}>` in `Checkout.js`.
1.  **Simulation:** "Keyboard User" tries to traverse `Checkout -> Submit`.
2.  **Barrier:** The `div` is not focusable; the path is blocked.
3.  **Action:** The agent refactors it to `<button onClick={submit} className="clean-btn">`.
4.  **Report:** "Fixed a Critical Barrier: Keyboard users could not submit the form. Replaced non-semantic div with button."
