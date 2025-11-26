# Design 1: The Sponsorship Accountant (Conservative)

## Purpose
A "Social Dependency" scanner that generates financial reports for open-source usage. It answers the question: "Who are we freeloading off of, and how much would it cost to pay our fair share?"

## Loop Structure
1.  **Scan:** On a schedule (e.g., weekly), scan all dependency manifests (`package.json`, `Cargo.toml`, `go.mod`).
2.  **Enrich:** Query GitHub API / Open Collective API to find funding links for each dependency.
3.  **Calculate:** Apply a simple heuristic (e.g., "Criticality Score" based on depth + usage frequency) to estimate "Value Received".
4.  **Report:** Generate a `SPONSORSHIP.md` report listing:
    *   Top 10 Critical Unfunded Dependencies
    *   Top 10 Critical Funded Dependencies (with current sponsorship status)
    *   Recommended Budget Distribution
5.  **Alert:** If a critical dependency is "At Risk" (archived, no funding, low bus factor), open a GitHub Issue.

## Tool Usage
*   **filesystem:** Read dependency files, write markdown reports.
*   **web (fetch/search):** Query GitHub/OpenCollective for funding data.
*   **memory:** Cache funding data to avoid API rate limits; store "Ignore" lists for packages users don't want to fund.

## Memory Architecture
*   **Nodes:** `Package`, `Maintainer`, `Organization`.
*   **Relations:** `DEPENDS_ON`, `MAINTAINED_BY`, `FUNDED_VIA`.
*   **Observations:** "Package X has a funding goal of $500/mo and is at 10%."

## Failure Modes
*   **API Limits:** Handling rate limits from GitHub/OpenCollective.
*   **Misattribution:** Identifying the *correct* funding link (avoiding scams).
*   **Noise:** Generating too many alerts for small packages.

## Human Touchpoints
*   **Read-Only:** Humans read the generated reports.
*   **Budget Approval:** Humans manually set up sponsorships based on the report.
