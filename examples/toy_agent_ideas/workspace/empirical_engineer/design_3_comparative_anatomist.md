# Design 3: The Comparative Anatomist

**Theme:** Knowledge-driven, analogical reasoning using external data.

## Purpose
To solve problems by finding "structural rhymes" in the wider world. It assumes most bugs have been seen before, just in different contexts.

## Core Loop: The Analogy Cycle
1.  **Abstraction:** Analyze the local error/code. Extract the *structure* of the problem (e.g., "Resource leak in async loop with retries").
2.  **Search:** Use `web` to find similar patterns, not just error text. Look for architectural patterns.
3.  **Mapping:** Retrieve "Case Studies" from `web` results and existing `memory`.
4.  **Transfer:** Adapt the solution from the Case Study to the local context.
5.  **Verification:** Check if the mapped solution applies (using `grep` to see if the vulnerable pattern exists locally).

## Tool Usage
*   **web:** Extensive use of `brave_web_search` and `brave_news_search` to find similar bugs, post-mortems, and design patterns.
*   **memory:** Stores a "Pattern Library". Abstract generalized bug classes.
*   **grep:** Scans local code to match external patterns.

## Memory Architecture
*   **Entities:** `Pattern`, `CaseStudy`, `FixStrategy`.
*   **Relations:**
    *   `CurrentBug IS_ANALOGOUS_TO CaseStudy`
    *   `Pattern SUGGESTS FixStrategy`

## Failure Modes
*   **False Analogy:** Applying a fix for a similar-looking but fundamentally different problem (e.g., fixing a symptom, not the cause).
*   **Hallucination:** Inventing a connection between unrelated technologies.

## Human Touchpoints
*   **Review:** Human must validate the analogy ("Is this really like the 2014 OpenSSL bug?").
