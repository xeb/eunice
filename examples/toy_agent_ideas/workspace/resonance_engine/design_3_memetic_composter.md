# Design 3: The Memetic Composter (Experimental)

## Purpose
A creative engine that treats the filesystem as a "compost heap" of ideas. It doesn't just link files; it *shreds* them to create new composite documents. It aims to generate **Novelty** by forcing collisions between unconnected text fragments.

## Core Toolset
*   **filesystem:** Read text files.
*   **grep/llm (simulated):** Extract interesting paragraphs/sentences.
*   **memory:** Stores "Memes" (independent ideas extracted from files).
*   **web:** Validates if the composite idea is actually new or a known concept.

## Loop Structure
1.  **Decomposition:**
    *   Randomly sample 2-3 "cold" text files.
    *   Extract specific paragraphs (using grep for key phrases like "concept is", "implies that").
    *   Store as disconnected "Memes" in Memory.
2.  **Bisociation (The "Collision"):**
    *   Select two Memes from different domains (e.g., "Biology: Mycelium networks" AND "Tech: Kubernetes clusters").
    *   Prompt LLM (or heuristic template): "How is [Meme A] like [Meme B]?"
3.  **Synthesis:**
    *   Generate a new Markdown file: `Drafts/Synthetic_Insights/Mycelium_Kubernetes_Protocol.md`.
    *   Content: The two source excerpts + the synthesized connection.

## Memory Architecture
*   **Entities:** `Meme` (text fragment), `SourceFile`, `SyntheticConcept`.
*   **Relations:**
    *   `Meme -> extracted_from -> SourceFile`
    *   `SyntheticConcept -> derived_from -> [Meme A, Meme B]`

## Failure Modes
*   **Nonsense Generation:** The connection is trivial or absurd. (Fix: User voting on quality).
*   **Context Loss:** Stripping paragraphs removes necessary nuance.

## Human Touchpoints
*   **"Garden Walk":** User reviews the "Synthetic Insights" folder.
*   **Pruning:** User deletes bad syntheses (training the selection logic).

## Pros/Cons
*   **Pros:** Generates actual *new* content. High potential for "Aha!" moments.
*   **Cons:** High noise-to-signal ratio. Very experimental.
