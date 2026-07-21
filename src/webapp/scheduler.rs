use super::handlers::{run_agent_with_events, EventSender, RunContext, SseEvent};
use super::persistence::{RunStatus, SessionMetadata, SessionStorage};
use super::server::AppState;
use crate::agents::{
    detect_provider_isolated, fingerprint, load_agents_file, prompt_preview,
    restricts_both_day_fields, AgentsConfig, LoadedAgent,
};
use crate::client::Client;
use crate::models::{Message, ProviderInfo};
use crate::tools::ToolRegistry;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, Mutex, MutexGuard, Notify, RwLock};

/// Longest single sleep between schedule evaluations, so a long idle period still
/// re-checks the clock and shutdown stays responsive.
const MAX_SLEEP: Duration = Duration::from_secs(60);

/// How often the watcher compares the on-disk config fingerprint with the loaded one.
const RELOAD_POLL: Duration = Duration::from_secs(3);

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

#[derive(Debug, Clone, Default)]
pub struct RunState {
    pub running: bool,
    pub last_run_at: Option<i64>,
    pub last_status: Option<RunStatus>,
    pub last_session_id: Option<String>,
    pub last_error: Option<String>,
    /// When this agent first became known, or when its schedule last changed. None for
    /// agents that were already loaded at startup, which must fire on their normal
    /// schedule; Some means occurrences at or before that instant predate the agent and
    /// are not back-fired.
    pub added_at: Option<i64>,
}

/// Per-agent execution context, built when a config is loaded. `client` is present
/// only when the agent resolves to a different model than the server, `tool_registry`
/// only when the agent sets `working_dir`; both fall back to the server's at run time.
#[derive(Clone)]
struct AgentContext {
    /// What this context was built from. A reload keeps the context as-is while these
    /// still match, because rebuilding one runs provider detection, which may probe
    /// Ollama over HTTP.
    model: Option<String>,
    working_dir: Option<PathBuf>,
    client: Option<(Arc<Client>, Arc<ProviderInfo>)>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

/// Loaded agents plus their live run state. Lives in AppState.
pub struct AgentRegistry {
    inner: RwLock<RegistryInner>,
    state: RwLock<HashMap<String, RunState>>,
    /// Wakes the scheduler loop the moment a reload lands, instead of leaving it
    /// asleep for up to MAX_SLEEP.
    reload: Notify,
    /// Serializes the read-modify-write of an edit. The fingerprint check only rejects
    /// a save that was *prepared* against stale text; without this, two saves that both
    /// passed that check interleave and the later write silently drops the earlier one.
    edit_lock: Mutex<()>,
    source_path: PathBuf,
    server_model: String,
}

struct RegistryInner {
    config: AgentsConfig,
    contexts: HashMap<String, AgentContext>,
    fingerprint: String,
    loaded_at: i64,
    /// Why the most recent reload attempt was rejected. Cleared on a successful reload.
    last_reload_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReloadStatus {
    pub fingerprint: String,
    pub loaded_at: i64,
    pub last_reload_error: Option<String>,
}

#[derive(Serialize)]
pub struct AgentStatus {
    pub name: String,
    pub schedule: String,
    pub model: Option<String>,
    pub enabled: bool,
    pub prompt_preview: String,
    /// Resolved path of the agent's `prompt_file`, for display. The full prompt text is
    /// deliberately not in the list response; `/api/agents/get` serves it.
    pub prompt_file: Option<String>,
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
    /// Whether the edit endpoints will accept a write: true whenever an agents file
    /// is configured. There is no host gating.
    pub editable: bool,
    /// Fingerprint the list was loaded with; a save carrying a stale one is rejected.
    pub fingerprint: String,
    pub loaded_at: i64,
    pub reload_error: Option<String>,
    /// IANA name of the server's local timezone (e.g. `America/Los_Angeles`). Cron fires
    /// in server-local time but the browser computes in its own, so the UI needs this to
    /// preview next-run times honestly. `None` when the zone cannot be determined — the
    /// client then falls back to browser-local and says so, rather than being told a
    /// default that may be wrong.
    pub server_timezone: Option<String>,
}

/// Server-local IANA timezone name, resolved once per process.
///
/// `iana_time_zone::get_timezone()` reads `/etc/localtime` (or the platform equivalent),
/// and `/api/agents` is polled every 30s by every open UI, so this is cached. The value
/// cannot meaningfully change while the process runs. Failure is cached as `None`.
pub fn server_timezone() -> Option<String> {
    static TZ: OnceLock<Option<String>> = OnceLock::new();
    TZ.get_or_init(|| iana_time_zone::get_timezone().ok())
        .clone()
}

impl AgentRegistry {
    /// Build from a validated config. Constructs a per-agent Client only for agents whose
    /// model differs from `server_model`, and a per-agent ToolRegistry only for agents
    /// with a `working_dir`. Agents needing neither share the server's.
    pub fn new(config: AgentsConfig, server_model: &str) -> Result<Self> {
        let contexts = build_contexts(&config.agents, server_model, &HashMap::new())?;
        let source_path = config.source_path.clone();
        let fingerprint = fingerprint(&source_path, &config.agents);

        Ok(Self {
            inner: RwLock::new(RegistryInner {
                config,
                contexts,
                fingerprint,
                loaded_at: now_unix(),
                last_reload_error: None,
            }),
            state: RwLock::new(HashMap::new()),
            reload: Notify::new(),
            edit_lock: Mutex::new(()),
            source_path,
            server_model: server_model.to_string(),
        })
    }

