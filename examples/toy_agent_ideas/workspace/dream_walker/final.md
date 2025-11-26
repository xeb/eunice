# The Dream Walker: A Bimodal Subconscious Processor

## Abstract
**The Dream Walker** is an autonomous background agent that mimics the human brain's "Default Mode Network" and sleep cycles to solve complex problems while the user is away. It employs a **Bimodal Incubation Strategy**: using evolutionary computation (NREM) for rigid logical errors, and associative graph walks (REM) for conceptual creative blocks.

## Core Mandates
1.  **Do Not Disturb**: Runs *only* when the system is idle or the user is "asleep" (scheduled).
2.  **Divergent Exploration**: Prioritizes novel/weird connections over immediate efficiency during the Dream Phase.
3.  **Consolidated Output**: Wakes up with a clean, summarized list of proposals, discarding the thousands of failed "dreams".

## Architecture Components

### 1. The Bedside Table (Input/Output)
*   **Inbox**: `dream_inbox.md` - User defines the problem type:
    *   Type A: **Logical/Bug** (e.g., "This test fails intermittently").
    *   Type B: **Conceptual/Block** (e.g., "I need a better UI metaphor for this graph").
*   **Journal**: `dream_journal.md` - The morning report containing "Prophetic Dreams" (Solutions) and "Nightmares" (Warnings).

### 2. The Hippocampus (Memory Graph)
*   **Short-Term**: Records the day's file edits and shell errors.
*   **Long-Term**: A persistent graph of Concepts, Patterns, and Analogies fetched from the Web.
*   **Entities**: `CodeBlock`, `Error`, `Concept`, `Metaphor`, `Mutation`.
*   **Relations**: `relates_to`, `fixes`, `is_analogous_to`, `evolved_from`.

### 3. The Dream Cycles (Execution Loop)

#### Phase 1: Hypnagogia (Context Loading)
*   Agent reads `dream_inbox.md`.
*   **For Type A (Bug)**: It creates a sandboxed copy of the codebase. It runs the failing test to establish a baseline.
*   **For Type B (Concept)**: It builds a local context graph of the relevant files and searches the Web for "Standard Definitions" to ground its understanding.

#### Phase 2: NREM Sleep (Deep Work / Optimization)
*   *Target: Logical/Bug*
*   **Strategy**: **Evolutionary Mutation**.
    *   The agent uses `text-editor` to apply AST-based mutations to the suspicious code.
    *   It runs `shell` test commands.
    *   If a test passes, it saves the patch. If it fails, it records the error (to learn "what breaks it").
    *   It repeats this 100-1000 times, optimizing for "Test Pass" + "Minimal Change".

#### Phase 3: REM Sleep (Associative Dreaming)
*   *Target: Conceptual/Block*
*   **Strategy**: **Random Graph Walk & Bisociation**.
    *   The agent picks the "Problem Node" in Memory.
    *   It performs a "Random Walk" to a distant node (e.g., "Ant Colony Optimization" or " Bauhaus Architecture").
    *   It forces a connection: "How is [My UI Problem] like [Bauhaus]?"
    *   It generates a "Concept Paper" (Markdown) exploring this analogy.
    *   It searches the Web to see if this connection actually exists in real-world engineering (validating the dream).

#### Phase 4: Waking (Consolidation)
*   The agent filters the results.
    *   **NREM Results**: Runs the best patch one last time. If it passes, writes `fix_candidate.patch`.
    *   **REM Results**: Ranks the metaphors by "Novelty" and "Plausibility". Writes the top 3 to `dream_journal.md`.
*   Updates the Memory Graph: "Consolidates" the successful paths, "Prunes" the dead ends.

## Tool Usage
*   **filesystem**: Sandboxed code manipulation, reading inbox, writing journal.
*   **shell**: Running tests, git operations.
*   **memory**: Storing the evolutionary tree (NREM) and the concept graph (REM).
*   **web**: Fetching random concepts for REM sleep, validating analogies.
*   **text-editor**: Applying mutations to code.

## Failure Modes & Recovery
*   **Fever Dream (Hallucination)**: REM cycle produces nonsense.
    *   *Recovery*: The "Waking" phase has a "Reality Check" filter. If a metaphor has 0 web search hits, it is discarded.
*   **Sleepwalking (Destructive Action)**: Agent deletes files.
    *   *Recovery*: Strictly limited to `/tmp/dream_sandbox`. Main codebase is read-only during sleep.
*   **Insomnia (No Solution)**:
    *   *Recovery*: Agent writes "I tossed and turned but found nothing. Try rephrasing the problem."

## Human Handoff
*   The user is the "Dream Interpreter". They wake up, check the journal, and decide if the "Patch" is valid or if the "Metaphor" is inspiring.
