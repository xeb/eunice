# Design 3: The Intent Injector (Hybrid/Proactive)

## Purpose
To permanently weld "Why" to "How" by **injecting requirement context directly into the source code** as structured metadata/comments, preventing future developers from deleting "weird code" that exists for a specific business reason.

## Loop Structure
1. **Watch Mode:** Monitor for new PRs or Commits.
2. **Identify Context:** Fetch the linked Ticket.
3. **Summarize Intent:** Generate a concise 1-line "Intent Statement" (e.g., "Fixes race condition in billing per TICKET-99").
4. **Code Injection:** 
    - Identify the function/block being modified using `grep` / AST analysis.
    - **Edit the File:** Insert a "Teleology Tag" above the function:
      ```python
      # @intent: [TICKET-99] Fixes race condition in billing (Urgency: High)
      def calculate_bill():
      ```
5. **Legacy Scan:** Periodically scan old code. If a function has no `@intent` tag, search `git blame` and old tickets to "archaeologically" find the original reason and inject it.

## Tool Usage
- **text-editor:** Precision insertion of comment blocks.
- **filesystem:** Read/Write source files.
- **web:** Fetch historical context.
- **memory:** Track which files have been "Enriched" with intent.

## Memory Architecture
- **Nodes:** `File`, `Function`, `IntentTag`.
- **Edges:** `HAS_INTENT`.
- **Persistence:** The *Codebase itself* becomes the primary storage of intent (via comments), Memory Graph is just an index.

## Failure Modes
- **Code Clutter:** Developers get annoyed by too many comments.
- **Drift:** The comment stays, but the code changes, making the comment a lie.
- **Recovery:** Agent has a "Pruning Mode" to remove tags for closed/old tickets if configured.

## Human Touchpoints
- **Review:** The Agent pushes a "Docs Fix" commit to the PR adding the tags.
- **Merge:** Human accepts the tags.
