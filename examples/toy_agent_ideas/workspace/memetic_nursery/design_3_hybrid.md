# Design 3: The Memetic Nursery (Hybrid)

## Purpose
To treat the "Idea Backlog" as a biological nursery where ideas are "Seeds" that need different resources (Context, Feasibility, Market Fit) to grow. It combines passive research with active, safe verification.

## Loop Structure (The "Gardening Cycle")
1. **Germination (Ingest)**: User drops a rough note ("Make an app that maps smells"). Agent parses it into a structured "Seed Node" in Memory.
2. **Watering (Context)**: Agent searches web for "Smell mapping apps", "Olfactory sensors", "Competitors". Adds findings to the file.
3. **Pruning (Feasibility Check)**: Agent checks technical constraints. "Does the iPhone have a smell sensor?" -> Result: No.
    - **Action**: Mark as `Dormant: Waiting for Hardware`.
    - **Trigger**: Set a "Watch" on "Mobile Olfactory Sensors".
4. **Growth (Prototyping)**: If hardware exists, Agent scaffolds a "Hello World" using the API to verify access.
5. **Harvest (Presentation)**: When an idea reaches "High Feasibility" and "High Market Interest" (based on web trends), Agent moves it to `ready_to_build/` and notifies the user.

## Tool Usage
- **memory**: Stores the "Genome" of the idea (Requirements, blockers, maturity score).
- **web_brave_web_search**: Continuous environmental scanning (Trends, Competitors, API Releases).
- **filesystem**: Maintains the "Plant Bed" (Markdown files) and "Fruit" (Prototypes).
- **fetch**: Downloads whitepapers or API specs.

## Memory Architecture
- **Maturity Score (0-100)**: Calculated based on:
    - Clarity (Does it have requirements?)
    - Feasibility (Do dependnecies exist?)
    - Novelty (Are there fewer than 5 competitors?)
- **Evolutionary Graph**: Tracks how Idea A evolved into Idea B.

## Failure Modes
- **False Positives**: Agent thinks a new API solves the problem but it doesn't. *Recovery*: User review of the "Feasibility Report".
- **Notification Spam**: Agent alerts on every minor update. *Recovery*: "Significance Thresholds" for alerts.

## Human Touchpoints
- **Planting**: User writes the initial seed.
- **Tending**: User answers specific questions from the Agent ("Is this for iOS or Android?").
- **Harvest**: User decides to actually build the project.
