use super::handlers::{run_agent_with_events, EventSender, RunContext, SseEvent};
use super::persistence::{SessionMetadata, SessionStorage};
use super::server::AppState;
use crate::agents::{
    detect_provider_isolated, prompt_preview, restricts_both_day_fields, AgentsConfig, LoadedAgent,
};
use crate::client::Client;
use crate::models::{Message, ProviderInfo};
use crate::tools::ToolRegistry;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, RwLock};

/// Longest single sleep between schedule evaluations, so a long idle period still
/// re-checks the clock and shutdown stays responsive.
const MAX_SLEEP: Duration = Duration::from_secs(60);

/// How long a timed-out run gets to unwind and persist its transcript before it
/// is abandoned outright.
const TIMEOUT_GRACE: Duration = Duration::from_secs(30);

/// Characters of the prompt shown in the read-only agents UI.
const PREVIEW_CHARS: usize = 240;

/// Recent sessions returned per agent by `/api/agents`.
const RECENT_SESSIONS: usize = 10;

fn log(message: &str) {
    let timestamp = Local::now().format("%H:%M:%S%.3f");
    println!("[{}] [scheduler] {}", timestamp, message);
}

fn now_unix() -> i64 {
    Local::now().timestamp()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Default)]
pub struct RunState {
    pub running: bool,
    pub last_run_at: Option<i64>,
    pub last_status: Option<RunStatus>,
    pub last_session_id: Option<String>,
    pub last_error: Option<String>,
}

/// Per-agent execution context, prebuilt at startup. `client` is present only when
/// the agent resolves to a different model than the server, `tool_registry` only
/// when the agent sets `working_dir`.
struct AgentContext {
    client: Option<(Arc<Client>, Arc<ProviderInfo>)>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

/// Loaded agents plus their live run state. Lives in AppState.
pub struct AgentRegistry {
    pub config: AgentsConfig,
    state: RwLock<HashMap<String, RunState>>,
    contexts: HashMap<String, AgentContext>,
}

#[derive(Serialize)]
pub struct AgentStatus {
    pub name: String,
    pub schedule: String,
    pub model: Option<String>,
    pub enabled: bool,
    pub prompt_preview: String,
    pub working_dir: Option<String>,
    pub timeout_secs: u64,
    /// A run is in flight right now. `last_status` describes the previous run, so
    /// without this the UI cannot tell "currently running" from "idle".
    pub running: bool,
    pub next_run_at: Option<i64>,
    pub last_run_at: Option<i64>,
    pub last_status: Option<RunStatus>,
    pub last_session_id: Option<String>,
    pub last_error: Option<String>,
    pub recent_sessions: Vec<SessionMetadata>,
}

#[derive(Serialize)]
pub struct AgentsResponse {
    pub agents_file: Option<String>,
    pub agents: Vec<AgentStatus>,
    pub server_model: String,
}

impl AgentRegistry {
    /// Build from a validated config. Constructs a per-agent Client only for agents whose
    /// model differs from `server_model`, and a per-agent ToolRegistry only for agents
    /// with a `working_dir`. Agents needing neither share the server's.
    pub fn new(config: AgentsConfig, server_model: &str) -> Result<Self> {
        let mut contexts: HashMap<String, AgentContext> = HashMap::new();

        for agent in &config.agents {
            let client = match &agent.model {
                Some(model) if model != server_model => {
                    let info = detect_provider_isolated(model).map_err(|e| {
                        anyhow!("agent '{}': unknown model '{}': {}", agent.name, model, e)
                    })?;
                    if info.resolved_model == server_model {
                        None
                    } else {
                        let client = Client::new(&info).map_err(|e| {
                            anyhow!("agent '{}': could not create client: {}", agent.name, e)
                        })?;
                        Some((Arc::new(client), Arc::new(info)))
                    }
                }
                _ => None,
            };

            let tool_registry = agent
                .working_dir
                .as_ref()
                .map(|dir| Arc::new(ToolRegistry::with_cwd(Some(dir.clone()))));

            if client.is_some() || tool_registry.is_some() {
                contexts.insert(
                    agent.name.clone(),
                    AgentContext {
                        client,
                        tool_registry,
                    },
                );
            }
        }

        Ok(Self {
            config,
            state: RwLock::new(HashMap::new()),
            contexts,
        })
    }

