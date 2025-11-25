# Agent: The Refactoring Steward

## Purpose
A background agent that turns technical debt management from a "once a year crisis" into a continuous "compound interest" investment. It identifies rotting code (High Complexity + High Churn), proposes specific refactoring plans as Markdown artifacts, and autonomously executes them upon approval.

## Core Toolset
- **memory:** Stores the "Code Health Graph" (Entity Complexity, Churn, User Preferences).
- **filesystem:** Manages the "Refactor Inbox" interface (Proposals/Approved/Rejected).
- **grep:** Rapidly scans for patterns and code smells.
- **text-editor:** Performs precise, AST-aware code modifications.
- **shell:** Runs complexity analysis, linters, and tests.

## Architecture

### 1. The Surveillance Loop (Daemon)
- **Trigger:** Runs on file changes (watch mode) or scheduled intervals.
- **Action:**
  - Uses `git log` to calculate **Churn** (how often a file changes).
  - Uses complexity scanners (e.g., `radon`, `plato`, `cloc`) to calculate **Cognitive Load**.
  - Updates the **Memory Graph**:
    - `Function(Auth.login) -> complexity(15) -> churn(High)`
    - `Metric(RefactorScore) = Complexity * Churn`

### 2. The Proposal Engine (Filesystem UX)
- **Threshold:** When `RefactorScore > X`, the agent generates a **Refactor Proposal**.
- **Artifact:** Creates a file: `refactor_inbox/proposal_2025_11_25_auth_login.md`.
- **Content:**
  ```markdown
  # Proposal: Extract Method from Auth.login
  **Reasoning:** Function has complexity 15 and changed 12 times this week.
  **Plan:**
  1. Extract validation logic to `validateCredentials()`.
  2. Extract session creation to `createSession()`.
  **Risk:** Low (100% test coverage detected).
  ```

### 3. The Negotiation (Human-in-the-Loop)
- The user interacts via the filesystem (no chat required):
  - **Approve:** Move file to `refactor_inbox/approved/`.
  - **Reject:** Delete file or move to `refactor_inbox/rejected/`.
  - **Modify:** Edit the Markdown plan to change variable names or scope.
- **Learning:** If user rejects, the Agent updates Memory: `User prefers NOT to touch Auth.login`.

### 4. The Execution (Worker)
- **Trigger:** Detects file in `refactor_inbox/approved/`.
- **Action:**
  - Reads the plan.
  - Creates a git branch `refactor/auth-login`.
  - Uses `text-editor` to apply changes.
  - **Crucial Step:** Runs `shell_execute_command(npm test)`.
  - If Green: Commits and pushes.
  - If Red: Reverts, adds error log to the Proposal file, and moves it back to `inbox` for human review.

## Key Insight: "Debt as a User Interface"
Most refactoring agents fail because they are too aggressive (breaking things) or too passive (ignoring deeper issues). By reifying "Debt" as tangible files in an Inbox, this agent allows the human to *manage* debt at a high level while the agent does the low-level heavy lifting.

## Memory Graph Schema
- **Nodes:** `File`, `Function`, `RefactorProposal`.
- **Edges:** `contains`, `has_complexity`, `proposed_on`, `rejected_by`.
- **Properties:** `churn_rate`, `last_scanned`, `user_preference_rule`.

## Failure Recovery
- **Test Failures:** The "Red" state is handled gracefully by reverting and asking for help.
- **Merge Conflicts:** If the code changed since the proposal was generated, the agent detects the hash mismatch (via `text-editor`) and regenerates the proposal.
