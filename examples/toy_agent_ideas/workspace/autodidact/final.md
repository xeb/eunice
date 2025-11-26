# Agent: The Autodidact (aka "The Knowledge Package Manager")

## Purpose
Self-education is often chaotic. We hoard tutorials, skip prerequisites, and get stuck in "Tutorial Hell." **The Autodidact** solves this by treating Knowledge Acquisition as a **Dependency Resolution** problem. It builds a "Concept Graph" for your target skill, resolves the "Prerequisites," and scaffolds a **Linear Learning Path** on your filesystem. You only see the modules you are ready for.

## Core Loop
1. **Goal Definition:** User defines a goal in `GOALS.md` (e.g., "Learn Bevy Engine").
2. **Dependency Mapping (The "Scout" Phase):**
   - Agent searches Web/Video for "Bevy Engine prerequisites", "Rust Game Dev roadmap".
   - Agent uses `memory_create_entities` to build nodes: `[Rust] -> [ECS Architecture] -> [Bevy]`.
   - Agent identifies the **"Learning Frontier"** (the set of nodes with no un-mastered dependencies).
3. **Scaffolding (The "Teacher" Phase):**
   - Agent creates directories for the Frontier nodes:
     - `01_Rust_Basics/`
     - `02_ECS_Concepts/`
   - Agent populates these folders with a `README.md` containing:
     - **Context:** Why you are learning this.
     - **Resources:** Top 3 links/videos (filtered by views/date).
     - **Exit Criteria:** What you must be able to do to pass.
4. **Verification (The "Examiner" Phase):**
   - User studies and marks `status: complete` in the `README.md`.
   - (Optional) User runs `autodidact quiz`, agent generates questions in `QUIZ.md`.
5. **Progression:**
   - Agent detects the status change.
   - Agent updates Memory Graph (marks node as Mastered).
   - Agent recalculates the Frontier.
   - Agent "Unlocks" (creates) the next folder: `03_Bevy_Sprite_Rendering/`.

## Tool Usage
- **memory:** The "Brain." Stores the DAG of concepts (Nodes) and dependencies (Edges). Stores "Mastery" state.
- **web:** The "Eyes." Searches for syllabi, roadmaps, and tutorials.
- **filesystem:** The "UI." The user interacts *only* with files and folders. The agent manages the structure.
- **fetch:** Used to read tutorial content to generate quizzes or summaries.

## Persistence Strategy
- **Hybrid:**
  - **Memory Graph:** Holds the "Truth" (Concepts, Relations, Mastery).
  - **Filesystem:** Holds the "Artifacts" (Notes, Code, PDFs) and serves as the User Interface.

## Key Insight
**"Just-in-Time Curriculum"**.
Most learning plans fail because they are static lists. The Autodidact is dynamic. If you struggle with "Lifetime Annotations" in Rust, the agent can dynamically insert a sub-module "Stack vs Heap Memory" *before* letting you proceed, effectively patching the curriculum at runtime based on your performance.

## Failure Modes & Recovery
1. **Bad Graph:** Agent infers wrong dependencies (e.g., thinks "Java" is needed for "JavaScript").
   - **Recovery:** User can edit a `graph_overrides.md` file to manually force/break links.
2. **Resource 404:** Linked tutorials die.
   - **Recovery:** Agent periodically checks links (HEAD request) and replaces dead ones.
3. **Stalemate:** User refuses to do the work.
   - **Recovery:** Agent uses "Nudges" (updating the README with "You haven't touched this in 3 days. Need simpler resources?").

## Future Expansion
- **Project Scaffolding:** Instead of just folders, it runs `cargo new` or `npm init` to set up the actual coding environment for that module.
- **Spaced Repetition:** It re-surfaces old concepts in current quizzes to prevent decay.
