# Agent: The Community Shepherd

## Purpose
To act as a holistic "Digital Community Manager" that automates the dual responsibilities of **Safety** (moderation) and **Growth** (connection/recognition). It shifts the paradigm from "filtering bad content" to "cultivating social capital."

## Core Insight
**Contextual Reputation Graph:** Unlike standard bots that judge messages in isolation (stateless), the Shepherd judges messages based on the *author's position in the social graph* (stateful). A "trusted elder" uses different language than a "new account." Simultaneously, this same graph is used to *recommend mentors* and *spot burnout*.

## Architecture

### 1. The Social Graph (Memory)
The agent maintains a persistent graph in the `memory` server:
- **Entities:** `User`, `Topic`, `Interaction` (Help, Conflict, Chill), `Role`.
- **Relations:** 
  - `User HAS_REPUTATION Score`
  - `User EXPERT_IN Topic`
  - `User MENTORED User`
  - `User AT_RISK_OF Burnout` (Temporary state)

### 2. The Loop (Autonomy)
The agent runs as a background daemon with three phases:

#### Phase A: The Watchman (Safety)
- **Trigger:** New message stream.
- **Action:** Check message content + User Trust Score.
- **Logic:**
  - *New User + Slur* -> Immediate Ban.
  - *Trusted User + Slur* -> "Context Flag" (Draft report for human, don't auto-ban).
  - *Heated Argument* -> If 2 users interact negatively > 5 times in 1 hour, trigger "Cool Down" (DM both suggesting a break).

#### Phase B: The Scout (Growth)
- **Trigger:** Daily summary scan.
- **Action:** Identify "Helpful Acts" (e.g., long code blocks, "Thank you" replies).
- **Logic:**
  - Increment `Helpfulness` score.
  - **Matchmaking:** If User A asks about "GraphQL" and User B is flagged as "GraphQL Expert" (but inactive), ping User B? (Only if configured).
  - **Recognition:** If User C crosses a contribution threshold, draft a "New Contributor Shoutout" for the weekly newsletter.

#### Phase C: The Gardener (Health)
- **Trigger:** Weekly analysis.
- **Action:** Detect "Dead Zones" (quiet channels) and "Burnout" (top contributors posting less or more negatively).
- **Logic:** Draft a "Community Health Report" suggesting interventions (e.g., "Revive #python-help with a prompt", "Check in on maintainer X").

## Tool Usage
- **memory:** Stores the Social Capital Graph (Users, Skills, Trust).
- **web:** Used to cross-reference users (e.g., "Is this new user a known troll on other platforms?" - *strictly optional/privacy-gated*) and to fetch documentation to verify "correct" answers.
- **filesystem:** 
  - `inbox/reports/`: Drafts of ban appeals and conflict flags.
  - `outbox/shoutouts/`: Drafts of recognition posts.
  - `logs/chat/`: Raw message logs for analysis.
- **grep:** Fast pattern matching for initial triage.

## Persistence Strategy
**Hybrid:** 
- **Graph (Memory):** The "Who knows who" and "Trust" model.
- **Filesystem:** The "Paper Trail" (Logs, Reports, Drafts). 
This allows the agent to be "rebooted" without losing the social context it has built up over months.

## Failure Modes & Recovery
1. **Bias Amplification:** If the agent learns that "senior devs are always right," it might ignore toxicity from them.
   - *Fix:* Hard "Code of Conduct" rules that override Trust Scores (e.g., specific hate speech is never allowed).
2. **Creepy Surveillance:** Users feeling watched.
   - *Fix:* Transparency. The agent has a public command `!myprofile` where users can see their own graph node and request deletion (GDPR compliance).
3. **False Positive "Burnout":** A user just goes on vacation.
   - *Fix:* The "Burnout Alert" is just a report for a human manager, never an automated intervention.

## Human in the Loop
- **Judiciary:** The agent *never* permanently bans a high-trust user without human sign-off.
- **Promoter:** The agent *drafts* shoutouts, but a human *publishes* them.
