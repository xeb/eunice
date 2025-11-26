# Design 3: The Team Mirror

## Purpose
A collective intelligence agent that anonymizes and aggregates error patterns across a team to optimize code review routing and pair programming matches.

## Loop Structure
1. **Harvesting:** Runs nightly on the CI/CD server. Analyzes `git log` of the main repository.
2. **Attribution:**
   - Identifies who introduced a bug (the "fixee") and who fixed it (the "fixer").
   - Builds a matrix of expertise: "Alice fixes Bob's SQL errors." "Charlie fixes Alice's CSS errors."
3. **Routing:**
   - When a new Pull Request is opened, the agent analyzes the changed files/concepts.
   - It assigns reviewers based on the *Inverse Error Probability*.
   - "This PR touches the Payment API. Bob has a 15% bug rate here. Alice corrects 90% of Payment bugs. Assign Alice."
4. **Nudging:**
   - It generates a "Team Health Report" (anonymized).
   - "Team Trend: We have had a 40% increase in Concurrency bugs this month. Recommended Resource: [Link to Concurrency Tutorial]."

## Tool Usage
- **shell:** Git operations.
- **fetch:** GitHub/GitLab API for PR management.
- **memory:** Storing the social graph of skills and deficits.
- **web:** Finding training resources for identified team deficits.

## Memory Architecture
- **Social Graph:**
  - Nodes: `Developer`, `Module`, `Skill`.
  - Edges: `Developer --HAS_MASTERY_IN--> Skill`, `Developer --PRONE_TO--> ErrorType`.

## Failure Modes
- **Privacy/Trust:** Developers feel surveilled or judged.
  - *Recovery:* Strict anonymization in reports. The agent only uses data for *routing* help, not for performance reviews.
- **Gaming:** People might avoid tricky tasks to keep their "stats" clean.
  - *Mitigation:* The agent rewards "Fixing" more than it penalizes "Breaking".

## Human Touchpoints
- **Configuration:** Team agrees on what constitutes a "bug" (e.g., specific tags in Jira/GitHub).
- **Opt-out:** Developers can pause data collection.