    /// Snapshot every agent for `/api/agents`. Takes storage because recent sessions
    /// live there, not in the registry.
    pub async fn snapshot(&self, storage: &SessionStorage) -> Vec<AgentStatus> {
        let states = self.state.read().await.clone();
        let mut out = Vec::with_capacity(self.config.agents.len());

        for agent in &self.config.agents {
            let run = states.get(&agent.name).cloned().unwrap_or_default();
            let recent_sessions = storage
                .list_agent_sessions(&agent.name, RECENT_SESSIONS)
                .await
                .unwrap_or_default();

            out.push(AgentStatus {
                name: agent.name.clone(),
                schedule: agent.schedule_expr.clone(),
                model: agent.model.clone(),
                enabled: agent.enabled,
                prompt_preview: prompt_preview(&agent.prompt, PREVIEW_CHARS),
                working_dir: agent
                    .working_dir
                    .as_ref()
                    .map(|d| d.display().to_string()),
                timeout_secs: agent.timeout_secs,
                running: run.running,
                next_run_at: next_run_at(agent),
                last_run_at: run.last_run_at,
                last_status: run.last_status,
                last_session_id: run.last_session_id,
                last_error: run.last_error,
                recent_sessions,
            });
        }

        out
    }

    /// Claim the run slot for an agent. Returns false when a previous run is still in
    /// flight; that tick is recorded as skipped rather than queued.
    async fn begin_run(&self, name: &str) -> bool {
        let mut states = self.state.write().await;
        let entry = states.entry(name.to_string()).or_default();

        if entry.running {
            entry.last_run_at = Some(now_unix());
            entry.last_status = Some(RunStatus::Skipped);
            return false;
        }

        entry.running = true;
        entry.last_status = Some(RunStatus::Running);
        entry.last_error = None;
        true
    }

    async fn finish_run(
        &self,
        name: &str,
        status: RunStatus,
        session_id: Option<String>,
        error: Option<String>,
    ) {
        let mut states = self.state.write().await;
        let entry = states.entry(name.to_string()).or_default();
        entry.last_run_at = Some(now_unix());
        entry.last_status = Some(status);
        entry.last_session_id = session_id;
        entry.last_error = error;
    }

    fn run_context(&self, name: &str, state: &AppState) -> Option<RunContext> {
        let ctx = self.contexts.get(name)?;

        let (client, provider_info) = match &ctx.client {
            Some((client, info)) => (client.clone(), info.clone()),
            None => (
                state.client.clone(),
                Arc::new(state.provider_info.clone()),
            ),
        };

        Some(RunContext {
            client,
            provider_info,
            tool_registry: ctx
                .tool_registry
                .clone()
                .unwrap_or_else(|| state.tool_registry.clone()),
        })
    }
}

/// Next fire time for a single agent, or None when it is disabled.
fn next_run_at(agent: &LoadedAgent) -> Option<i64> {
    if !agent.enabled {
        return None;
    }
    agent
        .schedule
        .upcoming(Local)
        .next()
        .map(|dt| dt.timestamp())
}

/// Soonest occurrence strictly after `after` across all enabled agents.
fn next_occurrence(agents: &[LoadedAgent], after: &DateTime<Local>) -> Option<DateTime<Local>> {
    agents
        .iter()
        .filter(|a| a.enabled)
        .filter_map(|a| a.schedule.after(after).next())
        .min()
}

/// Enabled agents with an occurrence in `(last_tick, now]`.
fn due_agents<'a>(
    agents: &'a [LoadedAgent],
    last_tick: &DateTime<Local>,
    now: &DateTime<Local>,
) -> Vec<&'a LoadedAgent> {
    agents
        .iter()
        .filter(|a| a.enabled)
        .filter(|a| {
            a.schedule
                .after(last_tick)
                .next()
                .is_some_and(|next| next <= *now)
        })
        .collect()
}

