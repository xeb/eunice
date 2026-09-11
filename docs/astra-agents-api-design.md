# Astra and OpenAI Agents API integration

Implemented against the public OpenAI documentation reviewed September 11, 2026. The default runtime remains Eunice. Protocol fixtures and local integration tests run without credentials; the paid live smoke test is opt-in.

## Two independent choices

`--model astra` resolves to `gpt-6-astra`. With the default `--runtime eunice`, Astra uses the Responses API while Eunice runs its existing agent loop. Astra tool calling requires Responses. Other providers retain their existing transports. [Astra guide](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-6-astra)

`--runtime openai-agents` selects OpenAI's managed session loop and defaults to Astra when no model is supplied. It also works with `--chat`, `--webapp`, and `--install`. This runtime requires an OpenAI model and an `OPENAI_API_KEY` with access to the Agents API. Authorization errors are surfaced without falling back to another runtime.

```bash
eunice --model astra "Inspect this repository"
eunice --runtime openai-agents --model astra "Inspect this repository"
eunice --webapp --runtime openai-agents --model astra
```

`--agents` continues selecting the scheduled-agent TOML file. Each entry can override the server runtime; omission inherits the server's setting. The web editor and systemd unit renderer preserve runtime selection.

```toml
[[agent]]
name = "daily-review"
schedule = "0 9 * * *"
runtime = "openai-agents"
model = "astra"
prompt = "Review recent changes and report findings."
working_dir = "/path/to/project"
```

## Implementation

| Component | Responsibility |
| --- | --- |
| `src/runtime.rs` | Runtime selection, conversation-owned managed state, checkpoint interface |
| `src/openai/responses.rs` | Native request and response conversion; portable tool/display projection |
| `src/openai/sse.rs` | Incremental UTF-8 SSE framing |
| `src/openai/agents.rs` | Managed lifecycle, action journal, local function bridge, cancellation and recovery |
| `src/client.rs` | Authentication, HTTP transport, Astra routing |
| `src/webapp/handlers.rs` | Session leases, browser events, managed checkpoints and restart observers |
| `src/webapp/persistence.rs` | Transactional transcript and managed-state snapshots, revision checks |
| `src/webapp/scheduler.rs` | Runtime inheritance/overrides, pending-run protection |

The binary and library retain separate module trees. The shared runtime and OpenAI modules have no dependency on Axum or `crate::webapp`.

## Astra in the Eunice runtime

Requests use `/v1/responses`, `store: false`, `include: ["reasoning.encrypted_content"]`, flat function definitions with `strict: false`, and native context compaction at 800,000 tokens. Unsupported sampling fields are omitted. Supported reasoning efforts are `low`, `medium`, `high`, `xhigh`, and `max`; omission uses the provider default.

The complete ordered output items are preserved alongside the portable assistant message: encrypted reasoning, message phase, function item IDs, call IDs and compaction items survive follow-ups and SQLite reload. Function results refer to `call_id`. After native compaction, the next request starts at the latest compaction item; the visible transcript remains intact. Generic Eunice history trimming is disabled for this path. Switching models projects history into portable messages and removes the previous model's private protocol state.

