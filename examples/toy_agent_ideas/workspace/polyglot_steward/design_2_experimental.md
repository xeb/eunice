# Design 2: The Cultural Liaison (Experimental)

## Purpose
A semantic agent that ensures translations are not just linguistically correct but culturally appropriate. It uses the Web to validate phrases against real-world usage in target regions and uses Memory to build a "Cultural Graph" of sensitive terms.

## Core Loop
1.  **Monitor**: Watches the i18n files for new or changed keys.
2.  **Contextualize**: When a new term (e.g., "cool") appears, it uses `web_search` to check:
    *   "Is 'cool' offensive in [Language X]?"
    *   "Best translation for 'cool' in UI context for [Country Y]".
3.  **Validate**: Checks the term against its internal **Cultural Graph** (Memory).
    *   *Observation*: "Red is associated with death in culture X, but prosperity in culture Y."
    *   *Inference*: "Warning: This UI error message uses red iconography for a userbase in X."
4.  **Enrich**: Adds "Translator Notes" automatically to the file, citing web sources.

## Tool Usage
*   **web_brave_web_search**: To research slang, idioms, and cultural connotations.
*   **memory**: Stores entities (Languages, Regions, Phrases) and relations (is_offensive_in, implies_urgency_in).
*   **filesystem**: Reads language files to propose annotations.

## Memory Architecture
*   **Entity Graph**:
    *   `Entity("Phrase: Thumbs Up")` -> `Relation("is_offensive_in")` -> `Entity("Region: Middle East")`
    *   `Entity("Term: Zip Code")` -> `Relation("has_format")` -> `Entity("Format: Numeric")`
*   **Evolution**: The graph grows as it learns about more regions and UI patterns.

## Failure Modes
*   **Hallucination**: Might flag innocent terms as offensive based on spurious forum posts.
*   **Over-correction**: Might suggest hyper-formal language for a casual app.

## Human Touchpoints
*   **Dispute Resolution**: Human translator overrides the agent's warnings. The agent then adds an observation: "Human approved this usage."
