# Design 3: The Simulation Director

## Purpose
An experimental agent that treats the story world as a running simulation. It tracks "off-screen" state—what are the other factions doing? How does the economy react to the war? It suggests consequences for the author's plot choices.

## Core Toolset
- **memory**: High-fidelity state tracking (resources, troop movements, relationships).
- **shell**: Run simple Python scripts/simulations for calendars, moon phases, or travel times.
- **web**: Research logistics (e.g., "How long does it take a horse to travel 50 miles?").

## Loop Structure
1.  **State Snapshot**: At the end of each chapter, the agent updates the "World State" (Date, Weather, Faction Resources).
2.  **Simulation Step**:
    -   Advance time by X days.
    -   Simulate off-screen agent goals (e.g., The Antagonist advances their plan by 1 step).
3.  **Suggestion**:
    -   Generate a "World Events" report: "While your hero was in the tavern, the Northern Army captured the Bridge of Souls."
    -   Alert on logistics: "You wrote they arrived in 2 days, but at this season/terrain, it takes 5."

## Memory Architecture
-   **State Vector**: Complex JSON objects in memory nodes tracking numeric values (Gold, HP, Distance).
-   **Rule Engine**: Logic stored in memory determining cause-and-effect.

## Failure Modes
-   **Over-rigid**: Complaining about "rule of cool" violations (e.g., "That magic spell violates the law of thermodynamics").
-   **Complexity Explosion**: Trying to simulate too many variables leads to noise.
-   **Recovery**: User can manually override the simulation state ("Magic portal opened, travel time is now 0").

## Human Touchpoints
-   **Game Master**: The agent acts as a GM, the author is the player.
-   **Configuration**: Setting the "simulation strictness" (Hard Sci-Fi vs. Soft Fantasy).
