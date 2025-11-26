# Design 1: The Prompt Stabilizer

## Purpose
To act as a "Continuous Integration" system for Large Language Model prompts. It ensures that changes to system prompts do not regress performance on critical test cases. It treats prompts as strict code artifacts with versioning and passing/failing states.

## Loop Structure
1. **Monitor**: Watch `prompts/` directory for file changes (hash-based).
2. **Trigger**: On change, identify associated test cases in `tests/<prompt_name>.json`.
3. **Execute**: Run the new prompt against all test inputs using `fetch` (to LLM API).
4. **Evaluate**: Compare outputs against "Gold Standard" answers using:
   - Exact Match (for structured output)
   - Semantic Similarity (cosine distance)
   - LLM-Judge (asking another model "Does A match B?")
5. **Report**: Write a markdown report `reports/<timestamp>_regression.md` and update the Memory Graph with the new version's health score.

## Tool Usage
- **filesystem**: Read prompt files, read test cases, write reports.
- **fetch**: Call external LLM APIs (OpenAI/Anthropic) to generate outputs for testing.
- **memory**: Store the history of Prompt Versions and their scores.
  - Entities: `PromptVersion`, `TestCase`, `RunResult`.
  - Relations: `PromptVersion TESTED_BY RunResult`.
- **shell**: Git operations (optional) to tag working versions.

## Memory Architecture
- **Nodes**:
  - `Prompt(name="CustomerService")`
  - `Version(hash="a1b2", content="...")`
  - `TestSet(name="RefundScenarios")`
- **Edges**:
  - `Version -> PASSED -> TestSet`
  - `Version -> REGRESSED_ON -> TestCase`

## Failure Modes
- **API Flakiness**: LLMs are non-deterministic. The agent must run tests N times (e.g., n=3) to ensure statistical significance.
- **Cost Runaway**: Infinite loops of testing could be expensive. Limit to 1 run per file save.
- **Judge Bias**: The "Judge" LLM might be biased. Use simple string matching where possible.

## Human Touchpoints
- **Test Creation**: Humans must provide the initial "Golden Set" of input/output pairs.
- **Review**: Humans read the generated reports to decide if a regression is acceptable (e.g., style change).