/// Clears `running` even when the run panics or is dropped mid-flight by the timeout.
struct RunGuard {
    registry: Arc<AgentRegistry>,
    name: String,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        // The state lock is async, so the clear has to happen on the runtime. During
        // runtime shutdown there is nothing left to clear.
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let registry = self.registry.clone();
        let name = std::mem::take(&mut self.name);
        handle.spawn(async move {
            let mut states = registry.state.write().await;
            if let Some(entry) = states.get_mut(&name) {
                entry.running = false;
            }
        });
    }
}

/// Spawn the cron loop. No-op when `state.agents` is None.
pub fn spawn(state: Arc<AppState>) {
    let Some(registry) = state.agents.clone() else {
        return;
    };

    let enabled = registry.config.agents.iter().filter(|a| a.enabled).count();
    if enabled == 0 {
        log("no enabled agents, scheduler idle");
        return;
    }

    log(&format!("watching {} enabled agent(s)", enabled));
    // Log the translated expression too: agents.toml takes standard Unix cron, but
    // the cron crate wants seconds-first and numbers day-of-week 1=Sunday, so the
    // day field is rewritten. Showing both makes a surprising firing day diagnosable.
    for a in registry.config.agents.iter().filter(|a| a.enabled) {
        log(&format!(
            "  {} — \"{}\" (as \"{}\")",
            a.name, a.schedule_expr, a.schedule_normalized
        ));
        if restricts_both_day_fields(&a.schedule_expr) {
            log(&format!(
                "  WARNING: {} restricts both day-of-month and day-of-week. Unix cron would fire on \
                 either; this fires only when both match. Split it into two agents if you meant \"or\".",
                a.name
            ));
        }
    }
    tokio::spawn(async move { run_loop(state, registry).await });
}

async fn run_loop(state: Arc<AppState>, registry: Arc<AgentRegistry>) {
    let mut last_tick = Local::now();

    loop {
        let Some(next) = next_occurrence(&registry.config.agents, &last_tick) else {
            log("no further scheduled occurrences, scheduler stopping");
            return;
        };

        let now = Local::now();
        let wait = (next - now).to_std().unwrap_or(Duration::ZERO);
        tokio::time::sleep(wait.min(MAX_SLEEP)).await;

        let now = Local::now();
        for agent in due_agents(&registry.config.agents, &last_tick, &now) {
            if registry.begin_run(&agent.name).await {
                let state = state.clone();
                let registry = registry.clone();
                let agent = agent.clone();
                tokio::spawn(async move {
                    let _guard = RunGuard {
                        registry: registry.clone(),
                        name: agent.name.clone(),
                    };
                    execute_run(state, registry, agent).await;
                });
            } else {
                log(&format!(
                    "[{}] previous run still in flight, skipping this tick",
                    agent.name
                ));
            }
        }

        // Never let the watermark move backwards: a backward wall-clock step (NTP
        // correction, DST) would otherwise re-fire occurrences that already ran.
        if now > last_tick {
            last_tick = now;
        }
    }
}

