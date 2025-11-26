# Design 2: The Prompt Evolver

## Purpose
To autonomously optimize prompts for higher performance. Instead of just testing, this agent *rewrites* the prompt using evolutionary algorithms (Genetic Programming) to maximize a specific metric (e.g., "conciseness", "accuracy", "JSON validity").

## Loop Structure
1. **Seed**: Start with a baseline prompt and a set of failing test cases.
2. **Mutate**: Generate N variants of the prompt using strategies:
   - *Paraphrasing* ("Rephrase this to be more formal")
   - *Few-Shot Injection* (Add a failed case as a negative example)
   - *Chain-of-Thought Insertion* (Add "Let's think step by step")
3. **Tournament**: Run all N variants against the test set.
4. **Select**: Keep the top K performers based on the fitness function (Score).
5. **Crossover**: Combine traits of top performers (e.g., take the persona from A and the constraints from B).
6. **Repeat**: Iterate until score plateaus or budget is exhausted.

## Tool Usage
- **fetch**: The workhorse. Used for both *generating* mutations (LLM call) and *evaluating* them (LLM call).
- **memory**: Track the "Genealogy" of prompts.
  - "Parent" -> "Child" relations.
  - Store "Fitness Scores" to visualize improvement over time.
- **filesystem**: Save the "Winner" to `prompts/optimized/<name>.txt`.

## Memory Architecture
- **Evolutionary Tree**:
  - `Generation(id=1)` -> `Variant(id=1a, score=0.8)`
  - `Variant(id=1a)` -> `MUTATED_TO` -> `Variant(id=2b)`
- **Insights**:
  - Store "Learned Principles" (e.g., "This model prefers bullet points over paragraphs").

## Failure Modes
- **Overfitting**: The prompt becomes excellent at the 10 test cases but fails on general inputs. (Mitigation: Hold-out validation set).
- **Prompt Drift**: The prompt becomes unreadable/bizarre to humans ("jailbreak style") but works for the model.
- **Cost**: High token usage. Needs strict budget caps (e.g., "Max  per optimization run").

## Human Touchpoints
- **Goal Definition**: User must define the "Fitness Function" (what does "good" mean?).
- **Final Approval**: User picks the best variant from the leaderboard to deploy.
