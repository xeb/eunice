# Agent Design: The Repo Anthropologist

## 1. High-Level Concept
**The Repo Anthropologist** is an autonomous agent that treats open-source contribution not just as code merging, but as **cultural integration**. It analyzes a repository's history to build a "Thick Description" of its social norms (communication style, unwritten rules, hierarchy) and actively guides the user to conform to these norms, maximizing the chance of PR acceptance.

**Problem:** Contributions are often rejected or ignored not because the code is bad, but because the "social packaging" (tone, description length, etiquette) doesn't match the tribe's expectations.
**Key Insight:** Automated "Ethnography as a Service." Using Memory Graphs to store the "Culture" of a repo separately from its code.

## 2. Core Architecture
The agent operates in three phases: **Observation** (Background), **Analysis** (Pre-Flight), and **Localization** (Action).

### Phase A: Observation (The Field Work)
*   **Trigger:** When a user clones a new repo or runs `repo-anthro init`.
*   **Action:**
    1.  **Web Search:** Scrapes the last 50 merged PRs and their comments using `web_brave_web_search`.
    2.  **Shell Analysis:** Runs `git log --no-merges --format="%s"` to analyze commit message patterns.
    3.  **Entity Extraction:** Builds a knowledge graph in `memory` with:
        *   `Norm:StrictGitmoji` (Confidence: 90%)
        *   `Norm:ReferenceIssues` (Confidence: 85%)
        *   `Maintainer:Alice` -> `prefers` -> `ConciseSummaries`
        *   `Repo:Linux` -> `taboo` -> `TopPosting`
    
### Phase B: Analysis (The Linter)
*   **Trigger:** User runs `repo-anthro check`.
*   **Action:**
    1.  Reads local `git diff`, `git log`, and the PR draft.
    2.  Compares against the `Norm` entities in Memory.
    3.  **Outputs Report:**
        *   "[PASS] Code style (Prettier detected)"
        *   "[FAIL] Commit Message: Your message is 72 chars long; repo avg is 50."
        *   "[WARN] Tone: You used 'I think'; Maintainers here prefer declarative 'This fixes'."

### Phase C: Localization (The Ghostwriter)
*   **Trigger:** User runs `repo-anthro fix`.
*   **Action:**
    1.  Generates a new PR description or commit message that mimics the repository's "Voice".
    2.  Example:
        *   *User Draft:* "Fixed the button."
        *   *Agent Rewrite:* "ui: resolve click handler race condition on Submit. Fixes #42."
    3.  Uses `text-editor` to apply the change after user confirmation.

## 3. MCP Tool Strategy
*   **Memory Server:**
    *   **Crucial Role:** Stores the "Cultural Profile" of different repos so the agent doesn't re-learn from scratch.
    *   *Schema:* `Entity(Type="Norm", Name="EmojiUsage", Obs="Used in 5% of commits")`.
*   **Web Server (Brave):**
    *   Used to fetch "Soft Data" (comments, mailing lists) that `git` doesn't show.
*   **Shell Server:**
    *   Used for `git` operations and reading local file content efficiently.
*   **Text-Editor Server:**
    *   Used to "Patch" the user's text files (PR templates, commit messages).

## 4. Failure Modes & Resilience
*   **"Poser" Risk:** The agent adopts a tone that is *too* authoritative for a new contributor.
    *   *Mitigation:* The "Persona" vector includes a "Seniority Adjustment" (e.g., mimic the style of *other new contributors* who were successful, not just the Lead Maintainer).
*   **Hallucinated Norms:** Inferring a rule from a coincidence (e.g., "All commits must start with 'Z'").
    *   *Mitigation:* Norms require a threshold of >20 occurrences to be active.

## 5. Future Composability
*   Can feed into **"The API Diplomat"** to handle the social side of negotiation while the Diplomat handles the technical side.
*   Can output a "Contributor Guide" markdown file for humans if one doesn't exist.
