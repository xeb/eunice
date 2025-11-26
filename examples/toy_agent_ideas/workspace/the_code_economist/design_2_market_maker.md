# Design 2: The Market Maker

## Purpose
"The Market Maker" gamifies technical debt reduction by creating an internal economy. It incentivizes developers to "buy" (refactor) toxic assets by offering "credits" that can be traded for privileges (e.g., skipping a standup, choosing the next task).

## Loop Structure
1. **Listing Assets:**
   - Identify "Toxic Assets" (High Volatility x Complexity).
   - "List" them on the internal market with a "Bounty" (Credit value).
2. **Bidding:**
   - Developers (simulated or real via CLI) "claim" a ticket.
   - The agent monitors the file.
3. **Settlement:**
   - When the file is modified and complexity drops, the agent validates the "Profit".
   - Credits are awarded to the developer's account in Memory.
4. **Market Dynamics:**
   - If a file causes a bug (detected via simple keyword search in commit msgs like "fix"), its Debt Value spikes.

## Tool Usage
- **memory:** Stores User Accounts, Wallets, and Asset Prices.
- **shell:** Git hooks or post-commit analysis to verify refactoring.
- **filesystem:** Maintains a `MARKETPLACE.md` file listing open bounties.
- **web:** (Optional) Search for salary data to peg debt to real currency? (Maybe too much).

## Memory Architecture
- **Entities:** `Developer`, `Asset` (File), `Transaction`.
- **Relations:**
  - `Developer -> OWNS -> Wallet`
  - `Asset -> HAS_BOUNTY -> Value`
- **Logic:** Smart Contract-like logic in the agent loop to release funds on verification.

## Failure Modes
- **Gaming the System:** Deleting lines just to lower complexity.
- **Inflation:** Too many easy bounties devalue the credits.
- **Recovery:** "Market Crash" reset (clear all wallets/bounties).

## Human Touchpoints
- **Active:** Developers explicitly claim bounties via CLI commands.
- **Verification:** Senior devs might need to "sign off" on big payouts.
