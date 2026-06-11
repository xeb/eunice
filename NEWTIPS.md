# NEWTIPS: Long Task Planning for AI Agents

Source: [HN discussion (78 points)](https://news.ycombinator.com/item?id=48461635) of
["Build a Basic AI Agent from Scratch: Long Task Planning"](https://medium.com/@rogi23696/build-a-basic-ai-agent-from-scratch-long-task-planning-14e803f9bd6d)
by Roger Oriol (Medium, June 2026).

---

## Tips from the Article

### The core problem
LLMs are trained conversationally, so a bare agent "does not think long term, and it stops
working after the littlest progress" on long tasks. Fixing this requires giving the agent
explicit planning machinery: understand the objective, plan ahead, break work into steps,
track status, replan on obstacles, and verify completion before stopping.

### Tool 1: Scratchpad (private reasoning workspace)
- An **in-memory** scratchpad (`read_scratchpad` / `write_scratchpad`) gives the agent a
  place to think before acting; session-scoped, not written to disk.
- Before each tool call in a complex task, the agent updates the scratchpad with a
  5-step structure:
  1. Restate the goal in its own words
  2. Survey known facts (files seen, structure, constraints)
  3. Evaluate options with reasoning (e.g., "wrapping is safer than rewriting")
  4. Anticipate failure modes and how to diagnose them
  5. Commit to exactly **one** next action
- Structured scratchpad entries catch reasoning errors before they compound.

### Tool 2: To-do list (FSM-style task tracker)
- Tasks have five statuses: `pending`, `in_progress`, `done`, `cancelled`, `failed`.
- Enforced constraints — these are the trick, not the list itself:
  - Only **one task `in_progress` at a time**
  - Invalid statuses rejected; duplicate tasks prevented
  - Retry counts tracked, with a hard `RETRY_LIMIT` of 3 to prevent infinite loops
- API: `todo_append`, `todo_list(include_completed)`, `todo_update`.

### System prompt as a behavioral framework
- **Planning gate:** only invoke the full planning process for tasks with 3+ steps;
  simple tasks bypass it entirely.
- **Workflow discipline:** mark a step `in_progress` before starting; complete tasks
  immediately rather than batching; call `todo_list` between steps to assess remaining work;
  cancel tasks that no longer apply.
- **Working-directory convention:** assume the current directory is the project root and
  explore from there instead of asking the user for paths.
- **Planning calls are internal bookkeeping, not responses to the user** — after a
  scratchpad/todo call the agent continues working immediately, never emitting empty messages.

### Replanning and recovery
After every tool result, compare outcome vs. expectation. On errors or surprises:
1. Diagnose in the scratchpad — recoverable or fundamental?
2. Mark the task `failed` via `todo_update`
3. Pick a recovery action: **retry** (fix the input), **replace** (wrong approach), or
   **reorder** (new urgency)
4. If the retry limit is hit, **escalate to the user with a clear diagnosis** instead of looping

Even on success, if a tool reveals new information, pause and reassess all pending items.

### Done detection (three-part verification before stopping)
1. **Structural:** `todo_list` shows no `pending`, `in_progress`, or `failed` items
2. **Verification:** test the output against the original goal (run tests, build, etc.)
3. **Uncertainty check:** scan the scratchpad for unresolved questions; treat `cancelled`
   tasks differently from `done`

If any check fails, re-enter the planning loop.

### Proof it works
Tested on an Eleventy→Hugo static-site migration: the agent scratchpadded an approach,
created four todos (inspect, map, implement, verify), executed sequentially, and verified
with `hugo --minify` — producing the Hugo config, six layout templates, migrated content,
and updated build scripts.

### Stated next step
Human-in-the-loop checkpoints for potentially destructive actions — an unattended agent
"might be editing files and running commands it isn't supposed to."

---

## Tips from the Top HN Comments

### athrowaway3z — keep planning machinery minimal (most substantive comment)
Tried every planning scheme (AGENTS.md guides, `./dev/` plan files, todo-list tools,
SQLite tracking) and found **none worth the overhead with modern models** — a year ago
models needed reminding; today they can follow a plan from plain text. Their escalation
ladder, in order of task complexity:
1. Just tell an agent to do it
2. Tell it to make a plan, then execute it
3. Make a plan → write it to a file → have a **subagent review it** → execute
4. Supervisor mode: the agent supervises subagents that implement phases, rolling context
   over with a `handoff.md` while the supervisor drives the task to completion

The latter two are kept as prepared prompts injectable with a few keystrokes. Verdict:
checklists with checkboxes are optional polish; dedicated planning tools don't pay off
when the agent already has file read/write in context.

### manishsharan — why delegate planning at all?
Asks why humans shouldn't do the planning: figure out the architecture, break it into
small tasks, and have the agent execute them — like a tech lead delegating to developers.
You retain understanding of how the system works and can extend it.

### jdw64 — honest assessment of the article's design
- **Strengths:** the design forces chain-of-thought as a memory buffer, the FSM-style
  todo list, and a good retry/recovery strategy.
- **Weaknesses:** the business logic lives in the prompt rather than in Python code, and
  there's no parallel execution.
- **For production:** use separate (persistent) storage instead of in-memory, and more
  carefully verify tool constraints and the actual scope limits of the tools — but as a
  single-run teaching script it's good enough.

### hilariously — "recovery" doesn't survive a crash
The in-memory scratchpad and todo list vanish if the process dies, so the system isn't
recoverable in the crash sense — the article's recovery only covers LLM retries within a
run. (jdw64 agreed; persistent storage fixes this.)

### Thread meta (low signal)
Much of the thread was complaints about Medium as a platform and content quality rather
than technique — the substantive engineering discussion is captured above.

---

## Quick Cheat Sheet

| Trick | Why |
|---|---|
| One task `in_progress` at a time | Prevents the agent scattering effort |
| Hard retry limit (3) + escalate with diagnosis | Prevents infinite loops |
| Scratchpad before every tool call (goal → facts → options → risks → one action) | Catches reasoning errors early |
| 3-part done check (todos empty + verified output + no open questions) | Stops premature "done" claims |
| Skip planning for tasks under 3 steps | Overhead isn't free |
| Persist plan state to disk, not memory | Survives crashes (HN consensus fix) |
| Scale ceremony to complexity (direct → plan → reviewed plan → supervisor+subagents) | Modern models need less scaffolding than you think |
