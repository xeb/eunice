# Design 1: The Reference Librarian (Conservative)

## Purpose
To passively identify "Not Invented Here" (NIH) code and suggest standard alternatives without modifying logic. It acts as an educational layer, annotating the codebase with "Global Context."

## Loop Structure
1. **Discovery**: Scan `utils/`, `common/`, `helpers/` directories using `grep`.
2. **Analysis**: Extract function signatures and docstrings.
3. **Research**: Query Brave Search with "standard library equivalent for [function description]".
4. **Verification**: Check if the suggested library is already in `package.json` or `requirements.txt`, or if it is a built-in standard library.
5. **Annotation**:
    - If a high-confidence match is found, create a "Knowledge Entity" in memory.
    - Append a "Cosmopolitan Report" to the repository root.
    - *Optionally* add a comment above the function: `// TODO: Consider replacing with lodash.flatten`.

## Tool Usage
- **grep**: Identifying candidate functions (high complexity, generic names like `deepMerge`, `validateEmail`).
- **web**: Brave Search to find the "Canon" implementation.
- **filesystem**: Reading code, writing reports.
- **memory**: Storing "Ignored Suggestions" so it doesn't nag about the same thing twice.

## Memory Architecture
- **Entities**: `LocalFunction`, `ExternalLibrary`.
- **Relations**: `reinvents`, `is_superior_to`, `ignored_by_user`.
- **Persistence**: Graph survives to remember user preferences ("We use custom JSON parser for speed, stop asking").

## Failure Modes
- **False Positives**: Suggesting a library that doesn't actually cover the edge case.
- **Recovery**: User adds `@cosmopolitan-ignore` to the docstring. Agent parses this and adds to memory.

## Human Touchpoints
- **Read-Only**: The agent never changes code logic.
- **Report Review**: Human reads the report and decides whether to refactor.