async fn execute_run(state: Arc<AppState>, registry: Arc<AgentRegistry>, agent: LoadedAgent) {
    let session = match state.storage.create_agent_session(&agent.name).await {
        Ok(session) => session,
        Err(e) => {
            log(&format!("[{}] failed to create session: {}", agent.name, e));
            registry
                .finish_run(&agent.name, RunStatus::Failed, None, Some(e.to_string()))
                .await;
            return;
        }
    };

    log(&format!(
        "[{}] run starting in session {} ({})",
        agent.name,
        &session.id[..8.min(session.id.len())],
        session.name
    ));

    // Publishing the broadcast sender is what lets a browser attach to an in-flight
    // scheduled run through /api/session/events.
    let (broadcast_tx, _) = broadcast::channel::<SseEvent>(100);
    state.storage.clear_runtime_events(&session.id).await;
    state
        .storage
        .set_runtime_state(&session.id, Vec::new(), Some(broadcast_tx.clone()), true)
        .await;

    // No HTTP client is attached to this channel, and EventSender::send awaits
    // tx.send, so the receiver must be drained or the run stalls at 100 events.
    let (tx, mut rx) = mpsc::channel::<SseEvent>(100);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    // Cancellation stays local; storing it in state.cancel_tx would detach a
    // concurrent interactive query from /api/cancel.
    let (cancel_tx, cancel_rx) = watch::channel(false);

    let event_sender = EventSender::new(
        tx,
        state.clone(),
        session.id.clone(),
        broadcast_tx,
    );
    let run_ctx = registry.run_context(&agent.name, &state);

    let run = run_agent_with_events(
        state.clone(),
        agent.prompt.clone(),
        session.id.clone(),
        session.name.clone(),
        event_sender,
        cancel_rx,
        run_ctx,
        false,
    );
    tokio::pin!(run);

    // On timeout, ask the loop to stop rather than dropping the future outright.
    // The loop writes the whole transcript in one batch after it exits, and aborts
    // its thinking timer there too, so a dropped future would lose the run's entire
    // history and leak that timer for the life of the process.
    let outcome = tokio::select! {
        result = &mut run => Some(result),
        _ = tokio::time::sleep(Duration::from_secs(agent.timeout_secs)) => None,
    };

    let outcome = match outcome {
        Some(result) => result,
        None => {
            log(&format!(
                "[{}] timed out after {}s, winding the run down",
                agent.name, agent.timeout_secs
            ));
            let _ = cancel_tx.send(true);
            // The loop only checks for cancellation between iterations, so a run
            // parked in a long tool call or API request may not return promptly.
            match tokio::time::timeout(TIMEOUT_GRACE, &mut run).await {
                Ok(_) => Err(format!("timed out after {}s", agent.timeout_secs)),
                Err(_) => {
                    log(&format!(
                        "[{}] did not wind down within {}s, abandoning the run",
                        agent.name,
                        TIMEOUT_GRACE.as_secs()
                    ));
                    // Abandoned mid-flight, so the loop never cleared this itself.
                    state
                        .storage
                        .set_runtime_state(&session.id, Vec::new(), None, false)
                        .await;
                    Err(format!(
                        "timed out after {}s and did not stop cleanly",
                        agent.timeout_secs
                    ))
                }
            }
        }
    };

    let failure = match outcome {
        Ok(()) => {
            log(&format!("[{}] run complete", agent.name));
            None
        }
        Err(message) => {
            log(&format!("[{}] run failed: {}", agent.name, message));
            Some(message)
        }
    };

    match failure {
        None => {
            registry
                .finish_run(&agent.name, RunStatus::Success, Some(session.id.clone()), None)
                .await;
        }
        Some(message) => {
            append_failure_note(&state, &session.id, &agent.name, &message).await;
            registry
                .finish_run(
                    &agent.name,
                    RunStatus::Failed,
                    Some(session.id.clone()),
                    Some(message),
                )
                .await;
        }
    }
}

