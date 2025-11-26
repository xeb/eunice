# Design 2: The Doppelgänger Hunter (Innovative)

## Purpose
This agent actively searches the internet for your digital footprint, building a graph of connected identities to identify how a motivated attacker could link your pseudonyms to your real identity.

## Problem Domain
People often assume their "anonymous" Reddit or gaming accounts are safe, but they leave breadcrumbs (shared usernames, profile pictures, writing styles, cross-posting) that allow correlation.

## Core Tools
- **memory**: Stores the "Identity Graph" (Nodes: Accounts, Emails, Usernames, URLs. Edges: "Uses", "Mentions", "Same-as").
- **web**: Searches social media, forums, and archives.
- **fetch**: Scrapes profile pages to extract metadata (bio location, join date).
- **grep**: Analyzes local text for unique phrases that might also appear online.

## Main Loop
1.  **Seed**: User provides a seed email or username.
2.  **Expansion**:
    -   Agent searches `web` for the seed.
    -   For every result, it extracts new potential identifiers (e.g., finding a GitHub profile from a Twitter handle).
3.  **Correlation**:
    -   Creates nodes in `memory`.
    -   Looks for strong links (same avatar hash, exact bio match) and weak links (similar username, same city).
4.  **Inference**:
    -   Calculates "Graph Diameter" between Real Identity and Anonymous Nodes.
    -   If the path is too short (e.g., < 2 hops), it flags a "Privacy Breach".

## Memory Architecture
- **Knowledge Graph**:
    -   Entities: `Persona`, `Account`, `DataPoint` (email, phone).
    -   Relations: `OWNS`, `LINKED_TO`, `EXPOSES`.

## Failure Modes
- **Hallucination**: The agent connects two "John Smith"s who are different people. *Mitigation:* Requiring 2+ unique data points for a "Same-as" edge.
- **Scraping Blocks**: Sites blocking bots. *Mitigation:* Using search snippets mostly, low-volume fetching.

## Human Touchpoints
- **Confirmation**: "Is this account actually you?" before adding to graph.
- **Risk Assessment**: User decides if a link is acceptable or needs deletion.

## Key Insight
Using a Knowledge Graph not for accumulating knowledge, but for **mapping liability**. It turns the abstract concept of "exposure" into a concrete graph topology.
