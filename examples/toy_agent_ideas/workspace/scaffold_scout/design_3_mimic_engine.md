# Design 3: The Mimic Engine (Hybrid/Just-In-Time)

## Purpose
To eliminate the need for maintaining static templates by treating the *entire codebase* as a living library of templates. The agent performs "Parametric Copy-Paste" on demand.

## Loop Structure
1. **Intent Recognition:** 
   - User runs a command: `agent mimic src/components/Button src/components/Dropdown`.
   - OR Agent watches for `mkdir src/components/Dropdown` and offers help.
2. **Source Analysis:**
   - Reads the "Source" directory (`Button`).
   - Identifies the "Dominant Noun" (e.g., "Button") based on filename and frequency.
3. **Parametric Transformation:**
   - Copies files to the destination.
   - Renames files: `Button.tsx` -> `Dropdown.tsx`.
   - Replaces content: `class Button` -> `class Dropdown`, `const handleButtonClick` -> `const handleDropdownClick`.
   - intelligently handles casing (camelCase, PascalCase, SNAKE_CASE).
4. **Post-Processing:**
   - Runs imports cleanups.
   - Updates `index.ts` exports if necessary.

## Tool Usage
- **shell:** `cp -r`, `sed` (advanced regex for case-preserving replace).
- **grep:** Finding all references to the Dominant Noun in the source.
- **memory:** Remembers "Mimicry Pairs" (e.g., "Dropdown was mimicked from Button").

## Memory Architecture
- **Nodes:** `Prototype` (High quality, often-copied components).
- **Edges:** `DERIVED_FROM`.
- **Insight:** Over time, the graph identifies the "Best" Button component to copy from (the one with the most successful descendants).

## Failure Modes
- **Context/Logic Leak:** Copies specific logic (e.g., `if (props.isSubmit)`) that doesn't apply to the new component.
- **Naming Collisions:** Accidental replacement of generic terms.
- **Mitigation:** The agent outputs a diff for review before finalizing, or marks specific blocks as `// TODO: Logic specific to Source`.

## Human Touchpoints
- **Initiation:** User triggers the mimic command.
- **Review:** User reviews the created files.
