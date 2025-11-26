# Design 3: The Dialectic Engine (Hybrid)

## Purpose
A persistent background daemon that integrates the user's private notes with public knowledge. It acts as a "Devil's Advocate," actively searching for evidence that contradicts the user's current beliefs to prevent echo chambers.

## Core Tools
- **memory**: Stores the "Worldview Graph" (User's beliefs + External Facts).
- **filesystem**: Watches user notes (`/zettelkasten`).
- **web**: Searches for counter-evidence.
- **text-editor**: Appends "Dialectic Footnotes" to user files.

## Loop Structure
1. **Ingest**: Watch file system. When user writes "Nuclear energy is dangerous because X", parse into Graph.
2. **Contextualize**: Search Memory: "Do we have evidence about X?"
3. **Challenge**: If Memory is empty, Search Web: "Safety statistics of X".
4. **Synthesize**:
   - If search results support user: Add `[Corroborated by Source Y]` metadata.
   - If search results contradict: Add a "Dialectic Footnote" to the file: `> [!NOTE] Counterpoint: Source Z argues that...`
5. **Evolve**: Over time, build a graph of *Stable Truths* (claims that survived challenges).

## Memory Architecture
- **Nodes**: `Claim` (User), `Fact` (External), `Conflict`.
- **Edges**: `challenges`, `refines`, `merged_into`.

## Failure Modes
- **Annoyance**: Agent modifying files too frequently. (Mitigation: Use a separate "Shadow File" or `comments.md`).
- **Hallucination**: Misinterpreting a search result as a contradiction when it isn't.

## Human Touchpoints
- **Configuration**: User sets "Aggressiveness" (Cheerleader vs. Critic).
- **Interaction**: User replies to the agent's footnotes in the text file.
