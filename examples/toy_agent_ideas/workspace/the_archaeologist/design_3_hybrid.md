# Design 3: The Curator (Hybrid)

## Purpose
The Curator treats the codebase as a museum. Its goal is to *preserve* value while modernizing structure. It combines the "Mapper" (analysis) with the "Excavator" (action) but adds a conversational "Proposal" layer. It doesn't commit code; it creates detailed "Refactoring Proposals" (Markdown + Patch files) for the human to enact.

## Loop Structure
1. **Curate:** Monitor codebase for "Rot" (complex files, lack of comments, old syntax).
2. **Draft Proposal:**
   - Create a folder: `proposals/refactor_user_auth_service/`
   - Generate `context.md`: Explains *why* this needs change (cyclomatic complexity > 15).
   - Generate `plan.diff`: The proposed changes (using `text-editor` to draft).
   - Generate `risk_assessment.md`: What tests cover this? What might break?
3. **Notify:** Alert the user (log entry or notification).
4. **Wait/Act:**
   - If User approves (via flag/command), apply the `plan.diff`.
   - If User rejects, store the reason in Memory ("Too risky before Q4 release").

## Tool Usage
- **memory:** Stores the "history" of the code. "Why was this function added?" (User inputs observations).
- **grep/filesystem:** For generating the context and diffs.
- **text-editor:** To write the proposal artifacts.

## Memory Architecture
- **Annotation:** The graph is enriched with human intent.
  - `Function: login` -> `Observation: "Don't touch, legacy auth provider"`.
- **Policy:** The agent reads "Curator Rules" from memory (e.g., "Always prefer functional style").

## Failure Modes
- **Stale Proposals:** Code changes invalidate the diff before the user approves it.
  - *Recovery:* Agent checks file hashes before applying. If stale, it regenerates the proposal.

## Human Touchpoints
- **The Critic:** The human acts as the Art Critic, approving or rejecting the Curator's exhibits.
- **Feedback:** The agent learns from rejections (e.g., "Stop suggesting to rename this variable").
