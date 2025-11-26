# Agent: The Shadow Scholar

## Problem Domain
**Community-Documentation Gap:** Official documentation is static and often outdated. "Street knowledge" (StackOverflow, GitHub Issues, Discord) is dynamic and current but dispersed and noisy. This leads to developers wasting time rediscovering known workarounds or bugs that are "common knowledge" to the community but absent from the docs.

## Core Insight
Treat community discussions not as "support" but as **Distributed Patching** for documentation. The agent acts as a bridge, formalizing the informal knowledge of the crowd into the official record.

## Architecture

### 1. The Observation Deck (Web + Fetch)
*   **Sources:** StackOverflow (via Brave Search), GitHub Issues (via API/Fetch), and specific forums.
*   **Triggers:** New accepted answers, Issues with label `documentation` or `bug`, high upvote counts.

### 2. The Knowledge Graph (Memory)
Instead of just summarizing, the agent builds a graph:
*   **Nodes:** `DocumentationPage`, `CodeSymbol` (function/class), `CommunityDiscussion`, `Claim`.
*   **Edges:** `Discussion --refutes--> CodeSymbol`, `Discussion --clarifies--> DocumentationPage`.
*   **Persistence:** The `memory` MCP server holds this state, allowing the agent to remember *why* it believes the docs are wrong.

### 3. The Action Tiers (Variable Autonomy)
The agent operates in two modes simultaneously:

#### Tier 1: The Appendix (Safe/Immediate)
*   **Action:** Appends summaries of active discussions to a `docs/COMMUNITY_KNOWLEDGE.md` file.
*   **Content:** "Known Issues," "Common Workarounds," "Clarifications."
*   **Risk:** Low. No existing docs are touched.

#### Tier 2: The Red Pen (High Confidence/Delayed)
*   **Action:** If a Tier 1 note persists for >30 days and has high validation (upvotes/comments), the agent promotes it.
*   **Execution:** It locates the relevant source file using `grep`, generates a patch using `text-editor`, and creates a Pull Request to *replace* the misleading text with the correct information.
*   **Risk:** Medium. Requires human PR merge.

## Tool Chain
1.  **Discovery:** `web_brave_web_search` ("site:stackoverflow.com [library-name] error")
2.  **Verification:** `grep_search` (Check if error string exists in codebase/docs)
3.  **Mapping:** `memory_create_relations` (Link external url to internal file path)
4.  **Drafting:** `filesystem_write_file` (Update Community Notes)
5.  **Refactoring:** `text-editor_edit_text_file_contents` (Create PR for official docs)

## Example Workflow
1.  User searches "How to auth with API v2?" on StackOverflow. Top answer says "The docs say use Header X, but v2 actually requires Header Y."
2.  Shadow Scholar finds this discussion.
3.  It checks the official `auth.md` and confirms it mentions "Header X".
4.  It adds an entry to `COMMUNITY_KNOWLEDGE.md`: "**Auth Header Mismatch**: Community reports v2 requires Header Y. See [Link]."
5.  After 2 weeks, if the SO answer is still valid and upvoted, Shadow Scholar creates a PR modifying `auth.md` to change "Header X" to "Header Y".

## Failure Recovery
*   **Hallucination Check:** Before creating a PR, the agent tries to verify the claim by searching for code examples in the repo that use the "new" method. If no code in the repo uses "Header Y", it pauses and asks for human review (via a PR comment).
*   **Spam Filter:** Ignores discussions from users with low reputation or accounts < 1 week old.

## Human Interface
*   **Passive:** Read the `COMMUNITY_KNOWLEDGE.md`.
*   **Active:** Merge/Close PRs.
*   **Direct:** Add `@shadow-scholar ignore` to a GitHub issue to tell the agent "this is not a doc issue".
