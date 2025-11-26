# Design 3: The "Positioning Aligner" (Hybrid)

## Purpose
A marketing-focused agent that ensures the project's *Public Face* (Landing Pages, README) matches its *Actual Capabilities* (Code) AND the *Market Standard* (Competitor Messaging). It detects when you built a feature but forgot to sell it, or when you are selling it using outdated terminology compared to competitors.

## Loop Structure
1.  **Competitor Scan**: Fetches competitor landing pages to extract "Value Propositions" and "Keywords" (e.g., "AI-Powered", "Real-time", "Collaborative").
2.  **Internal Doc Scan**: Fetches local , , and  content.
3.  **Code Capabilities Scan**: Greps the codebase to verify if claimed features actually exist (Basic Reality Check).
4.  **Alignment Analysis**:
    *   **Underselling**: Code has  but Landing Page doesn't mention "Data Export".
    *   **Overselling**: Landing Page says "AI-Powered" but code has no ML libraries.
    *   **Terminology Drift**: Competitors call it "Workflows", we call it "Pipelines".
5.  **Proposal Generation**: Creates a PR or Issue with specific copy changes to align with the market.

## Tool Usage
*   **filesystem**: Reading local docs and code.
*   **web/fetch**: analyzing competitor messaging.
*   **memory**: Storing the "Glossary of Market Terms".
*   **grep**: Verifying feature existence.

## Memory Architecture
*   **Graph**: Stores  nodes (e.g., "Single Sign On") with attributes for  (how many competitors use it) and  (do we use it?).

## Failure Modes
*   **Marketing Fluff**: Hard to verify "Easy to use" via grep.
*   **Context Missing**: The agent might suggest copying a competitor's term that is trademarked or legally distinct.

## Human Touchpoints
*   **Copy Approval**: The agent suggests *text changes*, but a human must approve the tone/voice.
