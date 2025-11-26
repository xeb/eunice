# Design 2: The Concept Grapher (Core Idea)

## Purpose
To treat knowledge acquisition as a **Dependency Resolution** problem. Just as you cannot compile code without dependencies, you cannot effectively learn "React" without "JavaScript". This agent builds a valid "Install Plan" for your brain.

## Loop Structure
1. **Ingest:** User submits a topic (e.g., "Quantum Computing").
2. **Mapping:** Agent iteratively searches for "Prerequisites of X" and "Concepts in X".
3. **Graphing:** Agent uses `memory_create_entities` and `memory_create_relations` to build a DAG (Directed Acyclic Graph).
   - Node: "Linear Algebra" -> Relation: "prerequisite_for" -> Node: "Quantum Gates".
4. **Projection:** Agent projects this graph onto the filesystem:
   - `01_Linear_Algebra/STATUS.md` (contains resources)
   - `02_Quantum_Gates/STATUS.md` (Locked/Hidden until 01 is marked DONE).
5. **Tracking:** Agent watches the filesystem. When user marks "Linear Algebra" as complete, it "unlocks" the next folder (populates it with content).

## Tool Usage
- **memory:** Stores the fine-grained Concept Graph (Nodes = Concepts, Edges = Dependencies).
- **web:** Searches for syllabi and roadmaps to infer dependencies.
- **filesystem:** Acts as the "User Interface" (Folders = Modules).

## Memory Architecture
- **Graph-First:** The source of truth is the Memory Graph. The filesystem is just a *view* of the current frontier.
- **Persisted State:** `ConceptNode { name: "Linear Algebra", status: "mastered", resources: [...] }`

## Failure Modes
- **Dependency Cycles:** A depends on B, B depends on A. (Recovery: Agent detects cycles and asks human to break the tie).
- **Granularity Mismatch:** Concepts are too broad ("Math") or too narrow ("Vector Addition"). (Recovery: Human merges/splits nodes).

## Human Touchpoints
- **Assertion of Knowledge:** "I already know Matrix Multiplication." (Agent marks node complete, cascades updates).
- **Goal Setting:** "I want to learn X."
