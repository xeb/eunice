# Design 3: The Viral Graph Guardian (Hybrid)

## Purpose
A "Just-in-Time" compliance agent that focuses on *education* and *preventing* pollution before it happens. It treats the legal graph as a living ecosystem, alerting developers *while they are coding* if they import a risky library.

## Loop Structure
1. **Interception:** Runs as a local daemon watching the file system.
2. **Analysis:** When a line like `import X from 'Y'` is written, it checks the license of 'Y' in the background.
3. **Feedback:** If 'Y' is high-risk (e.g., viral copyleft), it writes a warning to a `compliance.log` or updates a dedicated "Legal Dashboard" terminal window.
4. **Graphing:** Continually updates a visual "Dependency Risk Map" (HTML file) in the root directory, showing "Infection Vectors" (paths from root to restricted libs).
5. **Attribution:** Automatically fetches and appends the correct attribution text to a centralized `NOTICES` file.

## Tool Usage
- **grep:** Real-time scanning of import statements.
- **memory:** "Trust Graph" - learning which vendors/maintainers usually use permissive licenses.
- **fetch:** Download license text for hashing/verification.
- **web:** Verify maintainer reputation.

## Memory Architecture
- **Entities:** `Maintainer`, `Repository`, `LicenseContext`.
- **Relations:** `Maintainer PUBLISHES Package`, `License VIRAL_PATH_TO Root`.

## Failure Modes
- **Noise:** Too many alerts (Mitigated by "Quiet Mode" or only flagging High Severity).
- **Lag:** Web search might be slow (Mitigated by aggressive caching in Memory).

## Human Touchpoints
- Configuring "Risk Tolerance" (e.g., "Allow LGPL linked dynamically").
- Reviewing the `NOTICES` file before release.