Streaming displays text deltas, but tool calls are accepted only from a validated `response.completed`. Incomplete responses, malformed function arguments, unsupported output item types, and EOF before completion return errors without executing partial tools. [Responses migration](https://developers.openai.com/api/docs/guides/migrate-to-responses), [reasoning state](https://developers.openai.com/api/docs/guides/reasoning), [compaction](https://developers.openai.com/api/docs/guides/compaction)

Astra cost estimates apply pricing tiers per request, including cached input and cache writes. The long-context threshold is not applied to an accumulated conversation total. [Model and pricing](https://developers.openai.com/api/docs/models/gpt-6-astra)

## Managed lifecycle and tools

Managed requests use `/v1/agents/sessions` and `OpenAI-Beta: agents=v1`. Creation supplies initial input and inline agent configuration, `environment: {"type":"none"}`, local function definitions, and disabled multi-agent execution. Bash, Read, Write, Skill and get_output run on the machine running Eunice, in its working directory or the scheduled agent's configured directory. Hosted sandbox and self-hosted executor environments are outside this implementation. [Architecture](https://developers.openai.com/api/docs/guides/agents-api/architecture), [functions](https://developers.openai.com/api/docs/guides/agents-api/tools/functions)

Follow-ups send only the new user input to the existing remote session. Runtime, resolved model, endpoint, effort and working directory are pinned. An existing local web conversation cannot silently become a managed conversation; start a new session when changing runtime or model.

The observer polls durable session snapshots and paginated items/turns every 750 ms. Completed messages and tool progress are translated to CLI display events or the existing browser SSE channel. Managed text currently appears as completed messages, rather than token deltas. Polling deliberately uses the same reconciliation path for ordinary execution and restart; it does not depend on event-stream replay. An idle session or a completed subagent does not imply root-turn success. [Sessions](https://developers.openai.com/api/docs/guides/agents-api/sessions), [events and recovery](https://developers.openai.com/api/docs/guides/agents-api/sessions/events)

Only current `required_actions` belonging to the active root turn trigger tools. Each `(turn_id, call_id)` is journaled before execution, then its result and full output are saved before submission. Repeated pending actions resend the saved result. A claim with no committed result has an uncertain outcome and is never automatically re-executed. This handles acknowledgement loss without promising exactly-once side effects across a process crash. Large output previews retain a durable `get_output` reference.

Stateful POSTs are not automatically retried. Definite input rejection permits a later explicit attempt; ambiguous creation/submission errors preserve uncertainty. An ambiguous creation may require inspecting the OpenAI dashboard because Eunice did not receive the remote ID. Authentication failures are reported directly. GET observation retries transient transport/server failures with bounded backoff.

Cancellation drops local tool work, sends `agent.session.input.cancel`, then waits for the terminal root outcome. A failed or unconfirmed cancellation remains pending and recoverable. Web cancellation is scoped to the session owner. Creating a new local conversation preserves the previous session; deleting an inactive local record does not delete the remote session. Pending managed records cannot be deleted locally.

## Persistence and scheduling

SQLite stores a runtime binding, remote session state, completed-turn/item IDs, and tool journal. Managed state and the canonical transcript are committed together under an optimistic revision check. An exclusive in-process session lease prevents concurrent tool execution. Transcript snapshots also fix an existing SQLite bug that duplicated prior messages on each new turn.

Startup keeps managed runs available for reconciliation while retaining interrupted-run handling for local runs. Recovery uses saved configuration and reacquires the per-agent run slot. A pending remote run prevents another scheduled run even after its observer failed. Run only one Eunice server per `sessions.db`; revision checks are a safeguard, not a distributed leader-election mechanism.

CLI conversations and `--no-persist` keep managed state in memory and cannot recover it after process exit. Remote sessions still have OpenAI's retention policy. Agents API sessions require US data residency and are not compatible with Zero Data Retention. [Agents API retention](https://developers.openai.com/api/docs/guides/agents-api/overview)

Managed usage is counted once per completed root turn when supplied. Aggregate turn totals do not reveal per-request context tiers or cache writes, so managed cost is reported as unavailable (`estimated_cost: null` in browser events). Consult OpenAI billing for actual charges. [Usage contract](https://developers.openai.com/api/docs/guides/agents-api/observability)

## Tests

Run the complete offline suite with `cargo test`. New coverage includes:

- Responses wire format, encrypted reasoning and phase replay, same-name tool calls with distinct IDs, compaction, effort validation, refusal and usage parsing.
- SSE split at every byte boundary, Unicode, multi-line events, malformed/incomplete streams and tool execution only after completion.
- Real Read/Write/Bash calls against a scripted local HTTP server, follow-up session reuse, paginated root outcomes, unknown functions, rejected and ambiguous requests, cancellation, repeated actions and uncertain claims.
- Web execution with SQLite restart after a lost tool acknowledgement; duplicate query locking, cancellation ownership, transactional revision checks, history round trips and runtime pinning.
- CLI argument parsing, TOML/editor round trips, runtime/provider validation, scheduler context selection, daemon rendering and per-request Astra pricing.

To run the paid live smoke test after obtaining an authorized key:

```bash
EUNICE_LIVE_OPENAI=1 cargo test --test openai_runtime live_openai_two_runtimes -- --ignored --nocapture
```

It reads a fixture in a temporary directory through each runtime. This test is excluded from default runs, and has not been run during implementation. Live API compatibility and account access still need that validation.
