# Design 2: The Epistemic Auditor

## Purpose
A research-focused agent that actively "red teams" your knowledge base. It identifies claims you have made, cross-references them with the web, and flags potential inaccuracies, outdated information, or missing context.

## Loop Structure
1. **Scan**: Periodically reads a random subset of your "opinion" or "essay" notes.
2. **Extract Claims**: Uses an LLM to extract key assertions (e.g., "Python is single-threaded").
3. **Verify**: Uses **Brave Search** to find recent sources confirming or refuting the claim.
4. **Challenge**: If a contradiction is found, it appends a "Warning: Epistemic Drift" block to the file with links to the new evidence.
5. **Enrich**: If the note mentions a concept but doesn't define it, the agent fetches a definition and inserts it as a footnote.

## Tool Usage
* **web_brave_web_search**: The source of external truth.
* **filesystem**: Reading content and appending warnings.
* **memory**: Tracks which claims have already been verified to avoid redundant checking.

## Memory Architecture
* **Graph Database**: Nodes are `Claims`, Edges are `Evidence`. The agent builds a "Truth Graph" that maps your personal beliefs against external consensus.

## Failure Modes
* **Hallucination/Bad Search**: The agent might flag a correct claim as wrong based on a bad search result. *Recovery*: The user can tag the warning with `#false-positive`, teaching the agent to ignore that specific conflict.
* **Annoyance**: The agent becomes a "Reply Guy" to your own notes. *Recovery*: Strict confidence thresholds before posting a warning.

## Human Touchpoints
* **Dispute Resolution**: The human reads the warnings and decides to either update their belief or dismiss the agent's finding.
