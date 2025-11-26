# Design 2: The Surgeon (Aggressive)

## Purpose
The "Surgeon" is an active intervention agent. Its goal is **Micro-Vendorization**. It identifies heavy dependencies used for trivial tasks (e.g., importing `lodash` just for `_.get`) and surgically replaces them with local utility functions, allowing the heavy dependency to be uninstalled.

## Loop Structure
1.  **Identify:** Find single-function imports from heavy libraries (e.g., `import { method } from 'heavy-lib'`).
2.  **Source:**
    *   Use `web` to find the open-source implementation of that specific method (e.g., GitHub raw content).
    *   Alternatively, use `web` to find a "snippet" equivalent (StackOverflow, Gists).
3.  **Transplant:**
    *   Create a local file `src/vendor/[lib_name]/[method].js`.
    *   Paste the code (including necessary license headers).
4.  **Rewire:**
    *   Use `text-editor` to replace imports in the user's code to point to the new local file.
5.  **Verify:**
    *   Run project tests (`npm test`). If they fail, revert.
    *   If they pass, uninstall the original dependency.

## Tool Usage
*   **grep:** Locate candidate imports.
*   **web_brave_web_search:** Find source code or polyfills.
*   **fetch:** Download the raw source code.
*   **text-editor:** Apply patches to change import paths.
*   **shell:** Run tests and uninstall packages.

## Memory Architecture
*   **Session Memory:** Tracks which files were modified to allow for atomic reverts if tests fail.

## Failure Modes
*   **Hidden Complexity:** A "simple" function might rely on internal shared state or complex transitive dependencies that are hard to vendor.
*   **License Violation:** Copying code requires strict adherence to license compatibility (e.g., copying GPL code into a closed project).

## Human Touchpoints
*   **Approval:** The agent prepares the "Transplant Plan" (PR) but waits for explicit user confirmation before uninstalling the original package.
