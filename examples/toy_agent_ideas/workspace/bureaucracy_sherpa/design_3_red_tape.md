# Design 3: The Bureaucracy Sherpa (The Red Tape Cutter)

## Purpose
A project management agent for complex administrative life tasks (e.g., "Get a Mortgage", "Immigrate", "Start a Business"). It acts as a "Requirements Compiler" that maps external regulations to internal user assets, identifying gaps and generating the necessary paperwork.

## Core Loop
1. **Goal Definition:** User creates a folder `projects/japanese_visa/` and a `goal.md` file: "I want a Highly Skilled Professional Visa for Japan."
2. **Requirement Compilation:**
   - Agent searches the web for "Japan HSP visa requirements official".
   - It parses the results into a **Requirement Graph** in `memory`.
   - Nodes: `Requirement(Points Calculation Table)`, `Requirement(Proof of Income)`.
   - Edges: `Proof of Income --requires--> Tax Return`.
3. **Asset Mapping:**
   - Agent scans the user's `archive/` for files matching the requirements (using fuzzy name matching and grep).
   - It links found files to the Graph Nodes: `Tax Return --satisfied_by--> archive/2024_tax_return.pdf`.
4. **Gap Analysis:**
   - Agent generates a `status_report.md` in the project folder.
   - Lists: "Ready to Submit" vs "Missing Items".
5. **Action Generation:**
   - For missing items that are forms, it attempts to download the PDF.
   - For missing items that are actions (e.g., "Notarize"), it adds a TODO.

## Tool Usage
- **web**: Finding the official requirements and forms.
- **memory**: Maintaining the graph of (Requirement <-> Asset).
- **filesystem**: indexing user files and writing reports.
- **fetch**: Downloading forms.

## Memory Architecture
- **Process Graph:** Models the *process* itself.
- **Asset Index:** Models the user's files.
- **Mapping:** The core value is the dynamic linking of the two.

## Failure Modes
- **Semantic Mismatch:** Thinking "Bank Statement" satisfies "Proof of Income" when the requirement specifically says "Tax Certificate".
- **Regulatory Drift:** Rules change. The agent needs to re-verify the graph periodically.

## Human Touchpoints
- User defines the goal.
- User reviews the "Gap Analysis".
- User physically performs offline tasks (notarization).

## Pros/Cons
- **Pros:** Massive time saver, reduces cognitive load, turns vague anxiety into a concrete checklist.
- **Cons:** Complex to implement reliable "Requirement Parsing" from unstructured web text.
