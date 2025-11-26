# Agent: The Dependency Distiller

## Core Philosophy
Modern software development often involves "importing the kitchen sink" to get a glass of water. **The Dependency Distiller** fights this trend by auditing, extracting, and modernizing dependencies. It is a **"Software Osteopath"** that aligns the codebase with the underlying platform, removing unnecessary layers of abstraction.

## Architecture

### 1. The Diagnostic Loop (Auditor Mode)
*   **Trigger:** Scheduled weekly or manually triggered.
*   **Action:**
    *   Scans `package.json` / `requirements.txt`.
    *   Calculates "Bloat Factor" (Size vs. Import Frequency).
    *   Checks for "Zombie Dependencies" (installed but never imported).
    *   **Output:** A prioritized "Hit List" of dependencies to target.

### 2. The Extraction Loop (Surgeon Mode)
*   **Trigger:** User selects a dependency from the Hit List.
*   **Action:**
    *   **Analysis:** Finds all imports. If usage is sparse (e.g., only `_.get` and `_.set` from `lodash`), it proceeds.
    *   **Sourcing:** Fetches the source code for those specific functions from the library's repository or stable polyfills.
    *   **Vendorization:** Creates `src/utils/vendor/[lib_name].js` containing only the used code.
    *   **Refactoring:** Updates all import paths in the codebase to point to the local vendor file.
    *   **Verification:** Runs the test suite. If successful, uninstalls the original dependency.

### 3. The Modernization Loop (Native Advocate Mode)
*   **Trigger:** Continuous background scan.
*   **Action:**
    *   Matches code against a persistent Knowledge Graph of "Modern Equivalents" (e.g., `axios` -> `fetch`, `moment` -> `Intl`, `uuid` -> `crypto.randomUUID`).
    *   Checks `browserslist` / `engines` field to ensure target environment support.
    *   **Output:** Generates a Refactoring Plan (Markdown) or inline code comments suggesting the native alternative.

## Tool Integration
*   **filesystem & grep:** For deep code analysis and identifying usage patterns.
*   **web (Brave Search):** To find source code for extraction and verify browser support (MDN).
*   **memory:** To maintain the "Modernization Ruleset" (e.g., "If node >= 18, fetch is available") and track project-specific "Do Not Touch" lists.
*   **text-editor:** To perform the delicate surgery of changing imports and pasting code.

## Critical Safety Mechanisms
*   **Atomic Transactions:** All changes are applied to a temporary git branch.
*   **Test-Driven Gatekeeping:** No uninstallation occurs unless `npm test` passes.
*   **License Awareness:** The agent checks the license of the library before copying code (e.g., alerting the user if they are vendoring GPL code into a proprietary project).

## Future Extensibility
*   **Supply Chain Security:** By reducing external dependencies, the attack surface for malicious updates is reduced.
*   **Performance:** Less code to parse/compile for the JS engine.

