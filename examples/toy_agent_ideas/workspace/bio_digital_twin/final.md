# Final Design: The Bio-Digital Twin (The N-of-1 Scientist)

## Purpose
The **Bio-Digital Twin** is an autonomous "Principal Investigator" for your personal biology. It bridges the gap between **Quantified Self** (collecting data) and **Evidence-Based Medicine** (applying research). Instead of just showing a graph of your sleep, it notices your deep sleep is low, searches PubMed for causes, hypothesizes that your late-night magnesium intake might be the variable, and proposes a 1-week split-test to verify.

## Core Loop: The "Scientific Method" Loop
1.  **Observation (Ingest)**:
    *   Continuously watches `inbox/` for data exports (Apple Health, Oura, CSVs).
    *   Parses and updates the **Biometric Graph** in `memory`.
2.  **Analysis (Signal Detection)**:
    *   Runs statistical correlation checks on the graph (e.g., `Correlation(Alcohol, HRV) = -0.8`).
    *   Identifies outliers (e.g., "HRV is 2-sigma below baseline").
3.  **Research (Literature Review)**:
    *   If a strong correlation or anomaly is found, it uses `web_brave_web_search` to find relevant papers.
    *   *Query*: "alcohol heart rate variability mechanism pubmed".
    *   Stores findings as `Paper` nodes linked to `Biomarker` nodes.
4.  **Experimentation (The Proposal)**:
    *   Drafts an experiment protocol in `experiments/proposals/` (e.g., "Protocol: Alcohol Cessation n=7").
    *   Waits for user approval (moving file to `experiments/active/`).
5.  **Synthesis (The "Paper")**:
    *   After the experiment window, it analyzes the delta.
    *   Writes a final report: "The Effect of Alcohol on Subject A's HRV: An N=1 Study" with citations from the web search.

## Tool Usage
*   **memory** (The Brain):
    *   **Ontology**: `Biomarker`, `Intervention`, `Symptom`, `Paper`, `Mechanism`.
    *   **Graph**: `(Subject) --took--> (Magnesium) --at--> (22:00)`.
    *   **Causality**: `(Magnesium) --suggested_to_improve--> (Sleep Quality) --source--> (Paper ID)`.
*   **web** (The Library):
    *   Targeted searches for mechanisms and interactions.
    *   Verifying if a local correlation is a known biological phenomenon or a fluke.
*   **filesystem** (The Lab Notebook):
    *   `inbox/`: Drop zone for data.
    *   `experiments/`: Markdown files managing the state of N=1 trials.
    *   `knowledge_base/`: Summaries of conditions and relevant papers.
*   **grep**:
    *   Scanning raw data logs for specific event tags before ingestion.

## Memory Architecture
The graph is divided into two layers:
1.  **The Empirical Layer**: What actually happened (Data).
    *   `Observation` nodes: Time-stamped values.
2.  **The Theoretical Layer**: Why it happened (Knowledge).
    *   `Mechanism` nodes: Abstract biological concepts.
    *   `Paper` nodes: External validation.
    *   *Link*: `(Observation A) --supports--> (Mechanism B)`.

## Failure Modes & Recovery
1.  **Data Drought**: User stops syncing data.
    *   *Response*: Agent shifts to "Literature Review" mode, researching previously identified issues deeply, filling the `knowledge_base` with "Recommended Reading."
2.  **False Positives**: Finding correlations in noise.
    *   *Response*: The "Literature Filter". If the agent finds a correlation (e.g., "Carrots -> Headache") but finds ZERO papers on it, it flags it as "Low Confidence / Spurious" in the report.
3.  **Privacy Leak**:
    *   *Response*: Agent is strictly local. Web searches are anonymized (searching for general mechanisms, not uploading user data).

## Human Touchpoints
*   **The Ethics Board**: User must approve all experiments.
*   **The Subject**: User must provide the data (syncing devices).
*   **The Peer Reviewer**: User reviews the generated "Papers" to validate the logic before accepting the conclusions into the permanent graph.
