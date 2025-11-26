# Design 1: The IBIS Mapper (Passive)

## Purpose
To automatically convert unstructured local notes (Markdown) into a structured Argument Map (IBIS format: Issues, Positions, Arguments) to help the user see the logical structure of their own thoughts and identify unsupported claims.

## Core Tools
- **filesystem**: To scan and read user notes.
- **memory**: To store the extracted IBIS graph.
- **grep**: To find specific keywords indicating argumentation (e.g., "therefore", "however", "because").

## Loop Structure
1. **Scan**: Periodically scan the `notes/` directory for modified files.
2. **Extract**: Use LLM logic (simulated) to parse text into IBIS nodes:
   - *Issue*: What is the problem?
   - *Position*: What is the proposed solution/stance?
   - *Argument*: Why is this position good (Pro) or bad (Con)?
3. **Graph**: Store these nodes in the `memory` graph, linking them to the source file.
4. **Report**: Generate a `dashboard.md` listing:
   - "Orphaned Positions" (no parent Issue)
   - "Unsupported Claims" (Positions with no Pros)
   - "Unchallenged Claims" (Positions with no Cons)

## Memory Architecture
- **Entities**: `Issue`, `Position`, `Argument`, `SourceFile`
- **Relations**: `supports`, `opposes`, `responds_to`, `extracted_from`

## Failure Modes
- **Misinterpretation**: The agent might misclassify a casual comment as a formal "Position".
- **Noise**: The graph becomes cluttered with trivialities.
- **Recovery**: User can tag files with `#ignore` or explicitly structure critical notes to guide the parser.

## Human Touchpoints
- **Passive Observation**: The agent does not modify source files.
- **Reporting**: The user views the `dashboard.md` to see the analysis.
