# Design 3: The Hybrid Curator (Balanced)

## Purpose
To build a high-quality, verified map of an ecosystem where the agent does the legwork but the human acts as the Editor-in-Chief.

## Loop Structure
1. **Research Phase**:
   - Agent selects a focus topic from the backlog.
   - Uses `web_brave_web_search` and `fetch_fetch` to gather data.
   - Drafts a "Proposed Update" (new nodes/edges) in a JSON file.
2. **Review Request**:
   - Agent pauses and notifies the user (e.g., creates a `pending_review.md` file).
3. **Human Approval**:
   - User reviews the file. User can edit, approve, or reject entries.
   - User signals completion (e.g., moves file to `approved/` folder).
4. **Commit Phase**:
   - Agent wakes up, reads `approved/` files.
   - Executes `memory_create_entities` and `memory_create_relations`.
   - Archives the file.

## Tool Usage
- **filesystem**: The communication interface (staging area).
- **memory**: The trusted source of truth (only contains verified data).
- **web**: Research gathering.

## Memory Architecture
- **Dual-State**:
  1. **Staging (Files)**: Noisy, potentially incorrect data.
  2. **Production (Memory Graph)**: Clean, human-verified data.

## Failure Modes
- **Bottleneck**: The agent moves faster than the human can review. Recovery: Agent switches to "passive collection" mode, just piling up drafts.
- **Staleness**: If the human doesn't review, the "new" information becomes old.

## Human Touchpoints
- **High Touch**: Human is in the loop for every batch of updates.
- **Quality Control**: Ensures the knowledge graph remains high signal-to-noise.
