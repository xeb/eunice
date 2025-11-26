# Design 2: The N-of-1 Principal Investigator (Innovative)

## Purpose
An active research agent that treats the user's life as a continuous "N=1" clinical trial. Instead of just tracking data, it formulates hypotheses, designs micro-experiments, and verifies findings against medical literature to optimize health outcomes.

## Loop Structure
1.  **Hypothesis Generation**: Analyzes correlations in the graph (e.g., "Deep sleep is lower on days with late caffeine").
2.  **Literature Verification**: Uses `web_brave_web_search` to find papers confirming/refuting this mechanism (e.g., "caffeine half-life adenosine receptors").
3.  **Proposal**: Writes an experiment protocol to `proposals/experiment_01.md` (e.g., "Cut caffeine after 2 PM for 7 days").
4.  **Monitoring**: If user accepts, it tracks compliance and outcome metrics specifically for the experiment.
5.  **Publication**: Generates a "Personal Paper" summarizing the findings with citations.

## Tool Usage
*   **web**:
    *   Searching PubMed/ArXiv for mechanisms explaining observed local correlations.
    *   Checking drug/supplement interactions.
*   **memory**:
    *   **Hypothesis Graph**: `(Intervention A) --hypothesized_effect--> (Metric B)`.
    *   **Evidence Graph**: Linking local observations to external URL citations.
*   **filesystem**:
    *   Managing the "Lab Notebook" (Markdown files for each experiment).

## Memory Architecture
*   **Entities**: `Hypothesis`, `Experiment`, `Paper`, `Mechanism`, `Biomarker`.
*   **Relations**: `SUPPORTED_BY` (Paper), `REFUTED_BY` (Data), `CAUSES` (Mechanism).
*   **Persistence**: "Twin-State" — The graph represents the *theory* of the user's biology; the filesystem contains the *proof*.

## Failure Modes
*   **Spurious Correlations**: Agent finds random noise (e.g., "Wearing blue socks increases HRV").
    *   *Recovery*: The "Literature Verification" step filters this out—if no biological mechanism exists in literature, the hypothesis is down-weighted.
*   **Hallucination**: Agent misinterprets a medical paper.
    *   *Recovery*: It must quote the abstract directly in the "Personal Paper" for human verification.

## Human Touchpoints
*   **IRB (Institutional Review Board)**: The user acts as the ethics board, approving or rejecting every proposed experiment.
*   **Subject**: The user must manually log qualitative data (mood, energy) if sensors miss it.
