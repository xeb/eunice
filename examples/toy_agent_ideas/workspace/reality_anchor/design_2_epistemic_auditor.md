# Design 2: The Epistemic Auditor

## Purpose
An aggressive, semantic agent that treats documentation as a set of "Claims" that must be continuously verified against the world. It targets "Misinformation" and "Drift" in knowledge bases.

## Loop Structure
1.  **Claim Extraction:**
    *   Read a file. Use NLP heuristics (or `grep` for "is a", "always", "never", "best") to identify *assertions*.
    *   *Example:* "LILO is the default bootloader for Linux."
2.  **Litigation Phase:**
    *   Formulate a boolean search query: "Is LILO the default bootloader for Linux 2025?".
    *   Use `web_brave_web_search` to fetch snippets.
    *   Analyze snippets for negation ("deprecated", "replaced by GRUB", "no longer").
3.  **Adjudication:**
    *   If evidence suggests the claim is false:
        *   Create a `Dispute` node in `memory`.
        *   Draft a "Correction Note".
4.  **Intervention:**
    *   Use `text-editor` to *rewrite* the sentence or append a correction immediately after it.
    *   *Example:* "LILO is the default bootloader... [**CORRECTION 2025:** Replaced by GRUB2 in most distros]."

## Tool Usage
*   **memory:** Stores the "Knowledge Graph" of the user vs the "World Graph".
*   **web_brave:** The source of truth.
*   **text-editor:** In-line surgical edits.

## Memory Architecture
*   **Nodes:** `Claim`, `Evidence`, `Source`.
*   **Edges:** `Claim CONTRADICTS Evidence`, `File ASSERT_CLAIM Claim`.
*   **State:** `VerificationStatus` (Verified, Disputed, Unknown).

## Failure Modes
*   **Nuance Loss:** "React is fast" (context: 2015) vs "React is fast" (context: 2025 - maybe Svelte is faster?). The agent might pedantically correct subjective/contextual claims.
*   **Vandalism:** If the web search returns bad info (SEO spam), the agent might "correct" valid notes with spam.

## Human Touchpoints
*   **Review Queue:** The agent creates a "Pull Request" file (e.g., `corrections.diff`) for the user to approve before applying changes to the actual notes.
