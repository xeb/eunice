# Design 1: The Norm Linter

## Purpose
"The Norm Linter" acts as a pre-flight check for social compliance. Just as a code linter checks for syntax errors, this agent checks for "cultural syntax" errors in contributions. It answers the question: "Will this PR be rejected because I didn't follow the unwritten rules?"

## Loop Structure
1.  **Baseline Acquisition (Monthly):**
    *   The agent crawls the target repository's last 100 merged PRs and last 50 closed issues using `web_brave_web_search` (or GitHub API via `fetch`).
    *   It extracts metrics:
        *   Average PR description length.
        *   Presence of specific keywords (e.g., "Fixes #", "Signed-off-by").
        *   Emoji usage frequency.
        *   Tone formality score (simple heuristic).
    *   It stores these "Norms" in the `memory` graph (e.g., `Node(Repo:React) -> hasNorm(DescriptionLength > 50 words)`).

2.  **Pre-Flight Check (On Demand):**
    *   User invokes the agent on a local branch.
    *   Agent reads the local PR draft (commit messages, branch name, planned description).
    *   Agent compares local artifacts against the stored Norms.
    *   Agent outputs a report: "WARNING: Your commit message is 1 line; repo average is 3 lines with bullet points."

## Tool Usage
*   **Web:** accessing public repo data to establish baselines.
*   **Shell:** `git log` analysis of local history.
*   **Memory:** Storing the "Norm Profile" of the repo so it doesn't need to re-crawl every time.
*   **Grep:** Searching local files for contribution guides (`CONTRIBUTING.md`) to cross-reference with observed behavior.

## Memory Architecture
*   **Entities:** `Repository`, `Norm`, `Maintainer`.
*   **Relations:** `Repository -> requires -> Norm`, `Maintainer -> exhibits -> Tone`.
*   **Persistence:** The graph allows the agent to "remember" that "Repo A likes short commits" vs "Repo B likes long stories" without re-analyzing.

## Failure Modes
*   **False Positives:** Flagging a valid exception (e.g., a one-line fix for a typo) as violating the "long description" norm.
    *   *Recovery:* User can override with `--force`.
*   **Drift:** Norms change.
    *   *Recovery:* Scheduled re-baselining.

## Human Touchpoints
*   **Report Review:** The agent never changes code; it only produces a report (stdout or markdown file) for the user to read before pushing.
