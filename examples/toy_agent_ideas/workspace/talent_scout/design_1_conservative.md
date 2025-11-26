# Design 1: The Rolodex (Passive Profiler)

## Purpose
To build a highly relevant database of potential hires based on the actual software dependencies of the project, replacing generic keyword matching with "proven code contribution" metrics.

## Loop Structure
1.  **Dependency Scan:** Periodically reads `package.json`, `Cargo.toml`, `go.mod`.
2.  **Repo Resolution:** Uses Web Search to find the source code repositories for these packages.
3.  **Contributor Mining:** Fetches the top 10 contributors for each dependency.
4.  **Profile Building:** Creates entities in the Memory Graph for each contributor, tagging them with the specific libraries they maintain.
5.  **Report Generation:** Writes a `recruitment_pool.md` file listing top candidates, ranked by the criticality of the libraries they work on.

## Tool Usage
*   **filesystem:** Read dependency manifests.
*   **web:** Search for repo URLs and contributor lists (via GitHub/GitLab pages).
*   **memory:** Store candidates and link them to `Library` nodes.

## Memory Architecture
*   **Entities:** `Developer`, `Library`, `Skill`.
*   **Relations:** `MAINTAINS`, `CONTRIBUTED_TO`, `USED_BY_PROJECT`.
*   **Query:** "Find Developers who maintain libraries with >1000 downloads that we use in production."

## Failure Modes
*   **Rate Limits:** GitHub API or web scraping might be blocked. *Recovery:* Exponential backoff.
*   **Identity Resolution:** Same user, different usernames. *Recovery:* Naive matching, flagging for human review.

## Human Touchpoints
*   **Read-Only:** The human simply reads the generated report. No active outreach is performed by the agent.
