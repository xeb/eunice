# Agent: The Portfolio Curator

## Purpose
**The Portfolio Curator** is a background agent that transforms the "exhaust" of your daily work—git commits, code changes, and project dependencies—into a persistent, evidence-based Career Knowledge Graph. It uses this graph to autonomously maintain your Resume, generate "Brag Documents" for performance reviews, and identify skill gaps relative to real-time market trends.

It solves the problem of **"Resume Amnesia"**: Developers often forget the specific, high-value technical challenges they solved months ago, leading to generic resumes that fail to pass ATS filters or impress hiring managers.

## Architecture

### 1. The Mining Loop (Internal Awareness)
*Frequency: Daily*
The agent uses `grep` and `filesystem` tools to scan watched repositories.
- **Dependency Extraction:** Parses `package.json`, `requirements.txt`, `Cargo.toml` to identify raw tools.
- **Usage Analysis:** Uses `grep` to find *how* libraries are used (e.g., differentiating "import React" from "import { useEffect }").
- **Commit Correlation:** Links code changes to Commit Messages to extract "Intent" (Why was this code written?).
- **Output:** Updates the **Memory Graph**:
  - `(User) -[AUTHORED]-> (Commit hash) -[MODIFIED]-> (File) -[CONTAINS]-> (Skill: GraphQL)`

### 2. The Market Loop (External Awareness)
*Frequency: Weekly*
The agent uses `web` tools to calibrate the internal graph.
- **Trend Analysis:** Searches for "Top skills for [User's Role] 2025".
- **Synonym Mapping:** Learns that "OR Mapper" in the graph maps to "Hibernate" or "Prisma" in job descriptions.
- **Gap Detection:** Identifies skills present in the market but absent/weak in the local graph.

### 3. The Curation Loop (Action)
*Frequency: On-Demand or Monthly*
- **Resume Synthesis:** Generates a `resume.md` file. It selects "Evidence" nodes from the graph that match the "Market" nodes with the highest weight.
  - *Before:* "Worked on backend."
  - *After:* "Engineered high-throughput GraphQL resolvers using Apollo Server, reducing latency by 40% (See: Commit 8a4b2)."
- **Portfolio Generation:** Builds a static HTML site where every claim links to a highlighted code snippet (stripping secrets via regex).

## Core Tools
- **grep:** For deep code analysis and pattern matching.
- **memory:** To store the "Skill Graph" (Skills, Projects, Evidence, MarketValue).
- **filesystem:** To read code and write the final Portfolio/Resume artifacts.
- **web:** To research skill trends and synonyms.
- **shell:** To execute git commands (`git log`, `git blame`).

## Memory Architecture
The Memory Graph is the central source of truth.
- **Nodes:** `Skill`, `Project`, `Commit`, `File`, `MarketTrend`.
- **Edges:**
  - `(Project) -[USES]-> (Skill)`
  - `(Skill) -[EVIDENCED_BY]-> (Commit)`
  - `(Skill) -[HAS_SYNONYM]-> (Keyword)`
  - `(MarketTrend) -[VALUTES]-> (Skill)`

## Failure Modes & Recovery
1. **Secret Leakage:**
   - *Risk:* Publishing a portfolio with API keys in the snippets.
   - *Mitigation:* `grep` pre-flight check for high-entropy strings and common credential patterns (AWS_KEY, etc.) before any snippet extraction.
2. **Misattribution:**
   - *Risk:* Claiming credit for code copied from StackOverflow.
   - *Mitigation:* Check `git blame` and author emails; ignore large "init" commits or vendor folders.
3. **Keyword Stuffing:**
   - *Risk:* Resume becomes a word salad.
   - *Mitigation:* Limit generated bullets to 3 per role, ranked by "Impact Score" (complexity of code changed).

## Human Touchpoints
- **The "Brag Doc" Review:** Every Friday, the agent presents a drafted "Weekly Impact Report." The user confirms: "Yes, this bug fix was important" or "No, that was a typo fix."
- **Privacy Toggles:** User defines `private_repos` vs `public_repos` in a config file.

## Why This Matters
It turns "Career Management" from a painful, manual administrative task into a **data-driven side-effect** of the work you are already doing. It ensures you never lose credit for a solved problem again.
