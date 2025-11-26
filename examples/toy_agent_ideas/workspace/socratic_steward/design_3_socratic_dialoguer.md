# Design 3: The Socratic Dialoguer

## Purpose
A learning agent that simulates a tutor or debate partner. Instead of just quizzing facts, it opens a dynamic chat file (e.g., `conversation_with_plato.md`) to discuss the concepts in your notes, helping you deepen your understanding through dialogue.

## Loop Structure
1. **Trigger**: User tags a note with `#discuss`.
2. **Persona Gen**: The agent adopts a persona relevant to the content (e.g., "The Skeptic", "The Child", "The Expert").
3. **Dialogue**: The agent writes a question or prompt into a new Markdown file.
4. **Listen**: It watches that file for the user's response.
5. **Reply**: When the user saves the file, the agent reads the new text, processes it against the memory graph/web, and writes a reply.

## Tool Usage
* **filesystem**: The primary interface. The "Chat" is just a text file being edited by two parties (Human and Agent) asynchronously.
* **memory**: Stores the context of the conversation and the user's demonstrated knowledge level.
* **web**: Fetches examples to use in the dialogue ("Consider the case of X...").

## Memory Architecture
* **Episodic Memory**: Each conversation is a node in the graph, linked to the concepts discussed.
* **User Model**: The agent builds a profile of what the user understands well vs. where they are shaky.

## Failure Modes
* **Desync**: User and Agent editing the file at the same time. *Recovery*: Use file locking or distinct "Turn" markers (e.g., `> User:`, `> Agent:`).
* **Looping**: Agent repeats the same arguments. *Recovery*: Check conversation history in memory before generating a reply.

## Human Touchpoints
* **Deep Work**: This is a high-engagement agent. The user actively sits down to "talk" with their notes.
