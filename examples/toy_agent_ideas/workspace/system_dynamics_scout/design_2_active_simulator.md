# Design 2: The Feedback Simulator (Active)

## Purpose
This agent goes beyond mapping; it attempts to *simulate* the system. It parses descriptions of system behavior and generates executable Python simulation code (using simpy or scipy). It allows architects to ask 'What if?' questions.

## Tool Usage
*   **filesystem**: Reads code/docs, writes simulation scripts (simulation.py).
*   **shell**: Executes the simulation scripts and captures output (CSV/Plots).
*   **web**: Searches for standard coefficients (e.g., 'typical latency of Redis lookup').
*   **memory**: Stores the 'Model Specification' (Stocks, Flows, Parameters).

## Loop Structure
1.  **Hypothesis Generation**: The agent reads an ADR (Architecture Decision Record) like 'Use Retry with Exponential Backoff'.
2.  **Model Synthesis**: It prompts an LLM to generate a simpy model representing this logic.
3.  **Parameterization**: It fills in gaps using Web Search ('average network jitter AWS').
4.  **Simulation Run**: It runs the script for '100 simulation hours'.
5.  **Analysis**: It checks for stability. Did the queue grow to infinity? Did latency spike?
6.  **Alert**: If the simulation fails (unstable), it comments on the PR/ADR.

## Memory Architecture
*   **Entities**: Model, Parameter, Scenario.
*   **Relations**: USES_PARAMETER, TESTED_SCENARIO.

## Human Touchpoints
*   Checkpoint-based. The agent proposes a simulation.py. The user must review/run it.
