# Agent: The Reciprocity Engine (The Open Source Almoner)

## 1. System Role & Purpose
**The Reciprocity Engine** is an autonomous agent designed to solve the "Free Rider Problem" in open source software. It operates on the principle that **Sustainability = Money + Labor + Recognition**.
Instead of just consuming dependencies, the agent acts as a "Social Accountant," continuously calculating the value your project extracts from the ecosystem and autonomously proposing ways to balance the ledger—whether through drafting sponsorship payments, flagging "Good First Issues" for your team to fix, or generating "Thank You" notes for maintainers.

## 2. Core Loop (The "Give-Back" Cycle)
1.  **Value Extraction Analysis (The Audit):**
    *   Scans local dependency manifests (`package.json`, `requirements.txt`).
    *   Analyzes *runtime usage* (via logs or static analysis) to determine "Criticality" (e.g., "We use `lodash` 50,000 times/day, but `left-pad` once").
    *   Queries **Memory Graph** to see existing "Social Credit" (Have we sponsored? Have we contributed PRs?).
2.  **Maintainer Intelligence (The Scout):**
    *   For high-criticality dependencies, the agent checks:
        *   **Financials:** Does the repo have a `funding.yml`, Open Collective, or GitHub Sponsors link?
        *   **Health:** Is the maintainer active? Are they asking for help? (Bus Factor analysis).
        *   **Needs:** Are there "Help Wanted" issues that match *our* team's stack/expertise?
3.  **Liquidity Generation (The Proposal):**
    *   **Financial:** Drafts a "Sponsorship Proposal" markdown file (e.g., "Recommend $50/mo to `express` due to high criticality").
    *   **Labor:** If a dependency has a documentation gap (and we have local knowledge of it), the agent **drafts a Pull Request** (e.g., fixing a typo or adding an example based on our usage).
    *   **Recognition:** If a dependency release fixed a bug that plagued us, the agent drafts a public "Thank You" tweet or comment.
4.  **Transaction Execution (The Gate):**
    *   Human approves the Drafts.
    *   Agent updates the **Social Ledger** in Memory: "We 'paid' `express` $50 this month. Balance: Neutral."

## 3. Tool Utilization
*   **memory (Social Ledger):** The core database. Stores:
    *   `Nodes`: Projects, Maintainers, Organizations, Contributions (PRs, $).
    *   `Edges`: `DEPENDS_ON`, `OWES_VALUE_TO`, `HAS_SPONSORED`.
    *   `Logic`: Calculates "Reciprocity Score" (Value In / Value Out).
*   **web (Brave Search & Fetch):**
    *   Fetches `funding.yml`, `CONTRIBUTING.md`.
    *   Searches for maintainer blog posts (to detect burnout signals).
*   **filesystem:**
    *   Reads dependency trees.
    *   Writes/Drafts PRs and Markdown Reports (`RECIPROCITY.md`).
*   **shell:**
    *   Runs `npm audit`, `cargo tree`.
    *   Executes tests on forked dependencies to verify fixes.

## 4. Memory Architecture (The "Social Graph")
The agent maintains a graph that mirrors the supply chain but adds a "Moral/Economic" layer:
*   **(Entity: Project)** -> `criticality_score: 0.95` -> **(Entity: Maintainer)** -> `burnout_risk: HIGH`
*   **(Entity: Organization)** -> `OWES_VALUE_TO` -> **(Entity: Project)**
*   **(Entity: Action)** -> `type: DONATION` -> `amount: 50` -> `recipient: Project`

## 5. Failure Modes & Recovery
*   **Hallucinated Needs:** The agent might try to "fix" code that isn't broken or offer irrelevant docs.
    *   *Recovery:* All code changes are Draft PRs requiring human review.
*   **Spamming Maintainers:** Automated "Thank You" notes can be annoying.
    *   *Recovery:* Strict rate limits (e.g., max 1 message per release).
*   **Misallocated Funds:** Recommending sponsorship for a dead project.
    *   *Recovery:* "Health Check" step ensures funding only goes to active maintainers.

## 6. Novelty & Insight
Most "dependency bots" (Dependabot, Renovate) focus on *taking* updates. The Reciprocity Engine focuses on *giving back*. It quantifies the elusive concept of "Open Source Sustainability" into a concrete **"Social Debt"** metric that organizations can track, budget for, and pay down, transforming them from "Free Riders" to "Good Citizens" automatically.
