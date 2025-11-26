# Design 2: The Market Scout (Innovative)

## Purpose
To autonomously populate the backlog with "Gap Analysis" features by continuously monitoring competitors and market trends.

## Problem Domain
Product teams often get tunnel vision, focusing on their own legacy code while competitors ship new standard features. This agent ensures the product doesn't fall behind.

## Core Tools
- **Web (Brave Search)**: To find competitor changelogs, pricing pages, and "Alternative to X" blog posts.
- **Memory**: To build a "Market Graph" of competitors, their features, and user sentiment.
- **Grep**: To check if a "competitor feature" already exists in the local codebase.

## Loop Structure
1. **Surveillance**:
   - Query: "Competitor Name changelog" or "Top requested features for [Domain]".
   - Fetch: Download relevant pages.
2. **Synthesis**:
   - Extract "Feature Entities" (e.g., "Dark Mode", "SAML SSO").
   - Update **Memory Graph**: `(Competitor A) --[HAS_FEATURE]--> (SAML SSO)`.
3. **Comparison**:
   - For each *new* external feature, query local codebase (Grep) for related terms.
   - If missing: Create a **Memory Node** `MissingFeature: SAML SSO`.
4. **Proposal**:
   - Once a week, generate a "Market Gap Report" markdown file in the backlog.
   - "Competitors A, B, and C have shipped 'AI Summaries'. We do not have this. Priority: High."

## Persistence
- **Memory Graph**: Stores the evolving state of the external market (Competitors -> Features).
- **Filesystem**: Output only (Reports).

## Failure Modes
- **Hallucination**: Inventing features competitors don't actually have (reading rumors as facts).
- **Semantics**: Failing to realize that "SAML" (competitor term) is the same as "Enterprise Login" (local term).
- **Noise**: Flooding the team with trivial features.

## Human Touchpoints
- **Seed List**: Humans provide the list of competitors to watch.
- **Approval**: Humans must explicitly turn a "Gap Report" item into a "Dev Ticket".
