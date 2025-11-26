# Design 1: The Compliance Clerk (Conservative)

## Purpose
To ensure strict bureaucratic traceability between business requirements (Tickets) and technical implementation (Code) by enforcing a "No Code Without Ticket" policy.

## Loop Structure
1. **Poll Ticket System:** Every hour, fetch recently updated tickets from Jira/GitHub Issues (using `web` or `fetch`).
2. **Scan Git History:** Fetch the latest commits and PRs (using `filesystem` + `shell`).
3. **Link Verification:** Check if every Commit Message and PR Description contains a valid Ticket ID (Regex match).
4. **Status Check:** Verify that the referenced Ticket is in an "In Progress" or "Active" state (preventing work on closed/unapproved tickets).
5. **Report:** Generate a Markdown report listing "Orphan Commits" (code with no ticket) and "Zombie Tickets" (tickets with no code activity for >1 week).

## Tool Usage
- **web:** Search/Fetch issue tracker API (simulated via HTML scraping or API calls).
- **shell:** `git log`, `git diff`.
- **filesystem:** Read `CONTRIBUTING.md` to learn the regex pattern for ticket IDs (e.g., `JIRA-\d+`).
- **memory:** Store the mapping of `{TicketID -> [CommitHashes]}` to track coverage over time.

## Memory Architecture
- **Nodes:** `Ticket`, `Commit`, `PR`, `Author`.
- **Edges:** `IMPLEMENTS` (Commit -> Ticket), `MERGES` (PR -> Commit).
- **Persistence:** Simple graph to answer "Show me all code related to Ticket-123".

## Failure Modes
- **False Positives:** A typo in the ticket ID flags a valid commit as an orphan.
- **API Limits:** Rate limiting on the issue tracker.
- **Recovery:** Agent pauses and retries; Human can manually "claim" an orphan commit via a config file.

## Human Touchpoints
- **Report Review:** Humans receive a daily "Compliance Report".
- **Override:** Humans can whitelist "hotfix" commits that bypass the ticket process.
