# Design 3: The Curriculum Cartographer

## Purpose
An "Explorer Agent" that helps you learn a *new* domain by mapping it first. Instead of just giving you a tutorial, it crawls the documentation of a target tool (e.g., "Kubernetes"), builds a dependency graph of concepts, and generates a personalized "Syllabus" that guides you from zero to hero.

## Loop Structure
1. **Target Selection:** User provides a URL or name: `skill-weaver map "Rust Language"`.
2. **Reconnaissance:** Agent uses `web_brave_web_search` and `fetch` to crawl the official docs/book.
3. **Graph Construction:** Extracts concepts (Structs, Traits, Lifetimes) and their dependencies (Lifetimes depend on Borrowing). Builds a DAG in `memory`.
4. **Syllabus Generation:** Topologically sorts the graph and generates a folder structure: `curriculum/01_borrowing/README.md`, `curriculum/02_lifetimes/README.md`.
5. **Interactive Guidance:** As user marks modules "Complete" (by deleting a `.lock` file or passing a test), the agent reveals the next module.

## Tool Usage
* **web_brave_summarizer:** To condense documentation pages into "Concept Cards".
* **memory_create_relations:** To build the `DependsOn` graph of concepts.
* **filesystem_create_directory:** To structure the curriculum on disk.

## Memory Architecture
* **Entities:** `Concept`, `Resource` (URL), `Module`.
* **Relations:** `Concept A --prerequisite_for--> Concept B`.
* **State:** `User --completed--> Module`.

## Failure Modes
* **Hallucination:** Inventing concepts or dependencies.
  * *Recovery:* User can manually edit the `syllabus.yaml` file to correct the order.
* **Overwhelm:** Generating too much content.
  * *Recovery:* "Pruning" mode where user selects only specific sub-topics.

## Human Touchpoints
* **Curriculum Review:** User approves the generated syllabus before exercises are created.
* **Pacing:** User controls the speed of unlocking.

## Pros/Cons
* **Pros:** Great for structured learning of large topics. Visualizes the "Map".
* **Cons:** Heavy upfront processing. Less "in the flow" than the Shadow Sensei.
