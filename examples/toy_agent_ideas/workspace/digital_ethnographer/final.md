# Agent: The Digital Ethnographer (aka "The Cultural Attaché")

## Purpose
A "Social Linter" for digital communities. It proactively analyzes the implicit norms, jargon, and "sacred values" of a target community (e.g., a specific Open Source project, a Discord server, or a subreddit) to help outsiders contribute effectively without committing social faux pas.

## Problem Domain
**Community Onboarding & Friction:** Developers often bounce off new communities because they don't know the "unwritten rules" (e.g., "Don't ping the maintainer directly," "Use this specific template," "Don't suggest rewrites in Java").

## Key Insight
**"Normative Graphing"**: Treating social norms not as vague feelings but as a **Directed Graph of Cause-and-Effect** derived from empirical data (past interactions).
*   If *Action A* consistently leads to *Result B* (Thread Lock/Downvotes), then *Action A* is a "Taboo."
*   This allows the agent to "Lint" a user's draft message for "Social Syntax Errors" before they hit send.

## Core Toolset
*   **Web (Brave):** To scrape public archives (GitHub Issues, Mailing Lists, Discourse).
*   **Memory:** To store the "Culture Graph" (Nodes: `Norm`, `Topic`, `Influencer`, `Jargon`).
*   **Filesystem:** To watch for draft messages and output "Briefing Dossiers."
*   **Shell:** To fetch raw data via curl/API.

## Architecture

### 1. The Observation Loop (Background Daemon)
*   **Targeting:** User feeds the agent a list of URLs (e.g., `https://github.com/rust-lang/rust`).
*   **Mining:** The agent periodically scrapes recent interactions.
*   **Pattern Recognition:**
    *   *Tone Analysis:* Is the community formal or casual?
    *   *Authority Mapping:* Who are the gatekeepers? (Users whose 'No' closes threads).
    *   *Taboo Detection:* patterns in closed/locked threads.
*   **Graph Updates:**
    *   `(Community) --[VAL UES]--> (Thoroughness)`
    *   `(Community) --[DISLIKES]--> (ETA Requests)`

### 2. The "Social Linter" (Interactive Mode)
*   **Draft Watcher:** The agent watches a `drafts/` folder.
*   **Simulation:** When a file is saved, the agent "runs" the text against its Norm Graph.
    *   *Check:* Does it use banned terms?
    *   *Check:* Does it match the preferred structure (e.g., "RFC style")?
*   **Annotation:** Appends comments to the bottom of the draft:
    *   "⚠️ **Risk High:** You tagged @maintainer directly. History shows this often leads to silence. Try the mailing list first."
    *   "ℹ️ **Tip:** This community uses 'bikeshedding' to mean 'trivial debate'. You used it incorrectly."

### 3. The Bridge (Query Mode)
*   **Analogy Generation:** "I know React. Explain this Rust community's governance to me in React terms."
*   **Output:** " The 'Core Team' here is like the 'React Core Team', but 'WG-CLI' is like a standard Redux maintainer group."

## Persistence Strategy
*   **Memory Graph:** Stores the volatile, evolving "Social Physics" of the community.
*   **Filesystem:** Stores static "Field Guides" (Markdown reports) and interacts with user drafts.

## Failure Modes & Recovery
*   **Misread Sarcasm:** The agent might think "Wow, great job breaking production" is a compliment.
    *   *Mitigation:* User can flag graph nodes as "Sarcasm/False Positive."
*   **Private Channels:** Cannot see private DMs where real decisions happen.
    *   *Mitigation:* Agent explicitly lists its "Blind Spots" in reports.

## Autonomy Level
**High (Observation) / Low (Action).**
The agent creates the map autonomously, but the user must *choose* to walk the path. The agent never posts on the user's behalf—it only advises.

## Example Scenario
1.  User wants to propose a feature to the Linux Kernel mailing list.
2.  User writes a polite, casual email in `drafts/kernel_patch.md`.
3.  Agent detects "HTML formatting" and "Top-posting".
4.  Agent warns: "⛔ **Critical:** This community strictly requires Plain Text and Bottom-Posting. Your email will likely be ignored."
5.  User fixes format. Agent approves.
