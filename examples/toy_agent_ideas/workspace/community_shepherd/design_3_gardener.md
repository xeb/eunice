# Design 3: The Ecosystem Gardener (Holistic)

## Purpose
To monitor the systemic health of the community, treating it like a biological ecosystem. It detects "Semantic Drift," "Toxic Pockets," and "Dead Zones" (channels with no activity). It focuses on *groups* and *trends* rather than individuals.

## Core Loop
1. **Aggregate:** Read logs from all channels/repos over the last 24h.
2. **Measure:** Calculate metrics:
   - **Temperature:** Volume of discussion.
   - **Sentiment:** Ratio of positive/constructive to negative/destructive.
   - **Cohesion:** How many different users are interacting?
3. **Diagnose:**
   - "Channel X is freezing" (No posts in 7 days).
   - "Channel Y is overheating" (High volume + Negative sentiment).
4. **Intervene:**
   - **Seeding:** Post a "Prompt of the Week" in a dead channel (from a library of prompts).
   - **Cooling:** Enable "Slow Mode" (simulated recommendation) in heated channels.
   - **Reporting:** Generate a "Weather Report" for community managers.

## Tool Usage
- **memory:** Storing time-series data points (via observations) and Channel entities.
- **shell:** Executing data analysis scripts (python/jq) on log files.
- **filesystem:** Storing the "Prompt Library" and daily HTML/Markdown reports.

## Memory Architecture
- **Entities:** `Channel`, `Topic`, `Trend`.
- **Observations:** "Sentiment was -0.5 on 2025-11-25", "Activity dropped 20%".

## Failure Modes
- **Misinterpretation:** High activity might be a celebration, not a riot. *Recovery:* Sentiment analysis must differentiate "Excitement" from "Anger".
- **Spamming:** The "Seeding" prompts become annoying. *Recovery:* Limit interventions to 1/week per channel.

## Human Touchpoints
- **Strategy Session:** Humans read the "Weather Report" to decide on broad community initiatives.
- **Content Calibration:** Humans write the "Seed Prompts."
