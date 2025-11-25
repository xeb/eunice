# Design 1: The Code Cartographer (Conservative)

## Purpose
A purely observational agent designed to map, document, and visualize legacy codebases without making any modifications. It addresses the problem of "lost tribal knowledge" by recovering the structural architecture of the system.

## Core Toolset
* **filesystem**: For traversing directory structures and reading file contents.
* **grep**: For identifying symbol definitions, imports, and usages.
* **memory**: For building a persistent graph of code entities (Files, Classes, Functions) and their relationships.

## Loop Structure
1. **Survey**: The agent recursively lists all files in the target directory to understand the project footprint.
2. **Indexing**: 
   - It iterates through files, reading content.
   - Uses regex (via `grep` or internal processing) to extract entity definitions (classes, functions, variables).
   - Creates nodes in the **memory** graph for each entity.
3. **Linking**:
   - It scans for import statements and function calls.
   - Creates edges in the **memory** graph (e.g., `File A -> IMPORTS -> File B`, `Func X -> CALLS -> Func Y`).
4. **Reporting**:
   - Upon completion, it queries the memory graph to generate a "Project Atlas" in Markdown.
   - This includes dependency trees, orphan file lists, and "God Class" identification (nodes with excessive connections).

## Memory Architecture
* **Entities**: `File`, `Directory`, `Class`, `Function`, `Variable`.
* **Observations**: Lines of code, cyclomatic complexity score, documentation presence.
* **Relations**: `CONTAINS`, `IMPORTS`, `INHERITS_FROM`, `CALLS`, `MODIFIES`.

## Failure Modes & Recovery
* **Graph Explosion**: On massive monorepos, the graph might become too large. *Mitigation*: Limit depth or scope by directory; use "summary nodes" for dense subgraphs.
* **Language Ambiguity**: Regex parsing is fragile for complex languages (e.g., C++ macros). *Mitigation*: Fallback to "File-level" dependency mapping if fine-grained parsing fails.

## Human Touchpoints
* **Approval**: None required for execution (read-only).
* **Interaction**: User queries the agent: "Where is user authentication handled?" -> Agent queries graph -> Returns list of relevant files/functions.

