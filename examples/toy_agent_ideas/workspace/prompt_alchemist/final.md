# Agent: The Prompt Alchemist

## Purpose
To apply **Software Engineering rigor** to **Prompt Engineering**. The Prompt Alchemist is a background agent that treats prompts as compilable, testable code. It automatically runs regression tests when prompts change and, on demand, uses evolutionary algorithms to "refactor" prompts for better performance or lower token cost.

## Core Loop (The "Prompt-Ops" Cycle)

1.  **Watch**: The agent monitors the `prompts/` directory for changes to `.txt` or `.md` prompt files.
2.  **Test (The Stabilizer)**:
    *   When `system_prompt_v1.md` is modified, the agent looks for `tests/system_prompt_v1.json`.
    *   It executes the new prompt against the test cases using an LLM API.
    *   It compares results to "Gold Standard" outputs (Exact Match, Regex, or LLM-Judge).
    *   **Outcome**: A "Pass/Fail" report is generated. If failed, it alerts the user (e.g., via a failed build log or notification file).
3.  **Optimize (The Evolver)**:
    *   *Trigger*: User runs a command or sets a flag `@optimize goal="reduce tokens"`.
    *   The agent enters an evolutionary loop:
        1.  **Mutate**: Create 5 variants of the prompt (shorten, rephrase, add examples).
        2.  **Eval**: Run all 5 against the test set.
        3.  **Select**: Pick the best performer.
        4.  **Recurse**: Repeat for N generations.
    *   **Outcome**: A new file `system_prompt_v1_optimized.md` is created with a diff report showing improvements.
4.  **Persist**: The "Winning" prompt's metadata (Elo score, average tokens, latency) is stored in the **Memory Graph**.

## Tool Usage

### 1. Filesystem (The Workspace)
*   **Structure**:
    *   `prompts/`: Source of truth for system prompts.
    *   `tests/`: JSON files containing `{input, expected_output, criteria}` pairs.
    *   `reports/`: Markdown files summarizing test runs and optimization journeys.
*   **Action**: `filesystem_read_file` to ingest prompts/tests, `filesystem_write_file` to save optimized versions.

### 2. Fetch (The LLM Interface)
*   **Action**: `fetch_fetch` to call LLM APIs (via local proxy or direct HTTP) to generate completions for tests.
*   **Role**: Acts as both the "Subject" (running the prompt) and the "Judge" (grading the output).

### 3. Memory (The Knowledge Base)
*   **Action**: `memory_create_entities` to track Prompt Lineage.
*   **Graph Schema**:
    *   **Entities**: `Prompt`, `Version`, `TestResult`, `OptimizationGoal`.
    *   **Relations**:
        *   `Prompt HAS_VERSION Version`
        *   `Version ACHIEVED_SCORE TestResult`
        *   `Version DERIVED_FROM Version` (Genealogy)
*   **Value**: Allows the user to query: *"Which version of the prompt had the best accuracy on the 'refund' test case?"*

### 4. Grep (The Discovery Tool)
*   **Action**: `grep_search` to find where prompts are *used* in the codebase (e.g., in Python/JS files) to ensure the file on disk matches the one in code.

## Failure Modes & Recovery

1.  **Flaky Evals**: LLMs are non-deterministic.
    *   *Recovery*: The agent runs each test case 3 times and takes the majority vote or average score.
2.  **Infinite Optimization Loops**: The agent keeps tweaking without gain.
    *   *Recovery*: Strict "Generation Limit" (e.g., max 5 gens) and "Early Stopping" (stop if improvement < 1%).
3.  **API Cost Spikes**:
    *   *Recovery*: A daily budget tracker in Memory. If budget exceeded, the agent pauses all operations.

## Human Touchpoints

*   **The "Golden Set"**: The agent cannot invent truth. The human *must* provide the initial set of `{input, expected_output}` pairs.
*   **Judge Criteria**: The human defines *how* to grade (e.g., "Must be valid JSON" or "Must be polite").
*   **Merge Decision**: The agent proposes `_optimized.md` files, but the human must manually rename/overwrite the original to "accept" the change.

## Key Insight
Treating Prompt Engineering not as "Art" (writing prose) but as **Parameter Search** (Machine Learning). By automating the "Eval Loop," we unlock the ability to use Brute Force (Evolution) to find prompt patterns that humans would never intuit, while ensuring safety through Regression Testing.