/// Record a run failure in the session transcript itself. Runtime SSE events are
/// not persisted, so without this the stored history of a failed run ends mid-turn
/// with no indication of why.
async fn append_failure_note(state: &Arc<AppState>, session_id: &str, agent_name: &str, message: &str) {
    let note = Message::Assistant {
        content: Some(format!("Agent '{}' run failed: {}", agent_name, message)),
        tool_calls: None,
    };
    match serde_json::to_string(&note) {
        Ok(content) => {
            if let Err(e) = state
                .storage
                .append_event(session_id, "assistant_message", &content)
                .await
            {
                log(&format!("[{}] failed to record run failure: {}", agent_name, e));
            }
        }
        Err(e) => log(&format!("[{}] failed to encode run failure: {}", agent_name, e)),
    }

    // The two backends persist through different paths: SQLite derives history
    // from the events table above, while the in-memory store only keeps what
    // set_history writes. Without this second write, --no-persist drops the note.
    if !state.storage.is_persistent() {
        let mut history = state.storage.get_history(session_id).await.unwrap_or_default();
        history.push(note);
        let _ = state.storage.set_history(session_id, &history).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::normalize_cron;
    use chrono::{Duration as ChronoDuration, TimeZone};
    use std::str::FromStr;

    fn agent(name: &str, cron: &str, enabled: bool) -> LoadedAgent {
        let normalized = normalize_cron(cron).unwrap();
        LoadedAgent {
            name: name.to_string(),
            schedule_expr: cron.to_string(),
            schedule_normalized: normalized.clone(),
            schedule: cron::Schedule::from_str(&normalized).unwrap(),
            model: None,
            prompt: "do the thing".to_string(),
            enabled,
            timeout_secs: 600,
            working_dir: None,
        }
    }

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 7, 20, hour, minute, 0)
            .unwrap()
    }

    fn registry(agents: Vec<LoadedAgent>) -> AgentRegistry {
        AgentRegistry {
            config: AgentsConfig {
                source_path: std::path::PathBuf::from("/tmp/agents.toml"),
                agents,
            },
            state: RwLock::new(HashMap::new()),
            contexts: HashMap::new(),
        }
    }

    #[test]
    fn test_next_occurrence_picks_soonest_enabled_agent() {
        let agents = vec![
            agent("nine", "0 9 * * *", true),
            agent("hourly", "0 * * * *", true),
        ];
        let next = next_occurrence(&agents, &at(8, 30)).unwrap();
        assert_eq!(next, at(9, 0));
    }

    #[test]
    fn test_next_occurrence_ignores_disabled_agents() {
        let agents = vec![
            agent("hourly", "0 * * * *", false),
            agent("nine", "0 9 * * *", true),
        ];
        let next = next_occurrence(&agents, &at(8, 30)).unwrap();
        assert_eq!(next, at(9, 0));
    }

    #[test]
    fn test_next_run_at_is_none_when_disabled() {
        assert!(next_run_at(&agent("nine", "0 9 * * *", false)).is_none());
        assert!(next_run_at(&agent("nine", "0 9 * * *", true)).is_some());
    }

    #[test]
    fn test_due_agents_covers_the_elapsed_window_only() {
        let agents = vec![
            agent("nine", "0 9 * * *", true),
            agent("ten", "0 10 * * *", true),
            agent("nine-off", "0 9 * * *", false),
        ];

        let due = due_agents(&agents, &at(8, 59), &at(9, 0));
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["nine"]
        );

        assert!(due_agents(&agents, &at(9, 0), &at(9, 30)).is_empty());

        let due = due_agents(&agents, &at(8, 59), &at(10, 0));
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["nine", "ten"]
        );
    }

    #[test]
    fn test_due_agents_excludes_the_lower_bound_instant() {
        let agents = vec![agent("nine", "0 9 * * *", true)];
        // An occurrence exactly at last_tick was already fired on the previous pass.
        assert!(due_agents(&agents, &at(9, 0), &(at(9, 0) + ChronoDuration::seconds(1))).is_empty());
    }

    #[tokio::test]
    async fn test_begin_run_skips_while_a_run_is_in_flight() {
        let registry = registry(vec![agent("nine", "0 9 * * *", true)]);

        assert!(registry.begin_run("nine").await);
        assert_eq!(
            registry.state.read().await["nine"].last_status,
            Some(RunStatus::Running)
        );

        assert!(!registry.begin_run("nine").await);
        let states = registry.state.read().await;
        assert_eq!(states["nine"].last_status, Some(RunStatus::Skipped));
        assert!(states["nine"].running);
        assert!(states["nine"].last_run_at.is_some());
    }

    #[tokio::test]
    async fn test_run_guard_clears_running() {
        let registry = Arc::new(registry(vec![agent("nine", "0 9 * * *", true)]));
        assert!(registry.begin_run("nine").await);

        drop(RunGuard {
            registry: registry.clone(),
            name: "nine".to_string(),
        });
        tokio::task::yield_now().await;

        assert!(!registry.state.read().await["nine"].running);
    }

    #[tokio::test]
    async fn test_finish_run_records_outcome() {
        let registry = registry(vec![agent("nine", "0 9 * * *", true)]);
        registry.begin_run("nine").await;
        registry
            .finish_run(
                "nine",
                RunStatus::Failed,
                Some("abc123".to_string()),
                Some("timed out after 600s".to_string()),
            )
            .await;

        let states = registry.state.read().await;
        assert_eq!(states["nine"].last_status, Some(RunStatus::Failed));
        assert_eq!(states["nine"].last_session_id.as_deref(), Some("abc123"));
        assert_eq!(
            states["nine"].last_error.as_deref(),
            Some("timed out after 600s")
        );
    }

    #[test]
    fn test_run_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RunStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Success).unwrap(),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Skipped).unwrap(),
            "\"skipped\""
        );
    }

    #[test]
    fn test_agents_response_serializes_contract_field_names() {
        let response = AgentsResponse {
            agents_file: Some("/home/xeb/agents/agents.toml".to_string()),
            server_model: "claude-sonnet-4-5".to_string(),
            agents: vec![AgentStatus {
                name: "daily-digest".to_string(),
                schedule: "0 9 * * *".to_string(),
                model: Some("sonnet".to_string()),
                enabled: true,
                prompt_preview: "Summarize".to_string(),
                working_dir: Some("/home/xeb/p/myrepo".to_string()),
                timeout_secs: 600,
                running: false,
                next_run_at: Some(1784620800),
                last_run_at: Some(1784534400),
                last_status: Some(RunStatus::Success),
                last_session_id: Some("abc123".to_string()),
                last_error: None,
                recent_sessions: vec![SessionMetadata {
                    id: "abc123".to_string(),
                    name: "chrome-molly".to_string(),
                    turn_count: 4,
                    updated_at: 1784534460,
                    relative_time: "2h ago".to_string(),
                    agent_name: Some("daily-digest".to_string()),
                }],
            }],
        };

        let json: serde_json::Value = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json["agents_file"],
            "/home/xeb/agents/agents.toml"
        );
        assert_eq!(json["server_model"], "claude-sonnet-4-5");

        let agent = &json["agents"][0];
        for field in [
            "name",
            "schedule",
            "model",
            "enabled",
            "prompt_preview",
            "working_dir",
            "timeout_secs",
            "running",
            "next_run_at",
            "last_run_at",
            "last_status",
            "last_session_id",
            "last_error",
            "recent_sessions",
        ] {
            assert!(agent.get(field).is_some(), "missing field {}", field);
        }
        assert_eq!(agent.as_object().unwrap().len(), 14);
        assert_eq!(agent["last_status"], "success");
        assert!(agent["last_error"].is_null());
        assert_eq!(agent["recent_sessions"][0]["agent_name"], "daily-digest");

        let empty = AgentsResponse {
            agents_file: None,
            agents: vec![],
            server_model: "gpt-5".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&empty).unwrap(),
            r#"{"agents_file":null,"agents":[],"server_model":"gpt-5"}"#
        );
    }
}