    /// Held for the whole of an edit, from the fingerprint check through `apply`.
    pub async fn lock_edits(&self) -> MutexGuard<'_, ()> {
        self.edit_lock.lock().await
    }

    /// Re-read agents.toml from disk. Returns Ok(true) when a new config was applied,
    /// Ok(false) when the fingerprint was unchanged.
    ///
    /// A config that fails validation is recorded in `last_reload_error` and DISCARDED —
    /// the running daemon keeps serving the previous config. This is deliberately the
    /// opposite of startup, where an invalid file aborts: once agents are running, a typo
    /// must not take the service down.
    pub async fn reload_from_disk(&self) -> Result<bool> {
        let (loaded_fingerprint, agents) = {
            let inner = self.inner.read().await;
            (inner.fingerprint.clone(), inner.config.agents.clone())
        };

        let on_disk = fingerprint(&self.source_path, &agents);
        if on_disk == loaded_fingerprint {
            return Ok(false);
        }

        // Validation resolves models, which joins a thread that may make a blocking
        // HTTP probe, so it cannot run on a runtime worker.
        let path = self.source_path.clone();
        let loaded = tokio::task::spawn_blocking(move || {
            load_agents_file(&path, &|model| detect_provider_isolated(model).map(|_| ()))
        })
        .await
        .map_err(|e| anyhow!("agents reload task failed: {}", e))?;

        let config = match loaded {
            Ok(config) => config,
            Err(e) => {
                let message = e.to_string();
                log(&format!("config reload REJECTED: {}", message));
                log("keeping the previously loaded config; fix the file and it will be picked up");
                let mut inner = self.inner.write().await;
                inner.last_reload_error = Some(message);
                // Remember the rejected content so the same failure is not retried,
                // and re-logged, on every poll.
                inner.fingerprint = on_disk;
                return Ok(false);
            }
        };

        self.swap(config).await?;
        Ok(true)
    }

    /// Swap in an already-validated config (the UI save path, which has just written the file).
    pub async fn apply(&self, config: AgentsConfig) -> Result<()> {
        self.swap(config).await
    }

    /// Replace the live config. In-flight runs are left alone; only future scheduling
    /// reflects the new config.
    async fn swap(&self, config: AgentsConfig) -> Result<()> {
        let previous_contexts = self.inner.read().await.contexts.clone();

        // build_contexts resolves models, which joins a thread that may make a blocking
        // HTTP probe, so it cannot run on a runtime worker.
        let agents = config.agents.clone();
        let server_model = self.server_model.clone();
        let contexts = tokio::task::spawn_blocking(move || {
            build_contexts(&agents, &server_model, &previous_contexts)
        })
        .await
        .map_err(|e| anyhow!("context build task failed: {}", e))??;

        let new_fingerprint = fingerprint(&self.source_path, &config.agents);

        {
            let previous_schedules: HashMap<String, String> = self
                .inner
                .read()
                .await
                .config
                .agents
                .iter()
                .map(|a| (a.name.clone(), a.schedule_normalized.clone()))
                .collect();

            let now = now_unix();
            let mut states = self.state.write().await;
            states.retain(|name, _| config.agents.iter().any(|agent| &agent.name == name));

            for agent in &config.agents {
                // A rescheduled agent counts as newly arrived too: occurrences of the new
                // expression that fall in the window already elapsed belong to a schedule
                // that did not exist then.
                let arrived = previous_schedules
                    .get(&agent.name)
                    .is_none_or(|previous| *previous != agent.schedule_normalized);
                if arrived {
                    states.entry(agent.name.clone()).or_default().added_at = Some(now);
                }
            }
        }

        let previous_count = {
            let mut inner = self.inner.write().await;
            let previous_count = inner.config.agents.len();
            inner.config = config;
            inner.contexts = contexts;
            inner.fingerprint = new_fingerprint;
            inner.loaded_at = now_unix();
            inner.last_reload_error = None;
            previous_count
        };

        {
            let inner = self.inner.read().await;
            log(&format!(
                "config reloaded: {} agent(s) (was {})",
                inner.config.agents.len(),
                previous_count
            ));
            log_schedules(&inner.config.agents);
        }

        self.reload.notify_one();
        Ok(())
    }

