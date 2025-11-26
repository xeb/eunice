# Design 3: The Velocity Trader

## Purpose
To gamify estimation by treating time as a currency ("Chronos Credits") and the agent as a "Market Maker" that sets the odds based on historical performance.

## Loop Structure
1. **The Wager:**
   - User creates a task: `[ ] Implement Login (Bid: 4h)`.
   - Agent analyzes history. If the user usually underestimates Login tasks by 2x, the Agent "Counters" the bid: `Accepting bid at 8h. Reward: 50 Credits. Penalty: -100 Credits.`
2. **The Work:**
   - User commits code. Agent tracks `git` timestamps.
3. **The Settlement:**
   - If done in < 8h, User wins credits.
   - If done in > 8h, User loses credits.
   - A "Margin Call" (Warning) is issued if the user is 80% through the time but only 20% through the lines-of-code (compared to average).
4. **The Leaderboard:**
   - A global leaderboard (if multiple agents connect) or local high-score.

## Tool Usage
- **shell:** `git log`.
- **filesystem:** `bets.md` (Ledger).
- **memory:** Stores the "User Credit Score" and "Risk Profile".

## Memory Architecture
- **Financial Model:** Stores a "Risk Score" for the user. High risk = Agent requires higher time buffers.
- **Transaction Log:** Every task is a transaction.

## Failure Modes
- **Gaming the System:** User might make empty commits to "stop the clock" or delay commits to "sandbag" estimates.
- **Stress:** Gamification can lead to burnout if the penalties feel too real or the agent is too harsh.

## Human Touchpoints
- High interaction. Every task is a negotiation.
