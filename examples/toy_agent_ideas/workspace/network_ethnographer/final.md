# Agent Design: The Network Ethnographer

## Problem Statement
Modern home and office networks are populated by "Black Box" devices (Smart TVs, IoT plugs, Voice Assistants) that are opaque to the user. Traditional security tools (firewalls, antivirus) are binary (allow/block) and technical (ports/protocols), failing to capture the *behavioral* nuance needed to detect compromised or misbehaving IoT devices (e.g., a baby monitor suddenly streaming to a new country).

## The Core Solution
**The Network Ethnographer** is an autonomous background agent that treats the local network as a biological ecosystem. It passively observes packet flows to build "Ethograms" (behavioral profiles) for every device, identifying them not just by their Manufacturer ID, but by their "Lifestyle" (sleep cycles, communication partners, data volume).

## Key Architectural Insight
**Behavioral Topology over Network Topology**:
Instead of a static map of IPs, the agent builds a **Temporal Knowledge Graph** where nodes are Devices and edges are *Behavioral Habits*. It detects anomalies by observing deviations from the *evolved baseline* (e.g., "The Printer is acting like a Web Server today") rather than matching static attack signatures.

## Tools & Capabilities

### 1. Shell (The Eyes)
- **Passive Monitoring:** Uses `tshark` or `tcpdump` to capture headers (not payloads) of broadcast and unicast traffic.
- **Active Probing (Low Frequency):** Uses `nmap -sn` and `arp-scan` to detect silent devices.
- **DNS Resolution:** Uses `dig`/`nslookup` to map destination IPs to human-readable services (e.g., "1.2.3.4" -> "netflix.com").

### 2. Memory (The Brain)
- **Graph Database:** Stores the "Ecosystem Model".
  - **Entities:** `Device` (MAC), `Service` (Netflix), `Behavior` (Nocturnal).
  - **Relations:** `Device A` -> `TALKS_TO` -> `Service B`.
  - **Observations:** "Device X wakes up every day at 07:00 UTC."
- **Baseline Storage:** Stores statistical norms (mean bytes/hour, standard deviation) for drift detection.

### 3. Web (The Library)
- **Brave Search:** Queries for unknown MAC OUIs, unfamiliar ports, and destination domains (e.g., "What is tuya-cn.com?").
- **Documentation Lookup:** Searches for device manuals to verify observed behavior (e.g., "Does Hue Bridge use Port 80?").

### 4. Filesystem (The Journal)
- **Daily Reports:** Generates a Markdown "Field Journal" describing the day's ecosystem activity.
- **Anomaly Alerts:** Writes urgent "Invasive Species" reports when behavior drifts significantly.

## Execution Loop (The "Naturalist's Cycle")

1.  **Observation (Listen):**
    *   Agent wakes up (e.g., every 15 mins).
    *   Captures 60 seconds of traffic headers.
    *   Parses Source/Dest IPs, Ports, and Protocol.

2.  **Taxonomy (Classify):**
    *   For known devices: Update traffic stats.
    *   For new devices: Query OUI, infer type from traffic (e.g., HTTP -> Web Client).
    *   **Research Step:** If a device behaves strangely (e.g., Thermostat talking to a Game Server), trigger a Web Search to explain the connection.

3.  **Synthesis (Graph Update):**
    *   Update the **Ethogram**: "Device A is usually dormant 9am-5pm."
    *   Check for **Drift**: "Device A is active at 11am today. Strangeness Score: High."

4.  **Narrative Generation (Report):**
    *   Synthesize observations into a "Daily Nature Walk" report.
    *   *Example:* "The Living Room TV was active for 4 hours. It communicated primarily with Netflix and AWS. A new device (Guest Phone) appeared at 2pm."

## Failure Modes & Recovery
-   **Encrypted Traffic:** The agent cannot see *what* is being sent. *Recovery:* Rely on Metadata (Volume, Timing, Destination IP reputation).
-   **MAC Randomization:** Mobile devices change MACs. *Recovery:* Use heuristic correlation (Signal Strength + Timing) to group "Transient MACs" into a single "Mobile Guest" entity.
-   **Flooding:** High network load. *Recovery:* Dynamic sampling rate (reduce capture duration).

## Human Interaction
-   **The "Nature Walk" Report:** A daily summary of network health.
-   **Naming Ceremony:** The user can explicitly name devices ("This is Dad's iPad") to help the agent refine its model.
