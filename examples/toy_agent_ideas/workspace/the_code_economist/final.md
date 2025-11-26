# The Code Economist: A Market-Based Technical Debt Manager

## Problem
Technical debt is often invisible, qualitative, and easy to ignore until it causes a crisis. Teams lack a "price signal" to tell them when refactoring is more valuable than feature work.

## Solution
"The Code Economist" is an autonomous agent that establishes an **Internal Prediction Market for Code Health**. It quantifies debt using financial metrics (Principal vs. Interest) and incentivizes maintenance through a "Bounty System".

## Core Logic: The Debt Equation
Unlike standard linters, this agent calculates debt dynamically:
1.  **Principal ($):** Static complexity (Cyclomatic complexity, length, TODO count).
2.  **Interest Rate (%):** File Volatility (Churn rate over last 30 days).
3.  **Liability ($/Month):** `Principal * Interest Rate`.
    *   *Example:* A complex file that is never touched has **High Principal, Zero Interest**. (Safe Debt).
    *   *Example:* A messy file touched daily has **Compounding Interest**. (Toxic Asset).

## Architecture & Loop

### 1. The Market Ticker (Daily Daemon)
- **Tools:** `shell` (git log), `grep` (complexity scanning).
- **Action:** Scans the codebase to update the "Stock Price" (Debt Score) of every module.
- **Persistence:** Updates the `memory` graph with historical trends to calculate "Inflation".

### 2. The IPO (Initial Problem Offering)
- **Trigger:** When a file's `Liability` crosses a threshold (e.g., "Costing us 5 hours/month").
- **Action:** The agent issues a **Bounty** in `MARKETPLACE.md`.
- **Value:** The bounty value (Credits) is proportional to the calculated Liability reduction.

### 3. Settlement (Refactoring Verification)
- **Trigger:** Developer runs `economist claim <file>` and then `economist resolve <file>`.
- **Tools:** `text-editor` (diff analysis), `shell` (test execution).
- **Logic:**
    - Checks if Complexity decreased.
    - Checks if Tests pass.
    - Awards "Credits" to the developer in the Memory Graph.

### 4. The Quarterly Report
- **Tools:** `filesystem` (write report).
- **Action:** Generates a "Financial Statement" showing:
    - **Total Debt Load**
    - **Deficit/Surplus** (New Debt vs. Paid Debt)
    - **Top 10 Toxic Assets**

## MCP Toolchain
- **memory:** The Central Bank Ledger. Stores balances, asset history, and transaction logs.
- **grep:** The Auditor. Static analysis of code patterns.
- **shell:** The Exchange. Interface for git operations and user commands.
- **filesystem:** The Printing Press. Publishes `ECONOMY.md` and `MARKETPLACE.md`.

## Human Interaction
- **Gamification:** Developers earn "Credits" (which can be mapped to real perks or just bragging rights).
- **Governance:** Users configure the "Exchange Rate" (how much complexity = 1 credit).

## Failure Modes & Recovery
- **Market Crash:** If a "refactoring" breaks the build, the agent declares "Bankruptcy" on that transaction and revokes credits.
- **Hyperinflation:** If everyone writes messy code, the agent raises the "Interest Rate" (volatility penalty) to make bounties more lucrative.

## Novel Insight
**"Liquidity of Maintenance"**: By converting abstract "messiness" into a "liquid asset" (a bounty with a price), we align incentives. Developers want to "buy" the debt to earn credits, turning maintenance from a chore into a profit center.
