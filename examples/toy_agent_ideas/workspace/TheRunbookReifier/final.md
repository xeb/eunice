# Agent: The Runbook Reifier

## Concept
The Runbook Reifier is an autonomous "DevOps Historian" that solves the problem of stale or missing documentation. Instead of asking engineers to write runbooks *before* doing the work, it observes their shell sessions *during* an incident, reverse-engineers the logical steps, and generates an "Executable Runbook" (Markdown + Code) for approval. Once approved, the agent can autonomously execute these steps to solve future recurrences of the same issue.

## Problem Domain
*   **Incident Response**: Panic induces amnesia; fixes aren't documented.
*   **Onboarding**: Junior devs don't know the "magic commands" seniors use.
*   **Automation**: Scripts are brittle; "Runbooks" are flexible but manual. This bridges the gap.

## Core Toolset
*   **shell**: For observing `.bash_history` and executing approved runbooks.
*   **memory**: To build the "Symptom -> Diagnosis -> Solution" graph.
*   **filesystem**: To store the human-readable Markdown artifacts (Drafts/Approved).
*   **grep**: To categorize commands and link them to codebase context.

## Architecture

### 1. The Observer Loop (Passive)
*   **Trigger**: File system watcher on `~/.bash_history` (or shell hook).
*   **Action**: 
    1. Reads new lines.
    2. Groups them into "Sessions" based on time gaps (>5 min).
    3. Filters out noise (`ls`, `cd`, `whoami`).
    4. Creates a `Session Object` in Memory.

### 2. The Scribe Loop (Generative)
*   **Trigger**: End of a Session.
*   **Action**:
    1. Analyzes the command chain.
    2. Uses `grep` to find referenced files (e.g., `vim nginx.conf` -> agent reads `nginx.conf` diff).
    3. Generates a **Draft Runbook** in `workspace/runbooks/drafts/incident_YYYYMMDD.md`.
    4. Format:
       ```markdown
       # Incident Report: [Timestamp]
       **Detected Symptom**: (User inputs this or agent infers from logs)
       **Fix Sequence**:
       1. `systemctl stop app`
       2. `rm /var/cache/app.lock`
       3. `systemctl start app`
       ```

### 3. The Operator Loop (Active)
*   **Trigger**: User moves a draft to `workspace/runbooks/approved/` OR a matching Log Alert occurs.
*   **Action**:
    1. If Human Triggered: Parses the Markdown, extracts code blocks, and executes them safely.
    2. If Log Triggered (Advanced): Matches log error -> Memory Graph -> Approved Runbook -> Execute.

## Persistence Strategy (Hybrid)
*   **Memory Graph**: Stores the *logic* and *relationships* (e.g., "This error matches this Runbook").
    *   Nodes: `ErrorPattern`, `Runbook`, `Command`.
    *   Edges: `ErrorPattern -> mitigated_by -> Runbook`.
*   **Filesystem**: Stores the *content* and *interface*. The Markdown files are the "UI" for the human to edit/approve.

## Autonomy Level
**Level 3: Conditional Autonomy**.
*   **Drafting**: Fully Autonomous.
*   **Execution**: Human-in-the-loop (at first) -> Graduating to Fully Autonomous for "High Confidence" runbooks.

## Key Insight
**"Descriptive to Prescriptive Cycle"**: Most automation tools force you to define the prescription first. The Reifier accepts that *description* (history) comes first, and uses the agent to formalize it into a *prescription* (runbook), effectively "hardening" ephemeral shell commands into permanent assets.

## Failure Modes & Recovery
1.  **Dangerous Commands**: Agent might capture `rm -rf *`.
    *   *Recovery*: The "Draft" phase is read-only. The "Approved" phase requires human review. The Execution phase checks a "Blacklist" of dangerous patterns.
2.  **Context Missing**: User fixed it in the web UI, not shell.
    *   *Recovery*: Agent prompts "I saw no shell commands. Did you fix this via GUI?" in the draft note.

## Example Scenario
1.  **Crisis**: Database locks up.
2.  **Action**: Engineer runs 15 commands, checking logs, restarting processes.
3.  **Aftermath**: Engineer grabs coffee.
4.  **Agent**: "I noticed a flurry of activity. I've drafted `runbooks/drafts/db_lockup_fix.md` with the 3 key commands you ran (filtering out the 12 `ls` and `cat` commands)."
5.  **Review**: Engineer edits the title to "Fix DB Lockup", adds a note "Only do this if CPU > 90%", and moves to `approved/`.
6.  **Next Time**: Agent detects "CPU > 90%" (via log patrol) and suggests: "Execute 'Fix DB Lockup'?"