    pub async fn status(&self) -> ReloadStatus {
        let inner = self.inner.read().await;
        ReloadStatus {
            fingerprint: inner.fingerprint.clone(),
            loaded_at: inner.loaded_at,
            last_reload_error: inner.last_reload_error.clone(),
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// Raw text of agents.toml as it stands on disk, for the edit path.
    pub async fn config_text(&self) -> Result<String> {
        tokio::fs::read_to_string(&self.source_path)
            .await
            .map_err(|e| {
                anyhow!(
                    "failed to read agents file '{}': {}",
                    self.source_path.display(),
                    e
                )
            })
    }

    /// Snapshot every agent for `/api/agents`. Takes storage because recent sessions
    /// live there, not in the registry.
    pub async fn snapshot(&self, storage: &SessionStorage) -> Vec<AgentStatus> {
        let states = self.state.read().await.clone();
        // Cloned out of the lock: the session lookups below await, and a reload must
        // not be blocked behind them.
        let agents = self.inner.read().await.config.agents.clone();
        let mut out = Vec::with_capacity(agents.len());

        for agent in &agents {
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
                prompt_file: agent
                    .prompt_file
                    .as_ref()
                    .map(|p| p.display().to_string()),
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

    /// Record a finished run. A run outlives a reload that deleted its agent, and the
    /// swap already dropped that agent's state, so an unknown name is a no-op rather
    /// than a resurrected entry.
    async fn finish_run(
        &self,
        name: &str,
        status: RunStatus,
        session_id: Option<String>,
        error: Option<String>,
    ) {
        let mut states = self.state.write().await;
        let Some(entry) = states.get_mut(name) else {
            return;
        };
        entry.last_run_at = Some(now_unix());
        entry.last_status = Some(status);
        entry.last_session_id = session_id;
        entry.last_error = error;
    }

    /// Resolve an agent's execution context. `None` means the agent is no longer
    /// configured — NOT "use the server defaults", which is what a context present but
    /// empty means. Callers must skip the run rather than fall back, or an agent with a
    /// `working_dir` would execute tools in the server's directory instead of its own.
    async fn run_context(&self, name: &str, state: &AppState) -> Option<RunContext> {
        let inner = self.inner.read().await;
        let ctx = inner.contexts.get(name)?;

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

/// Build one context per agent, carrying over any whose `model` and `working_dir` are
/// unchanged. Every agent gets an entry, including those that need neither a dedicated
/// client nor a dedicated tool registry, so a later reload can recognise them as
/// unchanged instead of re-running provider detection.
fn build_contexts(
    agents: &[LoadedAgent],
    server_model: &str,
    previous: &HashMap<String, AgentContext>,
) -> Result<HashMap<String, AgentContext>> {
    let mut contexts: HashMap<String, AgentContext> = HashMap::new();

    for agent in agents {
        if let Some(existing) = previous.get(&agent.name) {
            if existing.model == agent.model && existing.working_dir == agent.working_dir {
                contexts.insert(agent.name.clone(), existing.clone());
                continue;
            }
        }

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

        contexts.insert(
            agent.name.clone(),
            AgentContext {
                model: agent.model.clone(),
                working_dir: agent.working_dir.clone(),
                client,
                tool_registry,
            },
        );
    }

    Ok(contexts)
}

/// Log what is now scheduled. Startup and every reload go through here so the two
/// print the same thing.
fn log_schedules(agents: &[LoadedAgent]) {
    let enabled: Vec<&LoadedAgent> = agents.iter().filter(|a| a.enabled).collect();
    if enabled.is_empty() {
        log("no enabled agents, scheduler idle");
        return;
    }

    log(&format!("watching {} enabled agent(s)", enabled.len()));
    // Log the translated expression too: agents.toml takes standard Unix cron, but
    // the cron crate wants seconds-first and numbers day-of-week 1=Sunday, so the
    // day field is rewritten. Showing both makes a surprising firing day diagnosable.
    for a in enabled {
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

/// Enabled agents with an occurrence in `(last_tick, now]`, excluding any occurrence at
/// or before the instant the agent became known — an agent added or rescheduled part-way
/// through the window must not back-fire the occurrence that preceded it.
fn due_agents<'a>(
    agents: &'a [LoadedAgent],
    last_tick: &DateTime<Local>,
    now: &DateTime<Local>,
    added_at: &HashMap<String, i64>,
) -> Vec<&'a LoadedAgent> {
    agents
        .iter()
        .filter(|a| a.enabled)
        .filter(|a| {
            a.schedule.after(last_tick).next().is_some_and(|next| {
                next <= *now
                    && added_at
                        .get(&a.name)
                        .is_none_or(|added| next.timestamp() > *added)
            })
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

/// Spawn the cron loop, the config watcher and the SIGHUP handler. No-op when
/// `state.agents` is None.
///
/// The loop is spawned even with nothing enabled: hot reload means an agent added to
/// an empty file has to start scheduling without a restart.
pub fn spawn(state: Arc<AppState>) {
    let Some(registry) = state.agents.clone() else {
        return;
    };

    spawn_watcher(registry.clone());
    spawn_sighup(registry.clone());
    tokio::spawn(async move { run_loop(state, registry).await });
}

/// Poll the config fingerprint and reload when it changes.
fn spawn_watcher(registry: Arc<AgentRegistry>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(RELOAD_POLL).await;
            if let Err(e) = registry.reload_from_disk().await {
                log(&format!("config reload failed: {}", e));
            }
        }
    });
}

/// SIGHUP triggers an immediate reload, which is what systemd `ExecReload` sends.
#[cfg(unix)]
fn spawn_sighup(registry: Arc<AgentRegistry>) {
    tokio::spawn(async move {
        let mut hangup =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
                Ok(hangup) => hangup,
                Err(e) => {
                    log(&format!("SIGHUP reload unavailable: {}", e));
                    return;
                }
            };

        while hangup.recv().await.is_some() {
            log("SIGHUP received, re-reading the config");
            if let Err(e) = registry.reload_from_disk().await {
                log(&format!("config reload failed: {}", e));
            }
        }
    });
}

#[cfg(not(unix))]
fn spawn_sighup(_registry: Arc<AgentRegistry>) {}

async fn run_loop(state: Arc<AppState>, registry: Arc<AgentRegistry>) {
    log_schedules(&registry.inner.read().await.config.agents);

    let mut last_tick = Local::now();

    loop {
        let agents = registry.inner.read().await.config.agents.clone();

        match next_occurrence(&agents, &last_tick) {
            Some(next) => {
                let wait = (next - Local::now()).to_std().unwrap_or(Duration::ZERO);
                tokio::select! {
                    _ = tokio::time::sleep(wait.min(MAX_SLEEP)) => {}
                    _ = registry.reload.notified() => {}
                }
            }
            None => {
                // Nothing is scheduled, so nothing can be missed while parked here.
                // Waking with the watermark moved to now is what keeps an agent added
                // during the idle period from back-firing the whole gap.
                registry.reload.notified().await;
                last_tick = Local::now();
                continue;
            }
        }

        // Re-read: a reload may have replaced the agent set during the sleep.
        let agents = registry.inner.read().await.config.agents.clone();

        let added_at: HashMap<String, i64> = registry
            .state
            .read()
            .await
            .iter()
            .filter_map(|(name, run)| run.added_at.map(|at| (name.clone(), at)))
            .collect();

        let now = Local::now();
        for agent in due_agents(&agents, &last_tick, &now, &added_at) {
            // Captured before the run is claimed: a reload during the run would leave
            // run_context returning None, and running with the server's context instead
            // would execute the agent's tools in the wrong directory.
            let Some(run_ctx) = registry.run_context(&agent.name, &state).await else {
                log(&format!(
                    "[{}] no longer configured, skipping this tick",
                    agent.name
                ));
                continue;
            };

            if registry.begin_run(&agent.name).await {
                let state = state.clone();
                let registry = registry.clone();
                let agent = agent.clone();
                tokio::spawn(async move {
                    let _guard = RunGuard {
                        registry: registry.clone(),
                        name: agent.name.clone(),
                    };
                    execute_run(state, registry, agent, run_ctx).await;
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

async fn execute_run(
    state: Arc<AppState>,
    registry: Arc<AgentRegistry>,
    agent: LoadedAgent,
    run_ctx: RunContext,
) {
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
    let run = run_agent_with_events(
        state.clone(),
        agent.prompt.clone(),
        session.id.clone(),
        session.name.clone(),
        event_sender,
        cancel_rx,
        Some(run_ctx),
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
            record_run_status(&state, &session.id, &agent.name, RunStatus::Success).await;
            registry
                .finish_run(&agent.name, RunStatus::Success, Some(session.id.clone()), None)
                .await;
        }
        Some(message) => {
            append_failure_note(&state, &session.id, &agent.name, &message).await;
            record_run_status(&state, &session.id, &agent.name, RunStatus::Failed).await;
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

/// Persist a run's outcome on its session, so the UI can mark it after a restart.
/// The in-memory `RunState` only remembers the *latest* run per agent; this is
/// per-session and survives the process. A failure here is logged, not propagated:
/// the run itself already succeeded or failed on its own terms.
async fn record_run_status(
    state: &Arc<AppState>,
    session_id: &str,
    agent_name: &str,
    status: RunStatus,
) {
    if let Err(e) = state.storage.set_run_status(session_id, status).await {
        log(&format!(
            "[{}] failed to record run status {}: {}",
            agent_name,
            status.as_str(),
            e
        ));
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
    use tempfile::TempDir;

    fn agent(name: &str, cron: &str, enabled: bool) -> LoadedAgent {
        let normalized = normalize_cron(cron).unwrap();
        LoadedAgent {
            name: name.to_string(),
            schedule_expr: cron.to_string(),
            schedule_normalized: normalized.clone(),
            schedule: cron::Schedule::from_str(&normalized).unwrap(),
            model: None,
            prompt: "do the thing".to_string(),
            prompt_file: None,
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

    fn config(agents: Vec<LoadedAgent>) -> AgentsConfig {
        AgentsConfig {
            source_path: PathBuf::from("/tmp/agents.toml"),
            agents,
        }
    }

    fn registry(agents: Vec<LoadedAgent>) -> AgentRegistry {
        AgentRegistry {
            inner: RwLock::new(RegistryInner {
                config: config(agents),
                contexts: HashMap::new(),
                fingerprint: "seed".to_string(),
                loaded_at: 0,
                last_reload_error: None,
            }),
            state: RwLock::new(HashMap::new()),
            reload: Notify::new(),
            edit_lock: Mutex::new(()),
            source_path: PathBuf::from("/tmp/agents.toml"),
            server_model: "server-model".to_string(),
        }
    }

    /// A registry backed by a real agents.toml, for the reload paths.
    fn on_disk_registry(dir: &TempDir, body: &str) -> (PathBuf, AgentRegistry) {
        let path = dir.path().join("agents.toml");
        std::fs::write(&path, body).unwrap();
        let config = crate::agents::load_agents_file(&path, &|_| Ok(())).unwrap();
        let registry = AgentRegistry::new(config, "server-model").unwrap();
        (path, registry)
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

    /// Regression: `cron` 0.12 walked candidate dates through chrono's deprecated
    /// `ymd`, which *panics* when a local date is ambiguous. Zones that move the
    /// clock at midnight (Chile, Cuba, Lebanon) make the plainest schedule there is
    /// — daily at midnight — hit exactly that on the fall-back date. The panic fired
    /// inside the scheduler's own spawned task, so it never crashed the server: it
    /// silently killed the scheduler while the web UI kept serving normally, and
    /// agents just quietly stopped. Fixed by cron 0.17.
    ///
    /// `TZ` and `chrono::Local` are process-global, and chrono caches the zone on
    /// first use — setting `TZ` from inside a test is both order-dependent (the
    /// cache may already be warm) and permanently corrupting for every other test in
    /// the binary. So the check runs in a child process with `TZ` set at exec time.
    #[test]
    fn test_midnight_schedule_in_midnight_transition_zone_does_not_panic() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "webapp::scheduler::tests::midnight_transition_zone_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TZ", "America/Santiago")
            .output()
            .expect("re-exec the test binary");

        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);

        // Without this the test would pass vacuously if the filter ever stopped
        // matching (a rename, a moved module), which is the one way a
        // regression test like this can rot silently.
        assert!(
            stdout.contains("1 passed"),
            "child ran no case -- the filter matched nothing.\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            out.status.success(),
            "a midnight schedule panicked in a midnight-transition zone.\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }

    #[ignore = "driven by test_midnight_schedule_in_midnight_transition_zone_does_not_panic, which sets TZ"]
    #[test]
    fn midnight_transition_zone_child() {
        assert_eq!(
            std::env::var("TZ").ok().as_deref(),
            Some("America/Santiago"),
            "must be run by its parent, which sets TZ"
        );

        // Chile moves the clock back one hour at midnight on 2027-04-04, so the
        // local time 2027-04-04 00:00 happens twice (once at -03, once at -04) and
        // the date itself is ambiguous. This is the instant 0.12 panicked on.
        let agents = vec![agent("midnight", "0 0 * * *", true)];
        let before = Local.with_ymd_and_hms(2027, 4, 3, 12, 0, 0).unwrap();

        // The scheduler's own loop: repeatedly ask for the next occurrence, walking
        // straight through the ambiguous date rather than stopping short of it.
        let mut cursor = before;
        for _ in 0..4 {
            let next = next_occurrence(&agents, &cursor)
                .expect("a daily schedule always has a next occurrence");
            assert!(next > cursor, "occurrences must advance, got {next} after {cursor}");
            cursor = next;
        }
        assert!(
            cursor > Local.with_ymd_and_hms(2027, 4, 5, 0, 0, 0).unwrap(),
            "should have stepped past the transition, stopped at {cursor}"
        );

        // The /api/agents handler reads the same schedule through next_run_at.
        // (This one measures from "now", so it is a smoke check on the same
        // schedule object rather than a second crossing of the transition.)
        assert!(next_run_at(&agents[0]).is_some());
    }

    #[test]
    fn test_due_agents_covers_the_elapsed_window_only() {
        let agents = vec![
            agent("nine", "0 9 * * *", true),
            agent("ten", "0 10 * * *", true),
            agent("nine-off", "0 9 * * *", false),
        ];

        let due = due_agents(&agents, &at(8, 59), &at(9, 0), &HashMap::new());
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["nine"]
        );

        assert!(due_agents(&agents, &at(9, 0), &at(9, 30), &HashMap::new()).is_empty());

        let due = due_agents(&agents, &at(8, 59), &at(10, 0), &HashMap::new());
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["nine", "ten"]
        );
    }

    #[test]
    fn test_due_agents_excludes_the_lower_bound_instant() {
        let agents = vec![agent("nine", "0 9 * * *", true)];
        // An occurrence exactly at last_tick was already fired on the previous pass.
        assert!(due_agents(
            &agents,
            &at(9, 0),
            &(at(9, 0) + ChronoDuration::seconds(1)),
            &HashMap::new()
        )
        .is_empty());
    }

    #[test]
    fn test_due_agents_does_not_backfire_an_occurrence_predating_the_agent() {
        let agents = vec![agent("hourly", "0 * * * *", true)];
        let added = HashMap::from([("hourly".to_string(), at(10, 0).timestamp() + 30)]);

        // Added at 10:00:30, so the 10:00:00 occurrence predates it.
        assert!(due_agents(&agents, &at(9, 30), &at(10, 1), &added).is_empty());

        // The next occurrence, which does postdate it, still fires.
        let due = due_agents(&agents, &at(10, 30), &at(11, 0), &added);
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["hourly"]
        );

        // An agent with no recorded arrival was already loaded, so it is unaffected.
        let due = due_agents(&agents, &at(9, 30), &at(10, 1), &HashMap::new());
        assert_eq!(
            due.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["hourly"]
        );
    }

    #[tokio::test]
    async fn test_apply_marks_added_and_rescheduled_agents_but_not_untouched_ones() {
        let registry = registry(vec![
            agent("keep", "0 9 * * *", true),
            agent("moved", "0 9 * * *", true),
        ]);
        // Present since startup, so neither has an arrival instant yet.
        registry.begin_run("keep").await;
        registry.begin_run("moved").await;
        assert!(registry.state.read().await["keep"].added_at.is_none());

        registry
            .apply(config(vec![
                agent("keep", "0 9 * * *", true),
                agent("moved", "0 10 * * *", true),
                agent("added", "0 9 * * *", true),
            ]))
            .await
            .unwrap();

        let states = registry.state.read().await;
        assert!(states["keep"].added_at.is_none());
        assert!(states["moved"].added_at.is_some());
        assert!(states["added"].added_at.is_some());
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
    fn test_build_contexts_covers_every_agent() {
        let contexts = build_contexts(
            &[agent("plain", "0 9 * * *", true)],
            "server-model",
            &HashMap::new(),
        )
        .unwrap();

        assert!(contexts["plain"].client.is_none());
        assert!(contexts["plain"].tool_registry.is_none());
    }

    #[test]
    fn test_build_contexts_reuses_when_model_and_working_dir_are_unchanged() {
        let dir = TempDir::new().unwrap();
        let mut a = agent("a", "0 9 * * *", true);
        a.working_dir = Some(dir.path().to_path_buf());

        let first =
            build_contexts(std::slice::from_ref(&a), "server-model", &HashMap::new()).unwrap();
        // A schedule change must not cost a rebuild.
        let mut rescheduled = a.clone();
        rescheduled.schedule_expr = "0 10 * * *".to_string();
        let second =
            build_contexts(std::slice::from_ref(&rescheduled), "server-model", &first).unwrap();

        assert!(Arc::ptr_eq(
            first["a"].tool_registry.as_ref().unwrap(),
            second["a"].tool_registry.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_build_contexts_rebuilds_when_model_or_working_dir_changes() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        let mut a = agent("a", "0 9 * * *", true);
        a.working_dir = Some(dir.path().to_path_buf());

        let first =
            build_contexts(std::slice::from_ref(&a), "server-model", &HashMap::new()).unwrap();

        let mut moved = a.clone();
        moved.working_dir = Some(elsewhere.path().to_path_buf());
        let rebuilt =
            build_contexts(std::slice::from_ref(&moved), "server-model", &first).unwrap();
        assert!(!Arc::ptr_eq(
            first["a"].tool_registry.as_ref().unwrap(),
            rebuilt["a"].tool_registry.as_ref().unwrap()
        ));

        let mut remodelled = a.clone();
        // Equal to the server model, so this rebuild resolves nothing over the network.
        remodelled.model = Some("server-model".to_string());
        let rebuilt =
            build_contexts(std::slice::from_ref(&remodelled), "server-model", &first).unwrap();
        assert!(!Arc::ptr_eq(
            first["a"].tool_registry.as_ref().unwrap(),
            rebuilt["a"].tool_registry.as_ref().unwrap()
        ));
        assert_eq!(rebuilt["a"].model.as_deref(), Some("server-model"));
    }

    #[tokio::test]
    async fn test_apply_keeps_run_state_only_for_surviving_agents() {
        let registry = registry(vec![
            agent("keep", "0 9 * * *", true),
            agent("drop", "0 9 * * *", true),
        ]);
        registry.begin_run("keep").await;
        registry.begin_run("drop").await;
        registry
            .finish_run("keep", RunStatus::Success, Some("s1".to_string()), None)
            .await;

        registry
            .apply(config(vec![
                agent("keep", "0 10 * * *", true),
                agent("added", "0 9 * * *", true),
            ]))
            .await
            .unwrap();

        {
            let states = registry.state.read().await;
            assert_eq!(states["keep"].last_session_id.as_deref(), Some("s1"));
            assert!(states["keep"].running);
            assert!(!states.contains_key("drop"));
            // A new agent gets an entry only to record its arrival, with no run history.
            assert!(states["added"].last_status.is_none());
            assert!(states["added"].last_run_at.is_none());
        }

        let inner = registry.inner.read().await;
        assert_eq!(
            inner
                .config
                .agents
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            vec!["keep", "added"]
        );
        assert!(inner.contexts.contains_key("added"));
        assert!(!inner.contexts.contains_key("drop"));
    }

    #[tokio::test]
    async fn test_finish_run_ignores_an_agent_that_no_longer_exists() {
        let registry = registry(vec![agent("gone", "0 9 * * *", true)]);
        registry.begin_run("gone").await;

        registry.apply(config(Vec::new())).await.unwrap();
        assert!(registry.state.read().await.is_empty());

        // A run outliving its own deletion must not resurrect a phantom entry.
        registry
            .finish_run("gone", RunStatus::Success, Some("s1".to_string()), None)
            .await;
        assert!(registry.state.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_apply_leaves_a_wakeup_for_a_loop_not_yet_parked() {
        let registry = registry(Vec::new());
        registry
            .apply(config(vec![agent("a", "0 9 * * *", true)]))
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_millis(50), registry.reload.notified())
            .await
            .expect("the swap must not be missed by a loop that parks after it");
    }

    #[tokio::test]
    async fn test_reload_keeps_the_previous_config_when_the_file_is_invalid() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
        );

        assert!(!registry.reload_from_disk().await.unwrap());

        std::fs::write(&path, "[[agent]\nbroken").unwrap();
        assert!(!registry.reload_from_disk().await.unwrap());

        {
            let inner = registry.inner.read().await;
            assert_eq!(inner.config.agents.len(), 1);
            assert_eq!(inner.config.agents[0].name, "a");
        }

        let error = registry.status().await.last_reload_error.unwrap();
        assert!(error.contains("failed to parse"), "{}", error);

        // The rejected content is fingerprinted, so it is not retried on every poll.
        assert!(!registry.reload_from_disk().await.unwrap());
        assert!(registry.status().await.last_reload_error.is_some());
    }

    #[tokio::test]
    async fn test_reload_applies_a_valid_change_and_clears_the_error() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
        );
        let before = registry.status().await;

        std::fs::write(&path, "[[agent]\nbroken").unwrap();
        assert!(!registry.reload_from_disk().await.unwrap());

        std::fs::write(
            &path,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n\n[[agent]]\nname = \"b\"\nschedule = \"0 10 * * *\"\nprompt = \"hi\"\n",
        )
        .unwrap();
        assert!(registry.reload_from_disk().await.unwrap());

        let after = registry.status().await;
        assert!(after.last_reload_error.is_none());
        assert_ne!(after.fingerprint, before.fingerprint);
        assert_eq!(registry.inner.read().await.config.agents.len(), 2);

        // Fingerprinted after the swap, so the watcher does not immediately re-reload.
        assert!(!registry.reload_from_disk().await.unwrap());
    }

    #[tokio::test]
    async fn test_source_path_and_config_text_track_the_file() {
        let dir = TempDir::new().unwrap();
        let body = "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n";
        let (path, registry) = on_disk_registry(&dir, body);

        assert_eq!(
            registry.source_path().to_path_buf(),
            std::fs::canonicalize(&path).unwrap()
        );
        assert_eq!(registry.config_text().await.unwrap(), body);
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
            editable: true,
            fingerprint: "0123456789abcdef".to_string(),
            loaded_at: 1784534400,
            reload_error: None,
            server_timezone: Some("America/Los_Angeles".to_string()),
            agents: vec![AgentStatus {
                name: "daily-digest".to_string(),
                schedule: "0 9 * * *".to_string(),
                model: Some("sonnet".to_string()),
                enabled: true,
                prompt_preview: "Summarize".to_string(),
                prompt_file: None,
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
                    run_status: Some(RunStatus::Success),
                }],
            }],
        };

        let json: serde_json::Value = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json["agents_file"],
            "/home/xeb/agents/agents.toml"
        );
        assert_eq!(json["server_model"], "claude-sonnet-4-5");
        assert_eq!(json["editable"], true);
        assert_eq!(json["fingerprint"], "0123456789abcdef");
        assert_eq!(json["loaded_at"], 1784534400);
        assert!(json["reload_error"].is_null());
        assert_eq!(json["server_timezone"], "America/Los_Angeles");

        let agent = &json["agents"][0];
        for field in [
            "name",
            "schedule",
            "model",
            "enabled",
            "prompt_preview",
            "prompt_file",
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
        assert_eq!(agent.as_object().unwrap().len(), 15);
        assert_eq!(agent["last_status"], "success");
        assert!(agent["last_error"].is_null());
        assert_eq!(agent["recent_sessions"][0]["agent_name"], "daily-digest");

        let empty = AgentsResponse {
            agents_file: None,
            agents: vec![],
            server_model: "gpt-5".to_string(),
            editable: false,
            fingerprint: String::new(),
            loaded_at: 0,
            reload_error: None,
            server_timezone: None,
        };
        assert_eq!(
            serde_json::to_string(&empty).unwrap(),
            r#"{"agents_file":null,"agents":[],"server_model":"gpt-5","editable":false,"fingerprint":"","loaded_at":0,"reload_error":null,"server_timezone":null}"#
        );
    }

    /// An undeterminable timezone must serialize as JSON null, because the client keys
    /// off null to fall back to browser-local time with an explicit note. Any fabricated
    /// default (e.g. "UTC") would make it silently show wrong fire times instead.
    #[test]
    fn test_unknown_server_timezone_serializes_as_null() {
        let response = AgentsResponse {
            agents_file: None,
            agents: vec![],
            server_model: "gpt-5".to_string(),
            editable: false,
            fingerprint: String::new(),
            loaded_at: 0,
            reload_error: None,
            server_timezone: None,
        };
        let json: serde_json::Value = serde_json::to_value(&response).unwrap();
        assert!(json.get("server_timezone").is_some(), "field must be present");
        assert!(json["server_timezone"].is_null());
    }

    #[test]
    fn test_server_timezone_lookup_is_stable_and_infallible() {
        // Must never panic, whatever the host's /etc/localtime looks like.
        let first = server_timezone();
        // Cached, so repeated calls agree — the UI polls this endpoint continuously.
        assert_eq!(first, server_timezone());
        if let Some(tz) = first {
            assert!(!tz.is_empty());
            assert!(
                !tz.contains(char::is_whitespace),
                "IANA names have no whitespace, got {:?}",
                tz
            );
        }
    }
}
