# Design 2: The Dependency Broker (Aggressive)

## Purpose
To aggressively eliminate local maintenance burden by replacing custom code with battle-tested libraries. It treats "Lines of Code" as a liability to be minimized.

## Loop Structure
1. **Targeting**: Identify "High Cost" local functions (frequent churn, low coverage, high complexity).
2. **Sourcing**: Find a drop-in replacement library via Web Search.
3. **Auditing**: Check the library's health (Last commit, stars, open issues) via Web/Fetch.
4. **Negotiation (Simulation)**:
    - Create a shadow branch.
    - Install the library.
    - Replace the local function body with the library call.
    - Run the project's test suite.
5. **Proposal**: If tests pass, open a PR (or save a patch file) with the message: "Deleted 50 lines of custom code, replaced with `requests`. Tests passed."

## Tool Usage
- **shell**: `git checkout`, `npm install`, `pytest`.
- **text-editor**: Surgical replacement of function bodies.
- **web**: Vetting library security and maintenance status.

## Memory Architecture
- **Entities**: `DependencyCandidate`, `RefactoringAttempt`.
- **Relations**: `failed_tests`, `passed_tests`, `introduced_regression`.
- **Persistence**: Remembers which libraries broke the build so it doesn't try them again.

## Failure Modes
- **Subtle Incompatibilities**: The library works 99% of the time but fails on a specific edge case not covered by tests.
- **Bloat**: Adding a 10MB library to replace a 5-line function.
- **Recovery**: Rollback git branch. Record "Bloat" observation in memory.

## Human Touchpoints
- **Merge Authority**: Human must merge the PR.
- **Dependency Approval**: Human must approve adding a new dependency to the project.
