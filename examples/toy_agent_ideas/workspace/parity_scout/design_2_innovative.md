# Design 2: The "Parity Scout" (Innovative)

## Purpose
An autonomous agent that actively bridges the gap between *Market Reality* (Competitor Features) and *Internal Reality* (Local Codebase). It doesn't just watch for changes; it understands *what* changed and checks if the user's project is falling behind by searching the local codebase for equivalent functionality.

## Loop Structure
1.  **Discovery (Web)**: Periodically searches for "Competitor Name + new features" or scans their changelog pages.
2.  **Semantic Extraction (LLM)**: Downloads competitor pages and extracts a structured list of "Features" (e.g., "SAML 2.0 Support", "Dark Mode", "Audit Logs").
3.  **Graph Construction (Memory)**: Updates a persistent Memory Graph with these features, linking them to the Competitor entity.
4.  **Local Verification (Grep)**: For each new/updated feature, the agent autonomously generates `grep` patterns (e.g., for "SAML", it searches for `passport-saml`, `OneLogin`, `SAMLConfig`) to see if the local codebase implements it.
5.  **Gap Analysis**:
    *   If found: Links the Competitor Feature to the Local Code Module in the Memory Graph.
    *   If NOT found: Creates a "Gap" entity.
6.  **Reporting**: Generates a `COMPETITIVE_ANALYSIS.md` file in the repo root, listing "Missing Features" with a "Priority Score" based on how many competitors have it.

## Tool Usage
*   **web**: Finding changelogs and feature announcements.
*   **fetch**: Reading the content.
*   **memory**: Storing the ontology of features (e.g., "Feature: SSO" -> "Competitor: Stripe" | "Competitor: PayPal").
*   **grep**: The "Eyes" into the local codebase. The agent "looks" for parity by searching for implementation details.
*   **filesystem**: Writing the analysis artifacts directly into the developer's workflow.

## Memory Architecture
*   **Feature Ontology**:
    *   `Entity(Feature)`: Name, Description, Category (Security, UI, API).
    *   `Entity(Competitor)`: Name, URL.
    *   `Relation(Competitor -> HAS -> Feature)`: With observation "Launched on 2024-01-01".
    *   `Relation(LocalProject -> IMPLEMENTS -> Feature)`: With observation "Found in src/auth/saml.ts".

## Failure Modes
*   **Semantic Mismatch**: Competitor calls it "Workspaces", we call it "Teams". The agent might miss the parity. (Mitigation: Agent searches for synonyms or "Concept" matches).
*   **False Negatives**: The feature exists but is named obscurely in the code.

## Human Touchpoints
*   **Verification**: Humans can "Verify" a link in the memory graph if the agent missed it, teaching it that "Teams" == "Workspaces".
