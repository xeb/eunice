# Design 1: The Conservative Invariant Linter

## Purpose
A passive analysis agent that identifies gaps between "documented constraints" (in comments/docs) and "enforced constraints" (in code/tests). It does not modify code but generates high-confidence reports for developers.

## Loop Structure
1. **Scan**: Iterates through codebase using `grep` to find:
   - Natural language constraints (e.g., "must be", "never", "always" in comments).
   - Code assertions (`assert`, `require`, `if (...) throw`).
2. **Graph Construction**: Builds a `memory` graph mapping:
   - `Entity` (e.g., "User")
   - `Property` (e.g., "email")
   - `Constraint` (e.g., "unique", "not null")
   - `Source` (e.g., "User.ts:42", "README.md:10")
3. **Verification Analysis**:
   - Checks if a "Doc Constraint" has a matching "Code Constraint".
   - Checks if a "Code Constraint" has a matching "Test Case".
4. **Reporting**:
   - Generates `invariant_report.md` listing:
     - **Phantom Constraints**: Documented but not enforced.
     - **Hidden Rules**: Enforced but not documented.
     - **Zombie Constraints**: Documented but contradicted by code.

## Tool Usage
- `grep`: Keyword searching ("must", "should", "invariant", "@param").
- `filesystem`: Reading file content for context.
- `memory`: Storing the "Constraint Graph" to track status over time (avoiding re-reporting known issues).

## Memory Architecture
- **Entities**: `Constraint`, `SourceFile`, `DomainEntity`.
- **Relations**: `MENTIONS`, `ENFORCES`, `CONTRADICTS`.
- **Observations**: "File X line Y says 'Input must be positive'."

## Failure Modes
- **False Positives**: Misinterpreting "we *should* do this" (future tense) as a current constraint.
- **Noise**: Reporting trivial findings.
- **Recovery**: User can "mute" constraints in the memory graph via a config file.

## Human Touchpoints
- **Config**: User defines ignored paths/patterns.
- **Report Review**: User reads the markdown report and acts on it manually.
