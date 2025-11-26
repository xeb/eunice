# Design 3: The Promise Keeper (Hybrid Variant - "The Negotiator")

## Purpose
To facilitate a "Renegotiation" of technical debt. Instead of silently tracking or aggressively deleting, the agent forces the user to make a conscious decision via a "Debt Inbox".

## Loop Structure
1. **Audit**: Finds "at-risk" TODOs (e.g., > 60 days old).
2. **Serve Notice**: Creates a Markdown file `debt_inbox/NOTICE_<hash>.md` for each stale item.
   - Content: "You promised to fix X 60 days ago. Options: [ ] Fix Now, [ ] Defer 30 Days, [ ] Declare Bankruptcy (Delete), [ ] Ticket."
3. **Listen**: Watches the `debt_inbox/` folder for changes.
   - If user checks `[x] Defer`: Agent updates the code comment to `// TODO(2025-12-01): ...`
   - If user checks `[x] Bankruptcy`: Agent deletes the comment.
   - If user checks `[x] Ticket`: Agent drafts a GitHub Issue.

## Tool Usage
- **filesystem**: The primary interface. The "Inbox" folder is the UI.
- **grep/shell**: For locating and blaming.
- **text-editor**: To modify the source code based on the user's checkbox selection.
- **memory**: Tracks which Notices are currently "out for signature" to avoid duplicates.

## Memory Architecture
- **Entities**: `Notice`, `Action`.
- **Relations**: `Notice PENDING_FOR Promise`.

## Failure Modes
- **Inbox Clutter**: If the user ignores the inbox, notices pile up.
- **Desync**: Code changes while notice is pending. (Mitigation: Hash check before applying action).

## Human Touchpoints
- **Async Negotiation**: The user interacts with the agent entirely by editing Markdown files in their own time.
