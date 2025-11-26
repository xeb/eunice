# Design 1: The Checklist Compiler

## Purpose
A lightweight, on-demand tool that converts complex, jargon-heavy administrative webpages (government sites, legal how-tos) into clean, actionable Markdown checklists. It does not maintain state or scan user files.

## Core Loop
1. **Trigger:** User provides a URL or a query (e.g., "How to apply for a specialized worker visa in Japan").
2. **Research:** Agent uses `web_brave_web_search` to find authoritative sources.
3. **Synthesis:** Agent reads the content and extracts:
   - Prerequisites (Documents needed)
   - Steps (Sequential actions)
   - Costs (Fees)
   - Deadlines
4. **Output:** Agent writes a `checklists/<topic>.md` file with checkboxes.
5. **Termination:** Agent exits.

## Tool Usage
- **web_brave_web_search**: To find the source material.
- **filesystem_write_file**: To save the resulting checklist.
- **shell**: To maybe run a quick `curl` if needed (optional).

## Memory Architecture
- **Stateless:** No persistent graph. Every run is fresh. This minimizes privacy risks and complexity.

## Failure Modes
- **Hallucination:** Might invent requirements. Mitigation: Include source URLs next to every checkbox.
- **Outdated Info:** Source might be old. Mitigation: Check page date.

## Human Touchpoints
- User initiates the request.
- User manually checks off items.
- User manually gathers files.

## Pros/Cons
- **Pros:** Simple, safe, privacy-preserving, easy to verify.
- **Cons:** High friction for the user (still have to do the work), no context awareness.
