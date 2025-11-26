# Design 1: The Town Square Watchman (Conservative)

## Purpose
To provide context-aware safety and moderation for online communities, reducing false positives in enforcement by maintaining a long-term "reputation graph" of users. unlike standard AutoMod bots which are stateless and rule-based, the Watchman knows *who* is speaking.

## Core Loop
1. **Ingest:** Periodically fetch recent messages/comments (via `web` or API simulation).
2. **Contextualize:** For each message, query `memory` for the User Entity.
   - Retrieve `trust_score`, `tenure`, `past_violations`, and `topic_expertise`.
3. **Evaluate:**
   - Run stateless checks (regex/keywords) via `grep` logic.
   - Run stateful checks: "Is this highly trusted user using a banned word in a meta-discussion context?"
4. **Act:**
   - **Low Risk:** Auto-flag to filesystem report.
   - **High Risk:** Auto-remove (simulated) + Log to memory.
   - **Edge Case:** If a trusted user violates a rule, generate a "Draft DM" for a human moderator to review, rather than an instant ban.

## Tool Usage
- **memory:** Stores `User` nodes, `Violation` events, and `TrustScore` properties.
- **web/fetch:** Simulates reading forum/chat logs.
- **grep:** Fast pattern matching for known slurs/spam patterns.
- **filesystem:** Stores `incident_reports/` and `ban_appeals/`.

## Memory Architecture
- **Entities:** `User`, `Message`, `Rule`, `Violation`.
- **Relations:** `User AUTHOR_OF Message`, `Message VIOLATES Rule`, `User HAS_TRUST_SCORE Integer`.
- **Key Logic:** Trust score decays slowly over time if inactive, increases with accepted posts.

## Failure Modes
- **The "Fallen Angel":** A high-trust user goes rogue. The agent might hesitate to ban them. *Recovery:* Hard limits on severity (e.g., zero tolerance for malware links regardless of trust).
- **Context Blindness:** Sarcasm is hard. *Recovery:* All auto-actions are logged to a "Review Queue" in filesystem.

## Human Touchpoints
- **Appeal Review:** Humans review the `decisions.log` file.
- **Rule Calibration:** Humans edit `rules.json` to update forbidden terms.
