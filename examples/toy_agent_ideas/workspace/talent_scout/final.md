# Final Design: The Contributor Catalyst (The "Talent Scout")

## Purpose
To transform a project's software supply chain into a **verified talent pool** by identifying the external developers who are already maintaining the code you depend on, and finding "warm paths" to recruit them.

## Key Insight
**"Reverse Dependency Recruitment"** — The best candidate for your team is not someone with "React" on their LinkedIn, but someone who just merged a complex fix into the *exact* version of `react-query` that your production app relies on. This agent acts as a bridge between **Technical Dependency** and **Social Opportunity**.

## Core Components
1.  **The Supply Chain Scanner:**
    *   Continuously monitors `package.json`, `go.mod`, `requirements.txt`.
    *   Identifies "Critical Dependencies" (high usage, high complexity).
    *   Uses **Web Search** to resolve these to GitHub/GitLab repositories.

2.  **The Maintainer Profiler:**
    *   For each critical repo, identifies the "active core" (top 5 recent contributors).
    *   Builds a **Memory Graph** entity for each contributor (`ExternalDev`), linking them to the `Library` and specific `Topics` (e.g., "Cryptography", "WASM").

3.  **The Social Graph Overlay:**
    *   Scans the **local git history** and **issue tracker** to find instances where *internal* team members have interacted with these *external* devs (e.g., "Employee A commented on PR #123 by ExternalDev B").
    *   Enriches the Memory Graph with `HAS_INTERACTED_WITH` edges.

4.  **The "Dream Team" Report:**
    *   Generates a living `recruitment/scouting_report.md`.
    *   Rankings based on:
        *   **Relevance:** How much of their code runs in your prod?
        *   **Proximity:** Do we have a warm intro?
        *   **Activity:** Are they actively shipping?

## Autonomy Level
**High Autonomy (Data Gathering)** with **Human-Initiated Execution**.
*   The agent runs in the background, constantly updating the graph.
*   It does *not* send emails. It produces a "Scouting Report" that a human Recruiter or Engineering Manager uses to send a highly personalized message: *"Hey, we love your work on [Library], specifically your recent fix for [Issue]. We use that heavily. Want to chat?"*

## Persistence Strategy
**Hybrid:**
*   **Memory Graph:** Stores the complex many-to-many relationships (Dev <-> Lib <-> Skill <-> InternalEmployee).
*   **Filesystem:** Stores the readable artifacts (`scouting_report.md`, `candidate_profiles/`).

## Tool Usage
*   **filesystem_read_text_file:** Parsing dependency manifests.
*   **web_brave_web_search:** Finding repo URLs, contributor profiles, and public contact info.
*   **memory_create_entities / relations:** Building the social graph.
*   **shell_execute_command:** `git log` analysis to find internal/external intersection.

## Failure Modes & Recovery
*   **API Limits:** Aggressive scraping of contributor lists can trigger blocks. *Mitigation:* Random jitter and low-frequency polling (once per day).
*   **Privacy:** Inadvertently doxxing. *Mitigation:* Only store *public* commit emails and handles. No enrichment from sketchy data brokers.
*   **False Positives:** Listing a "contributor" who only fixed a typo. *Mitigation:* Filter by "Lines of Code Changed" or "Merge Count".

## Future Expansion
*   **"Sponsorship Scout":** Instead of hiring, suggesting which devs to *sponsor* on GitHub Sponsors to ensure the health of your dependencies.
*   **"Onboarding Generator":** Auto-generating a "Gap Analysis" reading list for a new hire based on the diff between their known libraries and your stack.
