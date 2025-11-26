# Agent: The Digital Doppelgänger

## Abstract
A privacy-preserving background agent that obfuscates the user's digital footprint by managing a "Portfolio of Personas." Instead of blocking trackers, it drowns them in high-quality, internally consistent noise. It maintains a Memory Graph of several distinct synthetic identities (e.g., "The Retired Golfer," "The EDM Producer," "The DIY Plumber") and rotates through them, browsing the web to build up convincing cookie pools that obscure the user's true signal.

## Core Components
- **The Puppet Master**: The central logic that schedules which persona is active.
- **The Persona Graph (Memory)**: A graph database storing the backstory, current interests, and "browsing context" for each persona.
- **The Browser Automator (Web/Shell)**: Executes the actual searches and page visits.

## Execution Loop
1. **Schedule Check**: Runs every hour. Decides if it's time to generate noise.
2. **Persona Selection**: Picks a persona that hasn't been active recently (e.g., "The DIY Plumber").
3. **Context Rehydration**:
    - Queries Memory: `MATCH (p:Persona {name: 'DIY Plumber'})-[:INTERESTED_IN]->(topic)`
    - Retrieves topics: "PVC piping", "Sump pumps", "Basement waterproofing".
4. **Browsing Session**:
    - **Search**: `web_brave_web_search("best sump pumps for heavy rain")`
    - **Explore**: Visits results.
    - **Evolve**: Finds a related term (e.g., "French Drain") and adds it to the Memory Graph: `(DIY Plumber)-[:INTERESTED_IN]->(French Drain)`.
5. **Context Switch**: After 15 minutes, logs off (clears session context in its own sandbox) and updates the graph.

## MCP Tool Usage
### 1. Memory Server
- **Schema**:
    - `Entity(Type=Persona)`: Name, Age, Location.
    - `Entity(Type=Topic)`: Keywords.
    - `Relation(INTERESTED_IN)`: Links Personas to Topics.
    - `Relation(RELATED_TO)`: Links Topics (Knowledge Graph).
- **Usage**:
    - `memory_create_entities`: Create new personas/topics.
    - `memory_search_nodes`: Retrieve current interests to guide browsing.
    - `memory_add_observations`: Log what the persona "learned" today.

### 2. Web Server (Brave)
- **Usage**:
    - `web_brave_web_search`: Perform searches.
    - `web_brave_news_search`: Find articles relevant to the persona's demographic.

### 3. Shell Server
- **Usage**:
    - `shell_execute_command`: (Advanced) Could drive a headless browser (Puppeteer/Playwright) for realistic dwell time and scrolling, rather than just HTTP requests.

## Persistence Strategy
- **Memory Graph**: Holds the "Soul" of the personas. This ensures they don't just search random words (which is easy to filter) but follow logical trajectories of interest (e.g., Buying a camera -> searching for lenses -> searching for tripods). This "Narrative Consistency" makes the noise indistinguishable from real human behavior.

## Failure Modes & Recovery
- **Bot Detection**: If search engines present CAPTCHAs.
    - *Mitigation*: The agent backs off for random intervals (Exponential Backoff). It switches to "Passive Mode" (just reading news feeds) instead of "Active Mode" (searching).
- **Topic Contamination**: If a persona accidentally drifts into the Real User's actual interests (e.g., User likes Coding, Persona drifts into Coding).
    - *Mitigation*: User defines an "Exclusion List" (The Real Profile). The agent checks every new topic against this list before adopting it.

## Human Touchpoints
- **Setup**: User picks 3-5 archetypes (or lets the agent generate them randomly).
- **Exclusion List**: User inputs their *real* interests so the agent *avoids* them (creating a "Negative Space" around the user).
- **Status Report**: Agent provides a weekly "Confusion Report": "This week I generated 5,000 queries about 'Industrial Knitting' and 'Crypto-zoology'."

## Key Insight
**Narrative Noise > Random Noise**.
Ad algorithms are designed to filter out random spikes. They are *not* designed to filter out consistent, multi-week behavioral patterns that look like a legitimate demographic segment. By maintaining stateful personas in a graph, the Digital Doppelgänger defeats the "Bot Filter" and successfully poisons the commercial profile.
