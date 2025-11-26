# Design 3: The Epigenetic Mirror (Hybrid)

## Purpose
A predictive simulation agent that projects the user's current habits 10-20 years into the future. It uses actuarial data and medical studies to create a "Future Self" persona that communicates with the present user, incentivizing behavior change through narrative and visualization.

## Loop Structure
1.  **State Assessment**: Aggregates last 30 days of health data.
2.  **Projection**: Searches `web` for long-term studies on current habits (e.g., "impact of 6h sleep on alzheimers risk").
3.  **Simulation**: Updates the "Future Self" profile (e.g., "Future Self has high blood pressure").
4.  **Dialogue**: The "Future Self" writes a letter or daily briefing to the user, pleading for specific changes or thanking them for good choices.

## Tool Usage
*   **web**:
    *   Finding "Hazard Ratios" and longitudinal studies.
    *   Retrieving actuarial tables.
*   **memory**:
    *   **Persona State**: The attributes of the "Future Self".
    *   **Risk Model**: Probabilistic links between current habits and future outcomes.
*   **filesystem**:
    *   `daily_briefing.md`: The interface for the user.

## Memory Architecture
*   **Entities**: `Habit`, `RiskFactor`, `FutureOutcome`, `Study`.
*   **Relations**: `INCREASES_RISK_OF`, `MITIGATES`, `PROJECTED_ONSET`.
*   **Persistence**: The "Future Self" is a persistent entity in the graph that evolves as the user's habits change.

## Failure Modes
*   **Alarmism**: Agent becomes too negative/anxious about minor risks.
    *   *Recovery*: Tunable "Optimism/Stoicism" parameters in the config.
*   **Data Gaps**: Missing data leads to wild extrapolations.
    *   *Recovery*: Confidence intervals attached to every projection.

## Human Touchpoints
*   **Audience**: The user reads the narrative output.
*   **Tuner**: The user adjusts the "Age" of the simulation (e.g., "Show me myself at 80 vs 50").
