# Design 3: The Socratic Bridge (Hybrid)

## Purpose
To create a "negotiated break" where the agent acts as a Junior Partner who demands a handover before you leave, ensuring that the "Why" is captured alongside the "What".

## Core Loop
1.  **Surveillance:** Agent monitors shell and git state (like Design 2) to build a *draft* context.
2.  **Interception:** When user types `exit` or locks screen (via hooks), Agent triggers **The Bridge Protocol**.
3.  **Negotiation:**
    *   Agent: "I see you modified `auth.ts` but the tests are failing. Do you have a fix in mind?"
    *   User: "Yes, need to update the regex."
    *   Agent: "Noted. Anything else?"
4.  **Synthesis:** Agent generates a `BRIDGE.md` file in the root, containing the dialogue summary + links to relevant lines of code.
5.  **Resumption:**
    *   On login, the MOTD (Message of the Day) displays the `BRIDGE.md`.
    *   Agent asks: "Ready to apply the regex fix?"

## Tools
*   **memory:** Stores the "Long Term Context" (Project Goals).
*   **filesystem:** Stores the "Short Term Context" (BRIDGE.md).
*   **shell:** For interception hooks and git analysis.
*   **text-editor:** To open the BRIDGE.md for editing during negotiation.

## Memory Architecture
*   **Hybrid:**
    *   **Graph:** Stores high-level entities (`Feature: Auth`, `Bug: #123`).
    *   **File:** Stores the ephemeral session state (The "Bridge").
*   **Rationale:** Graph is for global knowledge; File is for immediate "Next Action" context.

## Failure Modes
*   **Annoyance:** User ignores the agent or force-quits.
    *   *Mitigation:* "Snooze" feature. Agent learns when to shut up.
*   **Drift:** If the user creates a Bridge but then works on something else.
    *   *Mitigation:* Agent detects divergence and asks "Did we change plans?"

## Human Touchpoints
*   **Conversational Handover:** 30-second dialogue at end of session.
*   **Reading:** 10-second read at start of session.
