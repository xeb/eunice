# Design 2: The Evidence Miner (Innovative)

## Purpose
A proactive agent that builds a **"Proof-of-Work" Knowledge Graph**. It ignores commit messages and looks at the *code* you write. If you import `pytorch`, it infers you know PyTorch. It links every skill on your resume to specific lines of code you wrote, creating an unshakeable "Verified Resume."

## Loop Structure
1. **Deep Scan:** Periodically runs `grep` / `ripgrep` across all projects to find imports, package.json dependencies, and specific syntax patterns.
2. **Inference:** Maps found patterns to a "Skill Ontology" (e.g., `import torch` -> `PyTorch` -> `Machine Learning`).
3. **Graphing:** Updates a Memory Graph: `(User) -[HAS_SKILL]-> (PyTorch) -[EVIDENCED_BY]-> (File: model.py)`.
4. **Portfolio Gen:** Generates a static HTML portfolio where clicking "PyTorch" reveals the exact snippets of code where you used it.

## Tool Usage
- **grep:** Scanning for imports and library usage (`grep -r "import torch" .`).
- **memory:** Storing the `Skill -> Evidence` graph and `Project -> Tech Stack` relations.
- **filesystem:** Reading code files to generate snippets.
- **web:** (Optional) Fetching icons/descriptions for identified skills.

## Memory Architecture
- **Knowledge Graph:**
  - Nodes: `Skill`, `Project`, `File`, `Commit`.
  - Edges: `USED_IN`, `DEPENDS_ON`, `AUTHORED_BY`.
- **Persistence:** High. The graph grows over years, tracking skill evolution.

## Failure Modes
- **False Positives:** Copy-pasting a library import but not using it.
- **Context Loss:** Knowing *syntax* doesn't mean knowing *architecture*.
- **Recovery:** Human can manually "prune" incorrect skill attributions in the graph.

## Human Touchpoints
- **Curating:** User selects which "Evidence" is public vs private.
- **Narrative:** User adds "Context" notes to the raw evidence.

## Pros/Cons
- **Pros:** Objective truth; handles "messy commits" by looking at code; visualizes growth.
- **Cons:** Computational overhead of scanning; might over-index on languages vs concepts.
