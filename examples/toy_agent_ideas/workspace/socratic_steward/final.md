# Agent: The Socratic Steward

## Abstract
The Socratic Steward is an autonomous background agent that acts as an "Epistemic Auditor" for your personal knowledge base. Unlike passive wikis or simple flashcard generators, the Steward actively reads your notes, cross-references them with the web to verify claims, and initiates "Socratic Dialogues" when it detects gaps, contradictions, or weak arguments. It turns static note-taking into an active debate with a tireless, objective partner.

## Problem Domain
Personal Knowledge Management (PKM) often suffers from the "Collector's Fallacy"—users hoard notes without truly understanding them. Additionally, static notes can become "stale" or factually incorrect over time. Users need a system that enforces **Active Recall** and **Truth Maintenance** without requiring manual scheduling.

## Core Toolset
* **memory**: Builds a persistent graph of Concepts, Claims, and Evidence derived from user notes.
* **web (Brave)**: Used to verify claims, find counter-arguments, and fetch definitions for undefined terms.
* **filesystem**: The primary user interface. The agent reads `.md` files and writes to a specific `inbox/` or `discussion/` directory.
* **grep**: For efficient scanning of large knowledge bases to find concept occurrences.

## Architecture & Loop

### 1. The Audit Loop (Background)
*   **Trigger**: File modification or scheduled nightly scan.
*   **Action**: The agent parses the note, extracting "Claims" (assertions of fact) and "Concepts" (entities).
*   **Graph Update**: It updates the internal Knowledge Graph.
*   **Verification**: For high-confidence claims, it performs a **Brave Web Search** to check for consensus.
    *   *Match*: The claim is marked "Verified" in the graph.
    *   *Conflict*: The claim is marked "Disputed".

### 2. The Challenge Protocol (Intervention)
*   **Trigger**: A "Disputed" claim or an "Orphaned" concept (used but never defined) is found.
*   **Action**: The agent creates a **Briefing** in the user's `inbox/`.
    *   *File Name*: `challenge_on_[topic].md`
    *   *Content*: "You stated X, but recent sources (Link A, Link B) suggest Y. How do you reconcile this?"
*   **Goal**: To provoke the user into updating the note or defending their position.

### 3. The Dialogue Mode (Interaction)
*   **Trigger**: The user writes a reply in the `challenge_*.md` file.
*   **Action**: The agent reads the user's defense. It adopts a specific "Persona" (e.g., The Skeptic, The Novice) to push the user deeper.
*   **Output**: The agent appends its counter-reply to the same file, creating a threaded conversation.
*   **Closure**: When the user is satisfied, they can tag the file `#resolved`. The agent then summarizes the conclusion and offers to update the original note automatically.

## Persistence Strategy
*   **Graph Database**: Stores the "Meta-Model" (what the user *thinks* they know vs. what the web says).
*   **Filesystem**: Stores the "Source of Truth" (the actual notes) and the "Interaction History" (the dialogue files).

## Failure Modes & Recovery
1.  **False Positives (The "Pedant" Problem)**: The agent flags a nuance as an error.
    *   *Recovery*: User tags the challenge as `#ignore`. The agent adds an exclusion edge in the memory graph.
2.  **Infinite Loops**: The agent keeps replying to itself or the user endlessly.
    *   *Recovery*: The dialogue file has a max-turn limit per session. The agent must explicitly ask "Do you want to continue?" after 5 turns.
3.  **Privacy Leaks**: Sending private notes to the web search.
    *   *Recovery*: A `.stewardignore` file lists folders (e.g., Journals) that are never scanned or searched.

## Key Insight
**"Knowledge as a Negotiation"**: Instead of treating notes as static facts, this agent treats them as *claims* that must be defended against an external reality. It shifts the user's relationship with their notes from "Librarian" to "Debater."
