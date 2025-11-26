# Design 3: The Governance Enforcer

## Purpose
The **Governance Enforcer** treats the Tech Radar as a living policy document. It doesn't just suggest changes; it actively prevents "Technical Pollution" by enforcing the radar's decisions. If a technology is moved to "Hold", the agent ensures no *new* code uses it.

## Loop Structure
1.  **Policy Definition:**
    - Reads the `tech-radar.json` (source of truth).
    - Identifies all libraries in the "Hold" or "Stop" ring.
2.  **Enforcement (Pre-emptive):**
    - The agent installs/updates linter configurations (e.g., `eslint-plugin-restrict-imports`, `archunit`) to explicitly ban the import of "Hold" libraries.
    - It runs `grep_search` to identify existing violations.
3.  **Deprecation (Retroactive):**
    - For existing usage of "Hold" libraries, the agent uses `text-editor` to append `@deprecated` or `# FIXME: TechRadar-Hold` comments to the lines of code.
    - It creates a "Debt Backlog" file listing all violations that need to be cleaned up.
4.  **Gatekeeping:**
    - The agent can be invoked as a CI step. If it detects a *new* usage of a "Hold" library in a Pull Request, it fails the build with a message: "Compliance Error: 'lodash' is on Hold. Use 'lodash-es' or native methods instead."

## Tool Usage
-   **filesystem:** To write linter config files (, ) and edit source code.
-   **shell:** To execute linters and CI checks.
-   **grep:** To find usages of banned packages.

## Memory Architecture
-   **Policy-as-Code:** The "Memory" is the  file itself. The agent enforces what is written there.
-   **Violation Log:** Maintains a  to track progress over time (e.g., "Violations reduced from 500 to 450").

## Failure Modes
-   **False Positives:** Blocking a valid use case (e.g., a migration script that *needs* to import the old library to migrate data). *Mitigation:* Support `// eslint-disable-line` or explicit overrides.
-   **Developer Friction:** Developers might get annoyed if the agent is too strict. *Mitigation:* "Warn" mode vs "Error" mode.

## Human Touchpoints
-   **Policy Updates:** Humans decide when to move an item to "Hold". The agent executes the will of the humans.
-   **Exception Handling:** Humans can whitelist specific files.

## Pros & Cons
-   **Pros:** Stops technical debt from growing. Enforces consistency automatically.
-   **Cons:** High risk of friction. Requires deep integration with build tools (Lint, CI). Can be seen as "draconian."
