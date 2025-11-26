# Design 3: The Behavioral Biologist (Hybrid/Social)

## Purpose
To treat the network as a biological ecosystem. This agent ignores technical specs (CVEs, Ports) and focuses on **Behavioral Fingerprinting**. It builds "Ethograms" (behavior profiles) for devices to understand their *role* and *health*.

## Loop Structure
1. **Observation:** Passive monitoring (like Design 1) but focused on *patterns*:
   - "Sleep/Wake cycles" (Circadian rhythm).
   - "Call Home frequency" (Who does it talk to on the internet?).
   - "Jitter/Burstiness".
2. **Hypothesis Generation:**
   - "Device X sleeps all day and streams high bandwidth to Netflix at 8 PM. It is likely a TV."
   - "Device Y beacons to China every 10 seconds. It is likely a cheap IoT plug."
3. **Research:** Use `web` to search for "What servers does a Philips Hue Bridge talk to?" and compare with observation.
4. **Role Assignment:** Update the Memory Graph with the inferred role (e.g., `Role: StreamingDevice`, `Role: Infrastructure`).
5. **Anomaly Detection:** "The TV is waking up at 3 AM. This is biological drift (Illness)."

## Tool Usage
- **shell:** `tcpdump`, `whois` (to identify remote IPs), `nslookup`.
- **memory:**
  - Entity: `Device`, `RemoteEndpoint` (e.g., "Netflix Servers").
  - Observation: "ActivityProfile: Nocturnal".
- **web:** Searching for device behaviors, domain reputation, and "Unknown Traffic" forums.
- **filesystem:** "Field Notes" (Journal style reports).

## Memory Architecture
- **Behavioral Ontology:**
  - Nodes: `Device`, `BehaviorPattern`, `ExternalEntity`.
  - Edges: `EXHIBITS`, `COMMUNICATES_WITH`.
- **Anomaly Scoring:** Each device has a "Strangeness" score based on deviation from its established Ethogram.

## Failure Modes
- **Misinterpretation:** Gaming traffic might look like DDoS.
- **Dynamic IPs:** Cloud services change IPs frequently, making "Call Home" tracking hard. Mitigation: Use DNS names.

## Human Touchpoints
- **"Naturalist's Journal":** The output is a readable narrative: "The Living Room Speaker was unusually active this morning."
- **Inquiry:** The agent asks: "I suspect Device A is a Printer. Is this correct?"
