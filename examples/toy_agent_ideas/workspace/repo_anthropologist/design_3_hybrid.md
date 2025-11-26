# Design 3: The Socio-Technical Bridge

## Purpose
Instead of just checking *your* behavior, this agent analyzes the *network* of the repository to identify "Congruence Gaps"—areas where technical dependencies exist (Module A calls Module B) but social connections do not (Author A never talks to Author B). It acts as a "Matchmaker" for collaboration.

## Loop Structure
1.  **Graph Construction:**
    *   **Technical Graph:** Agent uses `grep` and `filesystem` to map file dependencies (Import Graph).
    *   **Social Graph:** Agent uses `web` (GitHub Issues/PRs) and `shell` (git log) to map "Who talks to Whom" and "Who touches What".
    
2.  **Gap Analysis:**
    *   Agent identifies edges in the Technical Graph that have no corresponding edge in the Social Graph.
    *   *Insight:* "User A is refactoring `api.ts`, which is heavily used by `frontend.ts` (maintained by User B), but they have zero communication history."

3.  **Bridge Building:**
    *   Agent posts a comment on the User A's PR: "Hey, I noticed this changes the API contract for `frontend.ts`. @UserB has worked on that file recently but isn't on the reviewer list. cc'ing them."
    *   Alternatively, it sends a private summary to the user: "Before you merge, you should chat with @UserB."

## Tool Usage
*   **Memory:** Stores the massive "Socio-Technical Graph" (Nodes: Users, Files; Edges: Imports, Edits, Comments).
*   **Shell:** `git shortlog`, `git blame`.
*   **Web:** Scraping issue co-occurrences.

## Memory Architecture
*   **Entities:** `Developer`, `Module`, `Conversation`.
*   **Relations:** `Developer -> owns -> Module`, `Module -> dependsOn -> Module`, `Developer -> communicatesWith -> Developer`.
*   **Query:** "Find all Modules where Dependency exists but Communication does not."

## Failure Modes
*   **Noise:** Suggesting connections that are unnecessary (e.g., standard library usage).
    *   *Recovery:* Whitelisting/ignoring common stable dependencies.
*   **Social Awkwardness:** Tagging people who hate being tagged.
    *   *Recovery:* Learning "Do Not Disturb" lists from bio or past interactions.

## Human Touchpoints
*   **Permission to Ping:** Agent asks "Should I cc @UserB?" before posting.
