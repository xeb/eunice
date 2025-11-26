# Design 2: The Truth Surveyor (Active)

## Purpose
To autonomously research a given topic and construct a comprehensive, balanced "Map of the Debate" by recursively searching the web for supporting and opposing views.

## Core Tools
- **web** (Brave Search): To find diverse viewpoints.
- **fetch**: To read content from search results.
- **memory**: To build the citation and argument graph.

## Loop Structure
1. **Seed**: User provides a topic (e.g., "Is Nuclear Energy Safe?").
2. **Expansion**:
   - Agent searches for "Arguments for X" and "Arguments against X".
   - Extracts top 3 distinct arguments for each side.
3. **Verification**: For each argument, search for "Evidence supporting [Argument]".
4. **Recursion**: If an argument depends on a sub-claim, treat that sub-claim as a new Issue and repeat (up to depth N).
5. **Output**: Generate a hierarchical Markdown report with citations.

## Memory Architecture
- **Entities**: `Topic`, `Claim`, `Evidence`, `URL`
- **Relations**: `cites`, `contradicts`, `corroborates`

## Failure Modes
- **Rabbit Holes**: Getting stuck in infinite recursion of sub-arguments. (Mitigation: Max depth limit).
- **Source Quality**: Citing low-quality sources. (Mitigation: Whitelist/Blacklist domains).

## Human Touchpoints
- **Initiation**: User must start the process.
- **Review**: User evaluates the final report.
