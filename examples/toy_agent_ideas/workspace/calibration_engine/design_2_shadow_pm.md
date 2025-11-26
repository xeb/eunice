# Design 2: The Shadow Project Manager

## Purpose
To autonomously infer project velocity and predict delivery dates without requiring any manual data entry from the developer.

## Loop Structure
1. **Continuous Watch:** Monitors file changes and git commits in the background.
2. **Cluster Analysis:**
   - Uses `memory` to group commits into "Inferred Features" based on file locality (editing related files) and semantic similarity in messages.
   - Example: Commits touching `auth.ts`, `login.vue`, and `user_db.sql` within 2 days are clustered as "Feature A".
3. **Forecasting:**
   - When a *new* cluster begins (first commit to a new set of files), the agent searches its Memory Graph for similar past clusters.
   - It calculates the average duration of those past clusters.
4. **Reporting:**
   - Generates a dynamic dashboard `status_forecast.html` in the repo root.
   - "Current Focus: Auth Refactor. Predicted remaining time: 12h (Confidence: High)".

## Tool Usage
- **shell:** `git log`, `git diff --stat`.
- **memory:** Graph Database to store:
  - `Node(Cluster)` -> `Edge(Contains)` -> `Node(Commit)`
  - `Node(Cluster)` -> `Property(Duration)`
  - `Node(Cluster)` -> `Property(ComplexityScore)`
- **web:** (Optional) To check calendar holidays.

## Memory Architecture
- **Graph-Based:** Essential for clustering. Nodes represent Files, Commits, and inferred Features. Edges represent "ModifiedTogether" or "SemanticallyRelated".

## Failure Modes
- **Context Switching:** If the user jumps between 3 features in one day, the clustering algorithm might merge them into one giant "Mess" feature, skewing the data.
- **Refactoring vs. Feature:** Hard to distinguish a 2-hour bugfix from a 2-week refactor without semantic understanding.

## Human Touchpoints
- Zero-touch. The user just codes. The agent observes.
- Optional: User can "Name" a cluster in the dashboard to improve future training.
