# Design 3: The Triage Negotiator (Hybrid)

## Purpose
To bridge the gap between automated detection and human intent by treating technical debt as a "negotiation" process. The agent acts as a project manager for debt, creating detailed proposals (Tickets) and only acting when authorized.

## Core Toolset
- **filesystem:** To manage a "Debt Inbox" (Markdown files in a `tech_debt/` folder).
- **grep/shell:** To scan for `TODO`, `FIXME`, and complexity metrics.
- **memory:** To track the status of proposals and human preferences over time.
- **text-editor:** To implement approved tickets.

## Loop Structure
1. **Audit:**
   - Periodically scan the codebase for:
     - New `TODO` comments.
     - Functions exceeding complexity thresholds.
     - Deprecated dependencies.
2. **Ticket Generation:**
   - Instead of fixing it immediately, create a Markdown file: `tech_debt/proposal_001_auth_refactor.md`.
   - Content: "I found `AuthService.ts` has cognitive complexity of 45. Proposal: Split into `SessionManager` and `CredentialValidator`. Risk: High. Est. Time: 5 mins."
3. **Negotiation (Human-in-the-Loop):**
   - The user sees the new file.
   - User actions:
     - **Approve:** Move file to `tech_debt/approved/`.
     - **Reject:** Move file to `tech_debt/rejected/` (Agent learns via Memory).
     - **Defer:** Move file to `tech_debt/backlog/`.
     - **Edit:** User modifies the proposal instructions in the file.
4. **Implementation:**
   - Agent watches `tech_debt/approved/`.
   - When a file appears, it reads the instructions and executes the refactor using `text-editor`.
   - Runs tests -> Commits -> Deletes the ticket file.

## Memory Architecture
- **Ticket History:** Tracks which types of refactors get approved vs. rejected.
- **User Preference:** "User always rejects renaming variables, but accepts extracting methods."

## Failure Modes
- **Stale Proposals:** Code changes while a ticket sits in the inbox. Mitigation: Agent re-validates the code state before execution.
- **Misinterpretation:** Agent misunderstands the user's edits to the markdown proposal.

## Human Touchpoints
- **The "Debt Board":** The `tech_debt/` directory acts as a Kanban board.
- **Async Workflow:** No real-time chat needed. Users review proposals at their leisure.

## Pros & Cons
- **Pros:** High trust. Zero surprise changes. Documentation of debt is built-in.
- **Cons:** Slower. Requires human effort to review markdown files.
