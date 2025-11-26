# Design 2: The Active Warden (Innovative/Aggressive)

## Purpose
A proactive security agent that not only maps the network but aggressively interrogates devices to determine their OS, open ports, and potential vulnerabilities. It acts as an automated "Red Team" for the home/office network.

## Loop Structure
1. **Sweep:** Run `nmap -sn` (Ping Scan) to find live hosts.
2. **Interrogate:** For new hosts, run `nmap -sV -O` (Service/OS detection).
3. **Vulnerability Match:** Extract service banners (e.g., "Apache 2.4.49") and query `web` (Brave Search / CVE databases) for known exploits.
4. **Simulate Attack:** (Optional/Risky) Dry-run exploit scripts or check for default credentials (e.g., try logging into router with admin/admin).
5. **Quarantine/Alert:** If a "Critical" risk is found, alert the user immediately via shell (system notification) or update a "DEFCON" status file.

## Tool Usage
- **shell:** `nmap`, `hydra` (theoretical), `curl`.
- **web:** Searching CVE databases, exploit-db, and vendor security bulletins.
- **memory:**
  - Entity: `Vulnerability`, `Service`.
  - Relation: `Device` HAS_VULNERABILITY `CVE-XYZ`.
- **filesystem:** Storing scan reports, "Evidence" logs.

## Memory Architecture
- **Risk Graph:**
  - `Device` -> `Service` -> `CVE` -> `Risk Level`.
  - Allows queries like "Show me all devices vulnerable to Log4j".

## Failure Modes
- **Network Disruption:** Aggressive scanning can crash fragile IoT devices.
- **False Positives:** Misidentifying a service as vulnerable.
- **Legal/Ethical:** Scanning networks you don't own is illegal. Agent needs strict subnet constraints.

## Human Touchpoints
- **Authorization:** Must explicitly whitelist subnets.
- **Intervention:** Human must authorize any "Fix" or "Block" action.
- **Alerts:** Real-time urgency.
