# Design 1: The Backlog Gatekeeper (Conservative)

## Purpose
To prevent "Scope Creep" and "Backlog Rot" by automatically validating every new feature request against a defined Product Strategy before it is accepted into the active backlog.

## Problem Domain
Software backlogs often become dumping grounds for "good ideas" that never get built but clog up search results and mental bandwidth. Human PMs spend hours "grooming" tickets that should have been rejected immediately.

## Core Tools
- **Filesystem**: To read the `strategy.md` manifesto and the `backlog/` directory.
- **Grep**: To search for duplicate existing features or similar past rejections.
- **Shell**: To run git commands (e.g., checking who authored a ticket).

## Loop Structure
1. **Trigger**: Scheduled check (every hour) or triggered by a file change in `backlog/incoming/`.
2. **Analysis**:
   - Read the new ticket (Markdown).
   - Read `STRATEGY.md` (The constitution).
   - Use LLM (implied) to score "Alignment" (0-10).
3. **Action**:
   - If Score < 5: Move file to `backlog/rejected/` with a comment explaining *why* (citing the Strategy).
   - If Score >= 5: Move file to `backlog/approved/` and tag with relevant Strategic Pillar.
4. **Validation**: Grep for similar existing tickets to detect duplicates.

## Persistence
- **Filesystem-Native**: No complex database. State is stored entirely in the file structure (`incoming/`, `approved/`, `rejected/`) and Markdown frontmatter.

## Failure Modes
- **False Rejection**: Rejecting a brilliant innovation because it doesn't fit the *current* strategy.
- **Drift**: The `STRATEGY.md` becomes outdated, causing the agent to enforce obsolete rules.
- **Recovery**: Humans can simply move a file back from `rejected/` to `approved/` to override the agent.

## Human Touchpoints
- **Manifesto Updates**: Humans must write and update `STRATEGY.md`.
- **Overrides**: Humans review the `rejected/` folder periodically.
