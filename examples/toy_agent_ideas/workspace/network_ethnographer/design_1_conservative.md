# Design 1: The Passive Observer (Conservative)

## Purpose
A lightweight, privacy-preserving background daemon that builds a "Census" of devices on the local network without generating active traffic or triggering intrusion detection systems. It focuses on long-term trend analysis of device presence and traffic volume.

## Loop Structure
1. **Listen:** Run `tshark` or `tcpdump` for fixed windows (e.g., 5 minutes every hour) to capture broadcast/multicast traffic and headers.
2. **Census:** Parse MAC addresses and OUI (Organizationally Unique Identifier).
3. **Graph Update:**
   - Nodes: Devices (MACs).
   - Edges: "Talked To" (Flows).
   - Update `last_seen` timestamps and `bytes_transferred` counters in Memory.
4. **Drift Detection:** Compare current census to historical baseline. Identify "New Neighbors" or "Missing Regulars".
5. **Report:** Generate a Markdown daily summary in `filesystem`.

## Tool Usage
- **shell:** `tshark -I -a duration:300`, `arp -a`, `ip neigh`.
- **memory:** Stores the Device Graph.
  - Entity: `Device` (MAC, IP, Vendor).
  - Observation: "Seen at [Timestamp]", "Volume [X] bytes".
- **filesystem:** Stores PCAP summaries (anonymized) and daily Markdown reports.
- **web:** *Minimal usage* (only for OUI lookup if local DB fails).

## Memory Architecture
- **Graph Topology:**
  - `Device` nodes linked by `Communication` edges.
  - `Cluster` nodes for identifying "Device Groups" (e.g., "The Apple Ecosystem").
- **Persistence:** High. Historical data is crucial for establishing baselines.

## Failure Modes
- **Encryption:** Cannot see payload (feature, not bug).
- **MAC Randomization:** Modern phones change MACs. Agent needs logic to correlate "Transient MACs" based on traffic timing/patterns.
- **Flooding:** High traffic volume could fill memory/disk. Mitigation: Sampling and summary stats only.

## Human Touchpoints
- **Read-Only:** Human reads the "Daily Census" report.
- **Naming:** Human can manually tag a MAC address as "My iPhone" to train the system.
