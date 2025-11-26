# The Coherence Engine: A Semantic Consistency Sentinel

## Purpose
The Coherence Engine is an autonomous agent designed to prevent "Semantic Drift" in long-lived software projects. It acts as a "Truth Maintenance System," continuously verifying that the *Intent* (expressed in documentation and comments) matches the *Reality* (implemented in code and tests).

## Key Insight
Most tools treat code and comments as text. The Coherence Engine treats them as **Claims** in a **Knowledge Graph**. By reifying "The code claims to return X" and "The doc claims to return Y" as graph nodes, the agent can algorithmically detect contradictions (drift) and resolve them via a "Socratic" interface that respects the developer's time.

## Core Architecture

### 1. The Knowledge Graph (Memory)
The agent maintains a persistent `memory` graph:
*   **Entities**: `Artifact` (File), `Unit` (Function/Class), `Concept` (e.g., "User Authentication"), `Claim` (Fact assertion).
*   **Relations**:
    *   `Code --IMPLEMENTS--> Concept`
    *   `Doc --DESCRIBES--> Concept`
    *   `Test --VERIFIES--> Concept`
    *   `Claim A --CONTRADICTS--> Claim B`

### 2. The Loop
1.  **Ingestion (Drift Detection)**:
    *   **Trigger**: File save or commit.
    *   **Action**: `filesystem` reads changed files. `grep` + NLP extracts "Claims" (e.g., parsing Javadoc `@throws` or Python type hints).
    *   **Comparison**: Query `memory` for conflicting claims.
        *   *Drift Example*: Doc says "Returns null on failure", Code says "Throws Exception".

2.  **Hypothesis Formulation**:
    *   The agent doesn't assume the code is right. It formulates a hypothesis: "Documentation for `auth()` is outdated."
    *   It uses `web_brave_search` to check external references (e.g., "Does API v3 still support XML?").

3.  **Interaction (The Review Protocol)**:
    *   **No Code Pollution**: It does NOT insert comments into source code.
    *   **The Review File**: It creates/updates `coherence_reviews/pending.md`.
    *   **Format**:
        ```markdown
        ### Discrepancy #12: `auth()` Return Type
        - **Doc**: Claims to return `null`.
        - **Code**: Throws `AuthError`.
        - **Suggested Fix**: Update docstring to `@throws AuthError`.
        - [ ] Apply Fix
        - [ ] Ignore (False Positive)
        ```

4.  **Actuation**:
    *   The agent monitors `coherence_reviews/pending.md`.
    *   When the user checks `[x] Apply Fix`, the agent uses `text-editor` to patch the original source file.

## Tool Utilization
| Tool | Purpose |
|------|---------|
| **memory** | Storing the "Truth Graph" of claims and their verification status. |
| **filesystem** | Reading source code and managing the Markdown review interface. |
| **grep** | Semantic search for finding evidence of claims across the codebase. |
| **text-editor** | Precisely patching documentation or code after user approval. |
| **web** | Validating external URLs and library version compatibility in docs. |

## Failure Modes & Recovery
*   **Parsing Errors**: Agent misinterprets a comment.
    *   *Recovery*: User selects `[x] Ignore` in the review file. Agent adds a `FALSE_POSITIVE` edge in memory to prevent recurring alerts.
*   **Graph Desync**: Files change while agent is offline.
    *   *Recovery*: On startup, agent runs a `git diff` against its last known state and re-indexes changed files.

## Autonomy Level
**Checkpoint-Based Autonomy**. The agent runs autonomously to detect issues and prepare fixes, but requires a discrete human signal (checking a box) to modify the actual codebase. This builds trust.
