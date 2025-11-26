# Final Design: The Semver Sentinel

## Purpose
A highly autonomous, safety-critical agent that acts as the "Guardian of the Public API". It decouples *versioning* (mathematical truth of code compatibility) from *description* (human intent), ensuring that software releases rarely accidentally break consumers due to mislabeled semantic versions.

## Core Insight
**"The Map is the Territory":** By maintaining a persistent **Memory Graph of the Public API Surface**, the agent can mathematically prove whether a set of file changes constitutes a Patch, Minor, or Major release, regardless of what the developer wrote in the commit message.

## Tool Selection
*   **Core:** `grep` (for fast static analysis), `memory` (for API state comparison), `filesystem` (for changelogs).
*   **Support:** `shell` (git integration).

## Architecture

### 1. The Surveillance Loop (Daemon)
*   **Trigger:** Watches file system events or polls `git status` for changes in source directories.
*   **Action:**
    *   Identifies changed files.
    *   **Micro-Parsing:** Uses `grep` and regex-based heuristics (or language servers if available via shell) to extract "Signatures" from changed files.
        *   *Example:* `export function login(user: string, pass: string)`
    *   **Graph Lookup:** Queries the **Memory Graph** for the *previous* signature of these functions.
    *   **Impact Assessment:**
        *   *Signature Match:* No API change. (Potential PATCH).
        *   *Signature Mismatch:*
            *   Added optional arg? -> MINOR.
            *   Changed required arg? -> MAJOR.
            *   Removed export? -> MAJOR.
    *   **State Update:** Updates the "Pending Release" node in memory with the calculated impact.

### 2. The Gatekeeper Loop (Pre-Commit/CI)
*   **Trigger:** User attempts to `git commit` or `git push`.
*   **Action:**
    *   Agent checks the "Pending Release" state.
    *   **Verification:**
        *   If the developer's commit message says `fix:` but the Impact Assessment says **MAJOR**, the agent blocks the operation (or warns).
        *   *Message:* "⚠️ Blocked: You modified the signature of `login()` which is a Breaking Change, but used a `fix:` prefix. Use `feat!:` or `BREAKING CHANGE:`."

### 3. The Scribe Loop (Release Time)
*   **Trigger:** User runs `sentinel release`.
*   **Action:**
    *   Aggregates all changes in the "Pending Release" bucket.
    *   Calculates the final Version Bump (Max of all changes).
    *   Generates `CHANGELOG.md` with two sections:
        *   **Narrative:** (Derived from commit messages).
        *   **API Contract Changes:** (derived from Graph Diff, e.g., "🔴 `login`: `password` arg became required").
    *   Updates `package.json`.
    *   Commits and Tags.

## Memory Graph Structure
*   **Entity:** `Function`, `Class`, `Interface`.
*   **Properties:** `signature_hash`, `filepath`, `exported` (boolean).
*   **Relations:** `DEPENDS_ON` (internal deps), `PART_OF_RELEASE` (version history).
*   **Observation:** "Signature changed from hash A to B".

## Recovery & Failure Modes
*   **Parse Failure:** If `grep`/regex fails to parse complex code, fall back to "Conservative Mode" (assume any change to a file *might* be Major and ask human).
*   **False Alarm:** User creates a breaking change in an *internal* file that looks exported.
    *   *Fix:* User adds `// @internal` comment, Agent learns to ignore it.

## Implementation Plan (Prototype)
1.  **Initialize:** `sentinel init` scans the codebase and builds the baseline API Graph in Memory.
2.  **Watch:** `sentinel watch` runs in background, checking `git diff` every minute.
3.  **Report:** `sentinel status` shows "Current Detected Impact: MINOR".
