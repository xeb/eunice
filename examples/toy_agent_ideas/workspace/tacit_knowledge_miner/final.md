# Agent: The Tacit Knowledge Miner

## Core Tools
**shell**, **memory**, **grep**, **text-editor**

## Problem Domain
**Implicit Knowledge Loss & Bus Factor Risk.**
In every software project, the most critical knowledge (the "Why") is often not in the code or the docs, but in the heads of specific developers. When they leave or forget, that knowledge is lost. Existing tools measure this risk (Bus Factor) but do not actively *mitigate* it by extracting the knowledge.

## Key Insight
**"Just-in-Time Socratic Interrogation"**
Instead of asking developers to "write documentation" (which they hate and procrastinate), the agent acts as a curious background process that detects *high-leverage moments*—like a complex edit to a legacy file—and asks a single, specific question: "Why did you change X?". It captures the answer into a persistent Knowledge Graph, gradually externalizing the team's mental model.

## Architecture

### 1. The Risk Monitor (The "Ears")
*   **Tools:** `shell` (git), `grep`.
*   **Behavior:**
    *   Runs on a schedule or git-hook.
    *   Calculates a **Risk Score** for every file based on:
        *   **Bus Factor:** (1 / Number of unique committers in last year).
        *   **Complexity:** (Indentation depth, file length).
        *   **Churn:** (Frequency of changes).
    *   Identifies "Dark Zones" (High Risk, Low Documentation).

### 2. The Socratic Interceptor (The "Mouth")
*   **Tools:** `fetch` (LLM API for question generation), `shell` (CLI interaction or PR comment).
*   **Behavior:**
    *   **Trigger:** When a developer commits changes to a "Dark Zone" file.
    *   **Action:** The agent generates a context-aware question.
        *   *Example:* "I see you're touching `LegacyAuth.py`. This file has a Bus Factor of 1 (You). Could you explain the side-effect risks of this change for the next person?"
    *   **Constraint:** Low friction. Max 1 question per day per dev.

### 3. The Knowledge Graph (The "Brain")
*   **Tools:** `memory`.
*   **Structure:**
    *   **Entities:** `Developer`, `File`, `Concept` (e.g., "PaymentLogic"), `Constraint`.
    *   **Relations:**
        *   `Developer OWNS File` (derived from git).
        *   `Developer EXPLAINED Concept` (derived from answers).
        *   `File ENFORCES Constraint`.
    *   **Value:** Queries like "Who understands the PaymentLogic?" return results based on *explained knowledge*, not just lines of code.

## Persistence Strategy
**Hybrid (Graph + Inline Docs):**
1.  **Short-term:** Answers are stored in the `memory` graph for querying and analysis.
2.  **Long-term:** Periodically, the agent takes high-value answers and uses `text-editor` to append them as **docstrings** directly into the code, turning tacit knowledge into explicit documentation.

## Autonomy Level
**High (Background Daemon):**
The agent runs autonomously to monitor and identify risks. It requires Human-in-the-Loop only for the brief "Interview" moments.

## Handling Failures
*   **Refusal to Answer:** If a dev ignores the prompt, the agent records a "Resistance" score. If high, it stops pestering that user and flags it to the team lead.
*   **Hallucination:** The agent never *invents* documentation. It only aggregates *human* answers.

## Future Composability
*   Can feed into a **"Onboarding Agent"** that uses the graph to generate a personalized curriculum for new hires.
*   Can feed into a **"Refactoring Steward"** to warn about hidden constraints before code is deleted.
