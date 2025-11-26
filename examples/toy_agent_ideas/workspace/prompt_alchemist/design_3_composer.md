# Design 3: The Dynamic Composer

## Purpose
To move beyond static prompts entirely. This agent treats "The Prompt" as a dynamic artifact assembled at runtime based on the specific user query. It is a "Just-In-Time" Prompt Compiler.

## Loop Structure
1. **Intercept**: Receive user task (e.g., "Write a SQL query for users over 30").
2. **Retrieve**: Search Memory Graph and Filesystem for:
   - *Relevant Schema* (Table definitions)
   - *Style Guidelines* (SQL formatting rules)
   - *Similar Past Examples* (Few-shot history)
3. **Compose**: Assemble a cohesive system prompt from these fragments.
   - `[Role] + [Context] + [Few-Shot] + [Task]`
4. **Execute**: Send to LLM.
5. **Feedback**: If the user corrects the output ("No, use Postgres syntax"), store this correction as a new "Constraint Node" in Memory for future SQL queries.

## Tool Usage
- **memory**: The core engine. Stores fragments, constraints, and success rates.
- **grep/filesystem**: Indexing local project files to find context (e.g., schema.sql).
- **fetch**: Running the actual query.

## Memory Architecture
- **Context Graph**:
  - `TaskType(name="SQL Generation")`
  - `Constraint(text="Use snake_case")` linked to `TaskType`.
  - `Example(input="...", output="...")` linked to `TaskType`.
- **Retrieval Strategy**: Vector search on nodes to find relevant constraints.

## Failure Modes
- **Context Window Overflow**: Retrieving too much info. Needs a "Token Budget" allocator.
- **Hallucinated Constraints**: Applying Python rules to SQL tasks if retrieval is fuzzy.
- **Latency**: Composition adds time before the first token.

## Human Touchpoints
- **Implicit**: The agent learns from user corrections (Chat logs).
- **Explicit**: User can "pin" certain constraints ("Always use this table name").
