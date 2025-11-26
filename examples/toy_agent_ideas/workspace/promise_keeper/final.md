# Agent: The Promise Keeper

## Purpose
To solve the "Comment Insolvency" problem where `TODO` and `FIXME` comments accumulate as unpaid technical debt. The Promise Keeper acts as an automated "Debt Collector," auditing codebases for stale promises and forcing developers to renegotiate, pay (fix), or declare bankruptcy (delete) via a file-based interaction loop.

## Problem Domain
Software Engineering / Technical Debt Management.
Specifically addressing the "Write-Only" nature of code comments and the lack of lifecycle management for in-code tasks.

## Key Insight
**"Bureaucracy as an Interface"** — Instead of relying on passive reports (which are ignored) or aggressive deletion (which is dangerous), the agent reifies technical debt into **Interactive "Eviction Notices"** (Markdown files) in a `debt_inbox/` folder. This forces the user to make a conscious, low-friction decision (check a box) to resolve the debt, turning the filesystem into a negotiation table.

## Core Toolset
- **grep**: To locate debt artifacts (`TODO`, `FIXME`, `HACK`).
- **shell**: To run `git blame` and determine the "Interest Rate" (Age) of the debt.
- **filesystem**: To create and monitor the `debt_inbox/` for user responses.
- **text-editor**: To apply the negotiated outcome (delete line, update date, insert ticket URL).
- **memory**: To track the "Credit History" of developers (Deferral vs. Resolution rates).

## Architecture & Loop

### 1. The Audit (Discovery)
- **Trigger**: Cron or File Change.
- **Action**: Scans codebase for comments matching `// TODO`, `# FIXME`, etc.
- **Enrichment**: Runs `git blame` to find:
  - **Debtor**: Who wrote it?
  - **Principal**: What is the task?
  - **Term**: How long ago? (e.g., 400 days).
- **Graph Update**: Updates Memory Graph.
  - `User(name) --[OWES]--> Promise(hash)`
  - `Promise --[HAS_AGE]--> Days(400)`

### 2. The Summons (Notification)
- **Logic**: If `age > policy_limit` (e.g., 90 days) AND `status != renegotiated`:
- **Action**: Creates `debt_inbox/NOTICE_{hash}.md`.
- **Content**:
  ```markdown
  # DEBT NOTICE: P-8A2F
  **Debtor:** @jdoe
  **Age:** 124 days
  **Context:** `src/utils.ts:45`
  > // TODO: Refactor this O(n^2) mess

  ## Options (Check one)
  - [ ] **Pay Now**: I have fixed it. (Agent verifies & removes comment)
  - [ ] **Refinance**: Defer for 30 days. (Agent updates comment to `TODO(2025-12-25)`)
  - [ ] **Securitize**: Convert to GitHub Issue. (Agent posts to API, replaces comment with Issue URL)
  - [ ] **Bankruptcy**: Delete it. It's never happening. (Agent deletes line)
  ```

### 3. The Negotiation (Interaction)
- **Trigger**: Watcher detects file modification in `debt_inbox/`.
- **Action**: Parses the checked box.
- **Execution**:
  - **Refinance**: Edits source file: `// TODO: ...` -> `// TODO(deferred_until_2025): ...`
  - **Bankruptcy**: Uses `text-editor` to remove the line.
  - **Securitize**: Uses `fetch` to call GitHub API, gets Issue ID, updates source.

### 4. The Ledger (Persistence)
- **Memory Graph**: Stores the *outcome* of negotiations.
- **Credit Score**: Calculates a "Reliability Score" for each developer based on:
  - `Bankruptcy Rate` (Deleting without fixing)
  - `Refinance Rate` (Pushing the can down the road)
  - `Payoff Rate` (Actually fixing)

## Failure Modes & Recovery
1. **Desynchronization**: Code line numbers change while the Notice is pending.
   - *Fix*: The Notice includes a context hash. If the hash doesn't match the current file state, the Agent marks the Notice as "VOID" and regenerates it.
2. **Inbox Overflow**: Too many notices overwhelm the user.
   - *Fix*: "Debt Ceiling" policy. Only active the top 5 oldest debts at a time.
3. **Accidental Deletion**: Bankruptcy deletes critical context.
   - *Fix*: Agent works on a separate git branch `chore/debt-collection`, requiring PR merge.

## Autonomy Level
**High (Bureaucratic Autonomy)**.
The agent is fully autonomous in *finding* and *serving* notices. It requires Human Interaction to *resolve* them, but the interaction is constrained and structured (checking a box), minimizing cognitive load.
