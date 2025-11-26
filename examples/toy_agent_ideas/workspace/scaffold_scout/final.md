# Agent: The Scaffold Scout

## Purpose
A "Just-in-Time" architectural assistant that eliminates boilerplate fatigue by treating your *existing codebase* as a living library of templates. Instead of forcing you to maintain static generator scripts (which rot), it performs **"Parametric Mimicry"**—intelligently cloning and mutating existing patterns to fit new contexts on the fly.

## Core Loop
1. **Trigger (Shell/Filesystem):**
   - **Active:** User runs `agent mimic <SourcePath> <NewName>`.
   - **Passive:** Agent watches for `mkdir` events (e.g., `mkdir src/features/Refunds`) and scans the sibling directories. If they all share a structure (Model/View/Controller), it asks: *"Want me to scaffold 'Refunds' like 'Payments'?"*

2. **Structural Analysis (Grep + Filesystem):**
   - **Source Reading:** Ingests the source directory (e.g., `src/features/Payments`).
   - **Token Identification:** Identifies the "Dominant Nouns" (e.g., "Payment", "payments", "PAYMENT_ID") by frequency and filename matching.
   - **Logic Stripping:** Uses regex/heuristics to identify specific logic bodies (e.g., inside `function processPayment() { ... }`) vs structural boilerplate. It creates a "Skeleton" in memory.

3. **Parametric Transformation (Text-Editor/Shell):**
   - **Mutation:** Applies the transformation to the Skeleton:
     - `Payment` -> `Refund` (PascalCase)
     - `payment` -> `refund` (camelCase)
     - `PAYMENT` -> `REFUND` (SCREAMING_SNAKE_CASE)
   - **Generation:** Writes the new files to the destination.

4. **Genealogy Tracking (Memory):**
   - Records the operation in the Knowledge Graph:
     - `Entity(Refunds) --[MIMICKED_FROM]--> Entity(Payments)`
   - **Evolutionary Insight:** Over time, the agent identifies "High-Fitness Prototypes" (components that are copied often and rarely modified immediately after). It recommends these as "Golden Standards".

## Tool Usage
- **filesystem:** Watching directories, reading source files.
- **shell:** Executing `cp`, `sed`, or running linters after generation.
- **grep:** Finding all references to the Dominant Noun to ensure clean replacement.
- **memory:** Storing the "Cloning Graph" to track lineage and identify the best prototypes.

## Key Insight: "Codebase as Template Library"
Static templates (Yeoman, Hygen) drift from the actual code standards. By the time you use them, they are outdated. **The Scaffold Scout** ensures your new code always matches your *current* best practices because it copies from the code you wrote *yesterday*, not the template you wrote *last year*.

## Persistence Strategy
- **Memory Graph:** Stores the lineage of components.
- **Filesystem:** Purely functional (creates new files). No hidden config needed.

## Autonomy Level
- **Mixed:** 
  - **High Autonomy** for monitoring and internal graph updates.
  - **Human-in-the-Loop** for the actual file generation (User confirms the "Source" to copy from).

## Failure Modes
- **Logic Leak:** Accidentally copying specific business logic (e.g., a specific validation rule) that looks like boilerplate.
  - *Recovery:* The user deletes the specific line. The agent marks that line pattern as "Specific" in memory to avoid copying it next time.
- **Bad Source:** User copies from a buggy component.
  - *Recovery:* When the bug is fixed in the Source, the agent can use the Memory Graph to warn: *"You copied 'Refunds' from 'Payments', which had Bug X. Check 'Refunds' too."*

