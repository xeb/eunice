# Design 2: The Promise Keeper (Innovative Variant - "The Reaper")

## Purpose
To aggressively enforce code hygiene by treating TODOs as ephemeral artifacts with a strict Time-To-Live (TTL). If a TODO is not converted to a ticket or fixed within X days, it is deleted.

## Loop Structure
1. **Scan & Blame**: Identify all TODOs and their age.
2. **Triage**:
   - **Fresh (< 30 days)**: Ignore.
   - **Stale (30-90 days)**: Append Warning Comment `// WARNING: Expiring in 10 days`.
   - **Bankrupt (> 90 days)**: DELETE the comment line.
3. **Escalation**: For "Critical" FIXMEs, instead of deleting, it uses `fetch` to POST a new issue to the GitHub/Jira API, then replaces the comment with the Issue URL.

## Tool Usage
- **text-editor**: Used to surgically remove lines or append warnings.
- **shell**: Git operations (create branch `chore/reap-todos`).
- **web/fetch**: To integrate with Issue Trackers.

## Memory Architecture
- **Stateless**: Relies primarily on the filesystem (code) as the source of truth.
- **Memory**: Used only to cache API tokens or API rate limits.

## Failure Modes
- **Destructive**: Deleting context ("TODO: Don't touch this, it explodes") might be dangerous.
- **Recovery**: The agent commits to a separate branch, allowing easy revert.
- **Looping**: Re-adding the same TODO after the agent deletes it.

## Human Touchpoints
- **PR Review**: Humans must merge the `chore/reap-todos` branch.
