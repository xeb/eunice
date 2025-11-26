# Design 2: The Talent Scout (Innovative)

## Purpose
To actively identify, nurture, and connect valuable contributors in an open-source community. Instead of focusing on "keeping bad people out," it focuses on "keeping good people in" by recognizing their work and connecting them with peers.

## Core Loop
1. **Scan:** Read daily threads/issues/PRs.
2. **Analyze:**
   - Use `web_brave_summarizer` (simulated) or local NLP to extract *topics* and *sentiment*.
   - Identify "Helpful Acts" (answering questions, debugging).
3. **Map:** Update the Memory Graph:
   - Link `User` to `Topic` (e.g., "User A knows about Database Migrations").
   - Increment `Helpfulness` counters.
4. **Connect:**
   - **Matchmaking:** If New User asks about X, and User A is an expert on X, ping User A (or draft a suggestion).
   - **Recognition:** If User A passes a threshold, draft a "Kudos" post or suggest them for a "Maintainer Track".

## Tool Usage
- **memory:** The core "Social Graph" (Who knows what).
- **web:** Searching for user's cross-platform presence (GitHub, Twitter) to build a holistic profile (optional/privacy-sensitive).
- **filesystem:** Generating "Weekly Shoutout" drafts and "Talent Pipeline" reports.

## Memory Architecture
- **Entities:** `User`, `Skill`, `Contribution`, `Project`.
- **Relations:** `User MENTORS User`, `User EXPERT_IN Skill`, `Contribution SOLVED Issue`.

## Failure Modes
- **The "Echo Chamber":** Recommending the same 3 experts for everything, causing burnout. *Recovery:* `LoadBalancing` logic in the recommendation engine.
- **Privacy Creep:** Knowing too much about a user. *Recovery:* Strict "Public Data Only" policy; no cross-referencing without consent.

## Human Touchpoints
- **Nomination Approval:** Agent suggests a user for promotion; Human approves.
- **Introduction Review:** Agent drafts an intro between two users; Human sends it.
