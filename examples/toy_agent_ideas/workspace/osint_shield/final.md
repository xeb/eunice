# Final Design: The OSINT Shield

## Executive Summary
The OSINT Shield is a **personal counter-intelligence agent** that maps your digital footprint from an adversarial perspective. It combines local environment scanning (finding secrets) with external identity correlation (doxing yourself) to build a **Liability Graph**. This graph reveals not just what data is public, but how disconnected facts can be chained together to compromise your identity or security.

## Problem Domain
Individuals face asymmetric threats: attackers use automated tools to harvest data, while defenders (users) rely on memory and manual hygiene. Most users don't know that their "anonymous" Reddit account is mathematically linkable to their LinkedIn profile via a unique username pattern or shared bio text.

## Core Toolset
1.  **memory (The Graph)**: The central nervous system. Stores entities (`Persona`, `Account`, `Credential`, `File`) and their relations (`EXPOSES`, `LINKS_TO`, `MENTIONS`).
2.  **web (The Sensor)**: Brave Search API for finding footprints, username availability checks, and breach database queries.
3.  **grep + filesystem (The Scanner)**: Locates local leakage (API keys in dotfiles, PII in logs) that could be the "seed" for an external attack.
4.  **fetch (The Scraper)**: Retrieves specific profile pages to extract metadata for correlation (e.g., matching "Location: SF" across two accounts).

## Architecture & Loop

### Phase 1: The Internal Audit (Local)
*Frequency: Daily / Pre-commit*
1.  **Scan**: `grep` scans the `filesystem` for high-entropy strings, emails, and phone numbers.
2.  **Ingest**: Found items are added to `memory` as `Secret` nodes.
3.  **Check**: If a secret is found, the agent uses `web` to see if it's already leaked in public code repositories.

### Phase 2: The External Recon (Global)
*Frequency: Weekly / On-Demand*
1.  **Seed**: Starts with known nodes (your email, your name).
2.  **Spider**: Uses `web` search to find accounts associated with these seeds.
3.  **Correlate**:
    -   If Account A (GitHub) lists "Twitter: @cooldev", the agent creates a `Twitter` node and an `IS_SAME_PERSON` edge.
    -   **Fuzzy Matching**: If Account B (Reddit) has the same unique avatar hash or bio string as Account A, a probabalistic edge is created.
4.  **Pathfinding**: The agent calculates the "Distance" between your `Real Identity` node and your `Pseudonym` nodes.
    -   *Alert*: "Warning: Your Reddit account 'Throwaway123' is 1 hop away from your Real Name via a shared Steam ID."

### Phase 3: The Wargame (Simulation)
*Frequency: On Request*
1.  **Synthesize**: The agent looks at the graph and generates a "Dox Report".
2.  **Attack**: It constructs a hypothetical spear-phishing email or a generated password wordlist based on the graph's data.
    -   *Output*: "Based on your graph, an attacker knows your dog's name (Instagram) and your bank (Twitter complaint). Here is the phishing email they would send."

## Persistence Strategy
-   **Memory Graph**: Primary storage for the Identity Graph. This allows the agent to remember that "UserX" on ForumY was already checked and cleared, preventing loops.
-   **Filesystem**: Stores the generated reports (`dox_report_2025.md`) and a `.privacyignore` allowlist.

## Safety & Failure Modes
-   **Rate Limiting**: The agent uses exponential backoff for web queries to avoid being banned by search engines.
-   **Privacy**: All graph data is stored **locally**. The agent never uploads your profile to a cloud service.
-   **Hallucination Control**: "Same-as" links require at least 2 strong signals (e.g., Username + Location, or Bio + Avatar) to be created automatically; otherwise, they are flagged as "Potential" for human review.

## Human Interface
-   **The Graph Explorer**: The user can query the memory to ask "What connects me to X?".
-   **The Review Queue**: A list of "Potential Links" the agent found but isn't sure about. User confirms: "Yes, that is my old Flickr account."

## Unique Value Proposition
Unlike standard "privacy checkers" that just look for email breaches, The OSINT Shield **thinks in graphs**. It understands that privacy is a topology, not a checklist. It shows you the *paths* of exposure, empowering you to break the links that matter.
