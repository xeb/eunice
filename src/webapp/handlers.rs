use super::persistence::SessionMetadata;
use super::scheduler;
use super::server::AppState;
use crate::client::Client;
use crate::compact::{compact_context, is_context_exhausted_error, CompactionConfig};
use crate::key_rotation::{BadKeyAction, RateLimitAction};
use crate::models::{Message, ProviderInfo};
use crate::tools::ToolRegistry;
use crate::usage::SessionUsage;
use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, Sse},
    Json,
};
use chrono::Local;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tokio_stream::StreamExt;

/// Log a webapp event with timestamp
fn log(event: &str) {
    let timestamp = Local::now().format("%H:%M:%S%.3f");
    println!("[{}] [webapp] {}", timestamp, event);
}

/// Standard headers for authenticated user identity (checked in order)
const USER_IDENTITY_HEADERS: &[&str] = &[
    "x-forwarded-email",      // Common for OAuth proxies
    "x-auth-request-email",   // OAuth2 Proxy
    "x-forwarded-user",       // Generic proxy header
    "x-auth-request-user",    // OAuth2 Proxy
    "remote-user",            // CGI standard
];

/// Extract authenticated user identity from proxy headers
fn extract_user_identity(headers: &HeaderMap) -> Option<String> {
    for header_name in USER_IDENTITY_HEADERS {
        if let Some(value) = headers.get(*header_name) {
            if let Ok(user) = value.to_str() {
                let user = user.trim();
                if !user.is_empty() {
                    return Some(user.to_string());
                }
            }
        }
    }
    None
}

/// Embedded HTML frontend
const INDEX_HTML: &str = include_str!("../../webapp/index.html");

/// Index page handler
pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// Status response
#[derive(Serialize)]
pub struct StatusResponse {
    version: String,
    model: String,
    provider: String,
    tools_count: usize,
    /// Authenticated user from proxy headers (if present)
    authenticated_user: Option<String>,
    /// Whether session persistence is enabled
    persistence_enabled: bool,
}

/// Status endpoint handler
pub async fn status(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<StatusResponse> {
    let tools_count = state.tool_registry.get_tools().len();

    // Extract authenticated user from proxy headers
    let authenticated_user = extract_user_identity(&headers);

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: state.provider_info.resolved_model.clone(),
        provider: state.provider_info.provider.to_string(),
        tools_count,
        authenticated_user,
        persistence_enabled: state.storage.is_persistent(),
    })
}

/// Tool info for config response
#[derive(Serialize)]
pub struct ToolInfo {
    name: String,
    description: Option<String>,
}

/// Config response
#[derive(Serialize)]
pub struct ConfigResponse {
    tools: Vec<ToolInfo>,
}

/// Config endpoint handler - returns detailed configuration
pub async fn config(State(state): State<Arc<AppState>>) -> Json<ConfigResponse> {
    let tools: Vec<ToolInfo> = state
        .tool_registry
        .get_tools()
        .iter()
        .map(|tool| ToolInfo {
            name: tool.function.name.clone(),
            description: Some(tool.function.description.clone()),
        })
        .collect();

    Json(ConfigResponse { tools })
}

/// Query request
#[derive(Deserialize)]
pub struct QueryRequest {
    prompt: String,
    /// Session ID for server-side history management
    #[serde(default)]
    session_id: Option<String>,
}

/// New session response
#[derive(Serialize)]
pub struct NewSessionResponse {
    session_id: String,
    session_name: String,
}

/// Create a new session
pub async fn new_session(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<NewSessionResponse> {
    let user_id = extract_user_identity(&headers);

    match state.storage.create_session(user_id.as_deref()).await {
        Ok(session) => {
            log(&format!("New session created: {} ({})", session.name, &session.id[..8]));
            Json(NewSessionResponse {
                session_id: session.id,
                session_name: session.name,
            })
        }
        Err(e) => {
            log(&format!("Failed to create session: {}", e));
            // Return empty session on error
            Json(NewSessionResponse {
                session_id: String::new(),
                session_name: String::new(),
            })
        }
    }
}

/// List sessions response
#[derive(Serialize)]
pub struct ListSessionsResponse {
    sessions: Vec<SessionMetadata>,
    persistent: bool,
}

/// List all sessions for the current user
pub async fn list_sessions(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<ListSessionsResponse> {
    let user_id = extract_user_identity(&headers);

    match state.storage.list_sessions(user_id.as_deref()).await {
        Ok(sessions) => {
            log(&format!("Listed {} sessions", sessions.len()));
            Json(ListSessionsResponse {
                sessions,
                persistent: state.storage.is_persistent(),
            })
        }
        Err(e) => {
            log(&format!("Failed to list sessions: {}", e));
            Json(ListSessionsResponse {
                sessions: vec![],
                persistent: state.storage.is_persistent(),
            })
        }
    }
}

/// Delete session request
#[derive(Deserialize)]
pub struct DeleteSessionRequest {
    session_id: String,
}

/// Delete session response
#[derive(Serialize)]
pub struct DeleteSessionResponse {
    deleted: bool,
}

/// Delete a session
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DeleteSessionRequest>,
) -> Json<DeleteSessionResponse> {
    match state.storage.delete_session(&request.session_id).await {
        Ok(deleted) => {
            if deleted {
                log(&format!("Deleted session: {}", &request.session_id[..8.min(request.session_id.len())]));
            }
            Json(DeleteSessionResponse { deleted })
        }
        Err(e) => {
            log(&format!("Failed to delete session: {}", e));
            Json(DeleteSessionResponse { deleted: false })
        }
    }
}

/// Session history request
#[derive(Deserialize)]
pub struct SessionHistoryRequest {
    session_id: String,
}

/// History message for client display
#[derive(Serialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum HistoryMessage {
    User { content: String },
    Assistant { content: Option<String>, tool_calls: Option<Vec<HistoryToolCall>> },
    Tool { tool_call_id: String, name: String, result: String },
}

/// Tool call info for history
#[derive(Serialize)]
pub struct HistoryToolCall {
    name: String,
    arguments: String,
}

/// Session history response
#[derive(Serialize)]
pub struct SessionHistoryResponse {
    exists: bool,
    messages: Vec<HistoryMessage>,
}

/// Get session history
pub async fn get_session_history(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(request): Json<SessionHistoryRequest>,
) -> Json<SessionHistoryResponse> {
    let authenticated_user = extract_user_identity(&headers);

    let session_id = if request.session_id.is_empty() {
        // No session ID provided - for authenticated users, get their most recent session
        if let Some(ref user) = authenticated_user {
            match state.storage.get_or_create_user_session(user).await {
                Ok(session) => session.id,
                Err(_) => return Json(SessionHistoryResponse { exists: false, messages: vec![] }),
            }
        } else {
            return Json(SessionHistoryResponse { exists: false, messages: vec![] });
        }
    } else {
        // Validate session ownership for authenticated users
        if let Some(ref user) = authenticated_user {
            if let Ok(Some(session)) = state.storage.get_session(&request.session_id).await {
                if let Some(ref session_user) = session.user_id {
                    if session_user != user {
                        return Json(SessionHistoryResponse { exists: false, messages: vec![] });
                    }
                }
            }
        }
        request.session_id.clone()
    };

    // Get history from storage
    match state.storage.get_history(&session_id).await {
        Ok(history) if !history.is_empty() => {
            let messages: Vec<HistoryMessage> = history.iter().map(|msg| {
                match msg {
                    Message::User { content } => HistoryMessage::User {
                        content: content.clone(),
                    },
                    Message::Assistant { content, tool_calls } => HistoryMessage::Assistant {
                        content: content.clone(),
                        tool_calls: tool_calls.as_ref().map(|tcs| {
                            tcs.iter().map(|tc| HistoryToolCall {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            }).collect()
                        }),
                    },
                    Message::Tool { tool_call_id, content } => {
                        // Try to find the tool name from the corresponding tool call
                        let name = history.iter()
                            .filter_map(|m| {
                                if let Message::Assistant { tool_calls: Some(tcs), .. } = m {
                                    tcs.iter().find(|tc| tc.id == *tool_call_id).map(|tc| tc.function.name.clone())
                                } else {
                                    None
                                }
                            })
                            .next()
                            .unwrap_or_else(|| "unknown".to_string());

                        HistoryMessage::Tool {
                            tool_call_id: tool_call_id.clone(),
                            name,
                            result: content.clone(),
                        }
                    }
                }
            }).collect();

            Json(SessionHistoryResponse {
                exists: true,
                messages,
            })
        }
        _ => Json(SessionHistoryResponse {
            exists: false,
            messages: vec![],
        }),
    }
}

/// Clear session response - returns the new session info
#[derive(Serialize)]
pub struct ClearSessionResponse {
    cleared: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_name: Option<String>,
}

/// Clear session history (for authenticated users to start fresh)
pub async fn clear_session(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<ClearSessionResponse> {
    if let Some(user) = extract_user_identity(&headers) {
        match state.storage.clear_user_session(&user).await {
            Ok(new_session) => {
                log(&format!(
                    "Created new session for {}: {} ({})",
                    user, new_session.id, new_session.name
                ));
                return Json(ClearSessionResponse {
                    cleared: true,
                    session_id: Some(new_session.id),
                    session_name: Some(new_session.name),
                });
            }
            Err(e) => {
                log(&format!("Failed to clear session for {}: {}", user, e));
            }
        }
    }
    Json(ClearSessionResponse {
        cleared: false,
        session_id: None,
        session_name: None,
    })
}

/// Scheduled agents endpoint - config plus live run state
pub async fn agents(State(state): State<Arc<AppState>>) -> Json<scheduler::AgentsResponse> {
    let server_model = state.provider_info.resolved_model.clone();

    match &state.agents {
        Some(registry) => {
            let status = registry.status().await;
            Json(scheduler::AgentsResponse {
                agents_file: Some(registry.source_path().display().to_string()),
                agents: registry.snapshot(&state.storage).await,
                server_model,
                editable: true,
                fingerprint: status.fingerprint,
                loaded_at: status.loaded_at,
                reload_error: status.last_reload_error,
                server_timezone: scheduler::server_timezone(),
            })
        }
        None => Json(scheduler::AgentsResponse {
            agents_file: None,
            agents: Vec::new(),
            server_model,
            editable: false,
            fingerprint: String::new(),
            loaded_at: 0,
            reload_error: None,
            server_timezone: scheduler::server_timezone(),
        }),
    }
}

/// Shown when an edit endpoint is called on a server started without `--agents`.
const NO_AGENTS_FILE: &str =
    "no agents file is configured; start the server with --agents to edit agents";

/// Returned when the file changed underneath an open editor. The UI offers a reload
/// rather than letting the browser overwrite an edit made in a text editor.
const FINGERPRINT_CONFLICT: &str =
    "agents.toml changed on disk since you opened this editor; reload and try again";

/// Mirror the loader's defaults so a form that omits a field produces the same agent
/// the file's own defaults would, rather than silently disabling or re-timing it.
fn default_timeout_secs() -> u64 {
    600
}

fn default_enabled() -> bool {
    true
}

/// Full definition of one agent, including the complete prompt text.
#[derive(Serialize)]
pub struct AgentDetail {
    name: String,
    schedule: String,
    model: Option<String>,
    prompt: String,
    prompt_file: Option<String>,
    enabled: bool,
    timeout_secs: u64,
    working_dir: Option<String>,
}

/// Get agent request
#[derive(Deserialize)]
pub struct AgentGetRequest {
    name: String,
}

/// Get agent response
#[derive(Serialize)]
pub struct AgentGetResponse {
    found: bool,
    agent: Option<AgentDetail>,
    fingerprint: String,
}

/// Save agent request. `original_name` is None when creating.
#[derive(Deserialize)]
pub struct AgentSaveRequest {
    #[serde(default)]
    original_name: Option<String>,
    fingerprint: String,
    name: String,
    schedule: String,
    prompt: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,
    #[serde(default)]
    working_dir: Option<String>,
}

/// Delete agent request
#[derive(Deserialize)]
pub struct AgentDeleteRequest {
    name: String,
    fingerprint: String,
}

/// Result of a save or a delete. `conflict` is what tells the UI to offer a reload
/// instead of showing the error inline.
#[derive(Serialize)]
pub struct AgentEditResponse {
    ok: bool,
    error: Option<String>,
    conflict: bool,
    fingerprint: String,
}

/// Reload agents response
#[derive(Serialize)]
pub struct AgentReloadResponse {
    ok: bool,
    changed: bool,
    error: Option<String>,
}

/// A rejected edit. Carries `conflict` separately because the two failure modes lead
/// to different UI: a stale fingerprint is recoverable by reloading, a validation or
/// write failure is not.
#[derive(Debug)]
struct EditError {
    message: String,
    conflict: bool,
}

impl EditError {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            conflict: false,
        }
    }

    fn conflict() -> Self {
        Self {
            message: FINGERPRINT_CONFLICT.to_string(),
            conflict: true,
        }
    }
}

/// What a save or delete intends to do: one mutation of agents.toml, plus the prompt
/// file to rewrite first when the target agent keeps its prompt outside the TOML.
struct EditPlan {
    mutation: crate::agents::AgentMutation,
    prompt_write: Option<(PathBuf, String)>,
}

/// Optional text fields arrive as null, absent, or "" from the form; all three mean unset.
fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Resolve a `prompt_file` value the way the loader does: relative paths hang off the
/// directory containing agents.toml.
fn resolve_prompt_file(base_dir: &Path, file: &str) -> PathBuf {
    let candidate = Path::new(file);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base_dir.join(candidate)
    }
}

/// The `prompt_file` declared for `name`, resolved. None when the agent stores its
/// prompt inline, does not exist, or the file no longer parses.
fn declared_prompt_file(doc_text: &str, base_dir: &Path, name: &str) -> Option<PathBuf> {
    let parsed: crate::agents::AgentsFile = toml::from_str(doc_text).ok()?;
    let spec = parsed.agents.into_iter().find(|s| s.name == name)?;
    spec.prompt_file
        .map(|file| resolve_prompt_file(base_dir, &file))
}

/// Assemble one agent's full definition from agents.toml source text. An unreadable
/// prompt_file yields an empty prompt rather than a failure, so the editor can be used
/// to recreate it.
fn agent_detail(doc_text: &str, base_dir: &Path, name: &str) -> Option<AgentDetail> {
    let parsed: crate::agents::AgentsFile = toml::from_str(doc_text).ok()?;
    let spec = parsed.agents.into_iter().find(|s| s.name == name)?;

    let (prompt, prompt_file) = match &spec.prompt_file {
        Some(file) => {
            let resolved = resolve_prompt_file(base_dir, file);
            let text = std::fs::read_to_string(&resolved).unwrap_or_default();
            (text, Some(resolved.display().to_string()))
        }
        None => (spec.prompt.clone().unwrap_or_default(), None),
    };

    Some(AgentDetail {
        name: spec.name,
        schedule: spec.schedule,
        model: spec.model,
        prompt,
        prompt_file,
        enabled: spec.enabled,
        timeout_secs: spec.timeout_secs,
        working_dir: spec.working_dir,
    })
}

/// Distinguishes the temp files of concurrent saves within one process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write via a temp file in the same directory and rename over the target, so a failed
/// or interrupted write cannot leave a half-written config behind. Same directory
/// because rename is only atomic within a filesystem.
fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "agents.toml".to_string());
    let temp = dir.join(format!(
        ".{}.{}.{}.tmp",
        name,
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    std::fs::write(&temp, contents)
        .map_err(|e| format!("failed to write '{}': {}", temp.display(), e))?;

    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("failed to write '{}': {}", path.display(), e));
    }

    Ok(())
}

/// Copy the current agents.toml aside before overwriting it. Failing here aborts the
/// save: a write that cannot be backed up is a write that loses the previous version.
fn backup_config(path: &Path) -> Result<(), String> {
    let Some(name) = path.file_name() else {
        return Ok(());
    };
    let backup = path.with_file_name(format!("{}.bak", name.to_string_lossy()));
    std::fs::copy(path, &backup)
        .map(|_| ())
        .map_err(|e| format!("failed to back up '{}': {}", path.display(), e))
}

/// The shared tail of save and delete: fingerprint check, mutate, validate, write, swap.
///
/// Validation runs on the PROPOSED text before anything is written, so a rejected edit
/// leaves the file on disk exactly as it was.
async fn commit_edit<F>(
    registry: &scheduler::AgentRegistry,
    submitted_fingerprint: &str,
    build: F,
) -> Result<String, EditError>
where
    F: FnOnce(&str, &Path) -> Result<EditPlan, EditError>,
{
    // Held across the whole read-modify-write. Validation awaits a blocking task, so
    // without this two saves that each passed the fingerprint check both proceed and
    // the later write silently discards the earlier one.
    let _edit_guard = registry.lock_edits().await;

    let loaded = registry.status().await;
    if loaded.fingerprint != submitted_fingerprint {
        return Err(EditError::conflict());
    }

    let source_path = registry.source_path().to_path_buf();
    let base_dir = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let current = registry
        .config_text()
        .await
        .map_err(|e| EditError::failed(e.to_string()))?;

    let plan = build(&current, &base_dir)?;
    let proposed = crate::agents::apply_mutation(&current, &plan.mutation)
        .map_err(|e| EditError::failed(e.to_string()))?;

    // Model resolution joins a thread that may make a blocking HTTP probe, so it
    // cannot run on a runtime worker.
    let validation_path = source_path.clone();
    let validation_text = proposed.clone();
    let validated = tokio::task::spawn_blocking(move || {
        crate::agents::validate_text(&validation_text, &validation_path, &|model| {
            crate::agents::detect_provider_isolated(model).map(|_| ())
        })
    })
    .await
    .map_err(|e| EditError::failed(format!("validation task failed: {}", e)))?;

    let mut validated = validated.map_err(|e| EditError::failed(e.to_string()))?;

    // Nothing has been written up to this point.

    // The edit lock only serializes saves made through this process; an external editor
    // can still have rewritten the file since `current` was read. Re-checking here makes
    // the whole mutation a no-op rather than an overwrite of work never seen.
    let on_disk = registry
        .config_text()
        .await
        .map_err(|e| EditError::failed(e.to_string()))?;
    if on_disk != current {
        return Err(EditError::conflict());
    }

    let written_prompt = match &plan.prompt_write {
        Some((path, contents)) => {
            // Same reasoning as the agents.toml backup below, but the file may not exist
            // yet: an agent can declare a prompt_file the editor is about to create.
            if path.exists() {
                backup_config(path).map_err(EditError::failed)?;
            }
            write_atomic(path, contents).map_err(EditError::failed)?;
            Some((
                std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()),
                contents.clone(),
            ))
        }
        None => None,
    };

    backup_config(&source_path).map_err(EditError::failed)?;
    write_atomic(&source_path, &proposed).map_err(EditError::failed)?;

    // validate_text read the prompt file as it stood before the write, so the agent
    // whose prompt_file was just rewritten would otherwise run on the old text until
    // some later reload.
    if let Some((path, contents)) = written_prompt {
        for agent in &mut validated.agents {
            if agent.prompt_file.as_deref() == Some(path.as_path()) {
                agent.prompt = contents.clone();
            }
        }
    }

    registry
        .apply(validated)
        .await
        .map_err(|e| EditError::failed(e.to_string()))?;

    Ok(registry.status().await.fingerprint)
}

/// Turn a commit result into the response both save and delete return.
async fn edit_response(
    registry: &scheduler::AgentRegistry,
    result: Result<String, EditError>,
) -> Json<AgentEditResponse> {
    match result {
        Ok(fingerprint) => Json(AgentEditResponse {
            ok: true,
            error: None,
            conflict: false,
            fingerprint,
        }),
        Err(EditError { message, conflict }) => Json(AgentEditResponse {
            ok: false,
            error: Some(message),
            conflict,
            // The current fingerprint, so the UI can recover without a full reload.
            fingerprint: registry.status().await.fingerprint,
        }),
    }
}

/// Full definition of a single agent, for the editor
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AgentGetRequest>,
) -> Json<AgentGetResponse> {
    let Some(registry) = &state.agents else {
        return Json(AgentGetResponse {
            found: false,
            agent: None,
            fingerprint: String::new(),
        });
    };

    let fingerprint = registry.status().await.fingerprint;
    let base_dir = registry
        .source_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let agent = match registry.config_text().await {
        Ok(text) => agent_detail(&text, &base_dir, &request.name),
        Err(e) => {
            log(&format!("Failed to read agents file: {}", e));
            None
        }
    };

    Json(AgentGetResponse {
        found: agent.is_some(),
        agent,
        fingerprint,
    })
}

/// Create or update an agent
pub async fn save_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AgentSaveRequest>,
) -> Json<AgentEditResponse> {
    let Some(registry) = &state.agents else {
        return Json(AgentEditResponse {
            ok: false,
            error: Some(NO_AGENTS_FILE.to_string()),
            conflict: false,
            fingerprint: String::new(),
        });
    };

    let AgentSaveRequest {
        original_name,
        fingerprint,
        name,
        schedule,
        prompt,
        model,
        enabled,
        timeout_secs,
        working_dir,
    } = request;

    let result = commit_edit(registry, &fingerprint, |current, base_dir| {
        // The loader catches an empty inline prompt, but a prompt_file agent is
        // validated against the file as it stands *before* the write, so an emptied
        // prompt would otherwise slip through and only fail on the next reload.
        if prompt.trim().is_empty() {
            return Err(EditError::failed(format!("agent '{}': prompt is empty", name)));
        }

        let prompt_file = original_name
            .as_deref()
            .and_then(|target| declared_prompt_file(current, base_dir, target));

        let spec = crate::agents::AgentSpec {
            name: name.clone(),
            schedule,
            model: blank_to_none(model),
            prompt: Some(prompt.clone()),
            // Never set from the UI: apply_mutation leaves an existing `prompt_file`
            // key alone, and a newly created agent always stores its prompt inline.
            prompt_file: None,
            enabled,
            timeout_secs,
            working_dir: blank_to_none(working_dir),
        };

        Ok(EditPlan {
            mutation: crate::agents::AgentMutation::Upsert {
                original_name,
                spec,
            },
            prompt_write: prompt_file.map(|path| (path, prompt)),
        })
    })
    .await;

    match &result {
        Ok(_) => log(&format!("Saved agent: {}", name)),
        Err(e) => log(&format!("Failed to save agent {}: {}", name, e.message)),
    }

    edit_response(registry, result).await
}

/// Delete an agent
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AgentDeleteRequest>,
) -> Json<AgentEditResponse> {
    let Some(registry) = &state.agents else {
        return Json(AgentEditResponse {
            ok: false,
            error: Some(NO_AGENTS_FILE.to_string()),
            conflict: false,
            fingerprint: String::new(),
        });
    };

    let name = request.name.clone();
    let result = commit_edit(registry, &request.fingerprint, |_current, _base_dir| {
        Ok(EditPlan {
            // The prompt file of a deleted agent is left on disk: it may be shared,
            // and removing a file the user wrote is not this endpoint's call.
            mutation: crate::agents::AgentMutation::Delete { name },
            prompt_write: None,
        })
    })
    .await;

    match &result {
        Ok(_) => log(&format!("Deleted agent: {}", request.name)),
        Err(e) => log(&format!(
            "Failed to delete agent {}: {}",
            request.name, e.message
        )),
    }

    edit_response(registry, result).await
}

/// Re-read agents.toml on demand
pub async fn reload_agents(State(state): State<Arc<AppState>>) -> Json<AgentReloadResponse> {
    let Some(registry) = &state.agents else {
        return Json(AgentReloadResponse {
            ok: false,
            changed: false,
            error: Some(NO_AGENTS_FILE.to_string()),
        });
    };

    match registry.reload_from_disk().await {
        Ok(changed) => {
            // A file that fails validation is reported as unchanged, not as an error,
            // so surface the rejection the registry recorded instead of claiming success.
            let error = registry.status().await.last_reload_error;
            Json(AgentReloadResponse {
                ok: error.is_none(),
                changed,
                error,
            })
        }
        Err(e) => Json(AgentReloadResponse {
            ok: false,
            changed: false,
            error: Some(e.to_string()),
        }),
    }
}

/// Request for session events (reconnection)
#[derive(Deserialize)]
pub struct SessionEventsRequest {
    session_id: String,
}

/// SSE event types
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Thinking { elapsed_seconds: u64 },
    ToolCall { name: String, arguments: String },
    ToolResult { name: String, result: String, truncated: bool },
    Response { content: String },
    /// Streaming chunk (partial response)
    StreamChunk { content: String },
    /// Informational message (key rotation, retries, etc.)
    Info { message: String },
    Error { message: String },
    /// Session ID confirmation (sent at start of query)
    SessionId { session_id: String },
    /// Token usage summary for this query
    Usage { input_tokens: u64, output_tokens: u64, cached_tokens: u64, estimated_cost: f64 },
    Done,
}

/// Session events endpoint - replay stored events and subscribe to live events if query is running
pub async fn session_events(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(request): Json<SessionEventsRequest>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let session_id = request.session_id;
    let session_short = if session_id.len() >= 8 { &session_id[..8] } else { &session_id };
    log(&format!("[{}] Session events requested (reconnection)", session_short));

    // Validate session ownership for authenticated users
    let authenticated_user = extract_user_identity(&headers);
    let access_denied = if let Some(ref user) = authenticated_user {
        if let Ok(Some(session)) = state.storage.get_session(&session_id).await {
            if let Some(ref session_user) = session.user_id {
                if session_user != user {
                    log(&format!("[{}] Access denied - session belongs to different user", session_short));
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // Get stored events and check if query is running from storage
    let (stored_events, broadcast_rx, query_running) = if access_denied {
        (Vec::new(), None, false)
    } else if let Some((events, event_tx, running)) =
        state.storage.get_runtime_state(&session_id).await
    {
        let rx = event_tx.as_ref().map(|tx| tx.subscribe());
        log(&format!("[{}] Found {} stored events, query_running: {}",
            session_short, events.len(), running));
        (events, rx, running)
    } else {
        log(&format!("[{}] Session not found", session_short));
        (Vec::new(), None, false)
    };

    // Check if there's a live query to subscribe to
    let has_live = broadcast_rx.is_some() && query_running;

    // Wrap in async stream that handles both replay and live
    let (tx, rx) = mpsc::channel::<SseEvent>(100);

    if let Some(broadcast_rx) = broadcast_rx {
        if query_running {
            let tx_clone = tx.clone();
            let session_short_owned = session_short.to_string();
            tokio::spawn(async move {
                let mut stream = BroadcastStream::new(broadcast_rx);
                while let Some(Ok(event)) = stream.next().await {
                    if tx_clone.send(event).await.is_err() {
                        break;
                    }
                }
                log(&format!("[{}] Live event subscription ended", session_short_owned));
            });
        }
    }

    // First send replay events, then live events
    let tx_replay = tx.clone();
    tokio::spawn(async move {
        for event in stored_events {
            if tx_replay.send(event).await.is_err() {
                return;
            }
        }
        if !has_live {
            let _ = tx_replay.send(SseEvent::Done).await;
        }
    });

    // Convert channel to SSE stream
    let combined_stream = ReceiverStream::new(rx).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        let event_type = match &event {
            SseEvent::Thinking { .. } => "thinking",
            SseEvent::ToolCall { .. } => "tool_call",
            SseEvent::ToolResult { .. } => "tool_result",
            SseEvent::Response { .. } => "response",
            SseEvent::StreamChunk { .. } => "stream_chunk",
            SseEvent::Info { .. } => "info",
            SseEvent::Error { .. } => "error",
            SseEvent::SessionId { .. } => "session_id",
            SseEvent::Usage { .. } => "usage",
            SseEvent::Done => "done",
        };
        Ok(axum::response::sse::Event::default()
            .event(event_type)
            .data(data))
    });

    Sse::new(combined_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// Helper to send events to multiple destinations
pub(super) struct EventSender {
    pub(super) tx: mpsc::Sender<SseEvent>,
    pub(super) state: Arc<AppState>,
    pub(super) session_id: String,
    pub(super) broadcast_tx: broadcast::Sender<SseEvent>,
}

/// Per-run overrides so a scheduled agent can use its own model and its own
/// tool registry (for `working_dir`) without disturbing the interactive defaults.
pub(super) struct RunContext {
    pub client: Arc<Client>,
    pub provider_info: Arc<ProviderInfo>,
    pub tool_registry: Arc<ToolRegistry>,
}

impl EventSender {
    pub(super) fn new(
        tx: mpsc::Sender<SseEvent>,
        state: Arc<AppState>,
        session_id: String,
        broadcast_tx: broadcast::Sender<SseEvent>,
    ) -> Self {
        Self { tx, state, session_id, broadcast_tx }
    }

    async fn send(&self, event: SseEvent) {
        // Store in session for replay
        self.state.storage.push_runtime_event(&self.session_id, event.clone()).await;

        // Broadcast to any re-subscribers
        let _ = self.broadcast_tx.send(event.clone());

        // Send to current SSE stream
        let _ = self.tx.send(event).await;
    }

    fn tx_clone(&self) -> mpsc::Sender<SseEvent> {
        self.tx.clone()
    }
}

use crate::display_sink::{DisplayEvent, DisplaySink};

/// Display sink for webapp that converts DisplayEvents to SseEvents
#[allow(dead_code)]
pub struct WebappDisplaySink {
    tx: mpsc::Sender<SseEvent>,
    #[allow(dead_code)]
    tool_output_limit: usize,
}

impl WebappDisplaySink {
    #[allow(dead_code)]
    pub fn new(tx: mpsc::Sender<SseEvent>, tool_output_limit: usize) -> Self {
        Self { tx, tool_output_limit }
    }
}

impl DisplaySink for WebappDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        let sse_event = match event {
            DisplayEvent::ThinkingStart => return,
            DisplayEvent::ThinkingStop => return,
            DisplayEvent::ToolCall { name, arguments } => {
                SseEvent::ToolCall { name, arguments }
            }
            DisplayEvent::ToolResult { result, limit } => {
                let (display_result, truncated) = if result.lines().count() > limit && limit > 0 {
                    let lines: Vec<&str> = result.lines().take(limit).collect();
                    (lines.join("\n"), true)
                } else {
                    (result, false)
                };
                SseEvent::ToolResult {
                    name: String::new(),
                    result: display_result,
                    truncated,
                }
            }
            DisplayEvent::Response { content } => {
                SseEvent::Response { content }
            }
            DisplayEvent::StreamChunk { content } => {
                SseEvent::StreamChunk { content }
            }
            DisplayEvent::StreamEnd => return, // No separate event needed
            DisplayEvent::Info { message } => {
                SseEvent::Info { message }
            }
            DisplayEvent::Error { message } => {
                SseEvent::Error { message }
            }
        };

        let _ = self.tx.try_send(sse_event);
    }
}

/// Query endpoint handler - returns SSE stream
pub async fn query(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let (tx, rx) = mpsc::channel::<SseEvent>(100);

    // Create cancellation channel
    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Store cancel sender for /api/cancel endpoint
    {
        let mut cancel_guard = state.cancel_tx.lock().await;
        *cancel_guard = Some(cancel_tx);
    }

    let state_clone = state.clone();
    let prompt = request.prompt.clone();

    // Check for authenticated user from proxy headers
    let authenticated_user = extract_user_identity(&headers);

    // Log incoming query with user info
    let prompt_preview = if request.prompt.len() > 100 {
        format!("{}...", &request.prompt[..100])
    } else {
        request.prompt.clone()
    };
    let user_info = authenticated_user.as_deref().unwrap_or("anonymous");
    log(&format!("Query from {}: \"{}\"", user_info, prompt_preview));

    // Get or create session
    let (session_id, session_name) = match request.session_id {
        Some(ref id) if !id.is_empty() => {
            // User specified a session - validate ownership
            if let Some(ref user) = authenticated_user {
                if let Ok(Some(session)) = state.storage.get_session(id).await {
                    if let Some(ref session_user) = session.user_id {
                        if session_user != user {
                            // Session belongs to a different user - create new session
                            log(&format!("Session {} belongs to different user, creating new session", &id[..8.min(id.len())]));
                            match state.storage.create_session(Some(user)).await {
                                Ok(new_session) => {
                                    log(&format!("Created new session: {} ({})", &new_session.id[..8], new_session.name));
                                    (new_session.id, new_session.name)
                                }
                                Err(_) => (uuid::Uuid::new_v4().to_string(), "unknown".to_string()),
                            }
                        } else {
                            (session.id, session.name)
                        }
                    } else {
                        let user_ref = authenticated_user.as_deref();
                        match state.storage.ensure_session(id, user_ref).await {
                            Ok(session) => (session.id, session.name),
                            Err(_) => (id.clone(), "unknown".to_string()),
                        }
                    }
                } else {
                    let user_ref = authenticated_user.as_deref();
                    match state.storage.ensure_session(id, user_ref).await {
                        Ok(session) => (session.id, session.name),
                        Err(_) => (id.clone(), "unknown".to_string()),
                    }
                }
            } else {
                let user_ref = authenticated_user.as_deref();
                match state.storage.ensure_session(id, user_ref).await {
                    Ok(session) => (session.id, session.name),
                    Err(e) => {
                        log(&format!("Failed to ensure session {}: {}", id, e));
                        (id.clone(), "unknown".to_string())
                    }
                }
            }
        }
        _ => {
            if let Some(ref user) = authenticated_user {
                match state.storage.get_or_create_user_session(user).await {
                    Ok(session) => (session.id, session.name),
                    Err(e) => {
                        log(&format!("Failed to get/create session for {}: {}", user, e));
                        match state.storage.create_session(Some(user)).await {
                            Ok(s) => (s.id, s.name),
                            Err(_) => (uuid::Uuid::new_v4().to_string(), "unknown".to_string()),
                        }
                    }
                }
            } else {
                match state.storage.create_session(None).await {
                    Ok(session) => (session.id, session.name),
                    Err(e) => {
                        log(&format!("Failed to create session: {}", e));
                        (uuid::Uuid::new_v4().to_string(), "unknown".to_string())
                    }
                }
            }
        }
    };

    log(&format!(
        "Session: {} ({}) for {}",
        &session_id[..8.min(session_id.len())],
        session_name,
        user_info
    ));

    // Create broadcast channel for this query
    let (broadcast_tx, _) = broadcast::channel::<SseEvent>(100);

    // Set up session for this query
    state.storage.clear_runtime_events(&session_id).await;
    state.storage.set_runtime_state(
        &session_id,
        Vec::new(),
        Some(broadcast_tx.clone()),
        true,
    ).await;

    // Create event sender
    let event_sender = EventSender {
        tx,
        state: state.clone(),
        session_id: session_id.clone(),
        broadcast_tx,
    };

    // Spawn agent task
    tokio::spawn(async move {
        // The client already received the failure as an SSE Error event.
        let _ = run_agent_with_events(state_clone, prompt, session_id, session_name, event_sender, cancel_rx, None, true).await;
    });

    // Convert channel to SSE stream
    let stream = ReceiverStream::new(rx).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        let event_type = match &event {
            SseEvent::Thinking { .. } => "thinking",
            SseEvent::ToolCall { .. } => "tool_call",
            SseEvent::ToolResult { .. } => "tool_result",
            SseEvent::Response { .. } => "response",
            SseEvent::StreamChunk { .. } => "stream_chunk",
            SseEvent::Info { .. } => "info",
            SseEvent::Error { .. } => "error",
            SseEvent::SessionId { .. } => "session_id",
            SseEvent::Usage { .. } => "usage",
            SseEvent::Done => "done",
        };
        Ok(axum::response::sse::Event::default()
            .event(event_type)
            .data(data))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// Cancel endpoint handler
pub async fn cancel(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut cancel_guard = state.cancel_tx.lock().await;
    if let Some(tx) = cancel_guard.take() {
        let _ = tx.send(true);
        Json(serde_json::json!({"cancelled": true}))
    } else {
        Json(serde_json::json!({"cancelled": false, "reason": "no active query"}))
    }
}

/// Prepend the system prompt to the first message of a new session.
/// There is no System message variant, so the prompt rides in the first
/// user message and persists with the session history.
fn compose_first_message(system_prompt: Option<&str>, first_turn: bool, prompt: &str) -> String {
    match (system_prompt, first_turn) {
        (Some(sys), true) => format!("{}\n\n---\n\n{}", sys, prompt),
        _ => prompt.to_string(),
    }
}

/// Run the agent loop and emit events.
///
/// `Err` carries the message of whatever ended the loop early. Interactive
/// callers ignore it (the client already saw the `Error` event); the scheduler
/// uses it to distinguish a failed run from a successful one.
pub(super) async fn run_agent_with_events(
    state: Arc<AppState>,
    prompt: String,
    session_id: String,
    session_name: String,
    event_sender: EventSender,
    cancel_rx: watch::Receiver<bool>,
    run_ctx: Option<RunContext>,
    manage_cancel_slot: bool,
) -> Result<(), String> {
    let mut run_error: Option<String> = None;
    let client: &Client = run_ctx.as_ref().map_or(state.client.as_ref(), |c| c.client.as_ref());
    let provider_info: &ProviderInfo =
        run_ctx.as_ref().map_or(&state.provider_info, |c| c.provider_info.as_ref());
    let tool_registry: &ToolRegistry =
        run_ctx.as_ref().map_or(state.tool_registry.as_ref(), |c| c.tool_registry.as_ref());

    let session_short = &session_id[..8];
    let log_prefix = format!("{} ({})", session_short, session_name);
    log(&format!("[{}] Agent loop starting", log_prefix));

    // Send session ID to client first
    event_sender.send(SseEvent::SessionId {
        session_id: session_id.clone(),
    }).await;

    // Load existing history
    let incoming_history = state
        .storage
        .get_history(&session_id)
        .await
        .unwrap_or_default();
    log(&format!("[{}] Loaded {} messages", log_prefix, incoming_history.len()));

    // Build conversation history
    let mut conversation_history: Vec<Message> = incoming_history;
    let user_content = compose_first_message(
        state.system_prompt.as_deref(),
        conversation_history.is_empty(),
        &prompt,
    );
    conversation_history.push(Message::User {
        content: user_content,
    });

    // Get tools
    let tools = tool_registry.get_tools();
    let tools_option = if tools.is_empty() { None } else { Some(tools) };

    // Token usage tracking
    let mut session_usage = SessionUsage::new();

    // Thinking timer
    let tx_thinking = event_sender.tx_clone();
    let cancel_rx_thinking = cancel_rx.clone();
    let thinking_handle = tokio::spawn(async move {
        let mut seconds = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if *cancel_rx_thinking.borrow() {
                break;
            }
            seconds += 1;
            if tx_thinking
                .send(SseEvent::Thinking {
                    elapsed_seconds: seconds,
                })
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Compaction config
    let compaction_config = CompactionConfig::default();
    let mut compaction_attempted = false;

    // Agent loop
    let mut loop_iteration = 0;
    loop {
        loop_iteration += 1;
        log(&format!("[{}] Loop iteration {}", log_prefix, loop_iteration));

        // Check for cancellation
        if *cancel_rx.borrow() {
            log(&format!("[{}] Query cancelled by user", log_prefix));
            event_sender.send(SseEvent::Error {
                message: "Query cancelled".to_string(),
            }).await;
            run_error = Some("Query cancelled".to_string());
            break;
        }

        // Call LLM
        log(&format!("[{}] Calling LLM ({}) with {} messages",
            log_prefix,
            provider_info.resolved_model,
            conversation_history.len()
        ));

        let response = client
            .chat_completion(
                &provider_info.resolved_model,
                serde_json::to_value(&conversation_history).unwrap(),
                tools_option.as_deref(),
            )
            .await;

        let response = match response {
            Ok(r) => {
                log(&format!("[{}] LLM response received", log_prefix));
                compaction_attempted = false; // Reset on success
                r
            }
            Err(e) => {
                let error_msg = format!("{:#}", e);
                log(&format!("[{}] LLM error: {}", log_prefix, error_msg));

                // Check for bad API key (403, invalid key) - blacklist and rotate
                if Client::is_bad_key_error(&error_msg) {
                    match client.handle_bad_key() {
                        BadKeyAction::Rotated { new_key_index } => {
                            let (_, total) = client.key_info();
                            log(&format!("[{}] Invalid API key, switching to key {}/{}", log_prefix, new_key_index, total));
                            continue; // Retry with new key
                        }
                        BadKeyAction::AllKeysBad => {
                            log(&format!("[{}] All API keys are invalid", log_prefix));
                            event_sender.send(SseEvent::Error {
                                message: "All API keys are invalid".to_string(),
                            }).await;
                            run_error = Some("All API keys are invalid".to_string());
                            break;
                        }
                    }
                }

                // Check for rate limit / quota error (429, 503, quota exceeded)
                if Client::is_quota_error(&error_msg) {
                    match client.handle_rate_limit() {
                        RateLimitAction::RetryAfterDelay(delay) => {
                            log(&format!("[{}] Rate limited, retrying in {}s...", log_prefix, delay.as_secs()));
                            tokio::time::sleep(delay).await;
                            continue; // Retry with same key
                        }
                        RateLimitAction::Rotated { new_key_index } => {
                            let (_, total) = client.key_info();
                            log(&format!("[{}] Rate limited, switching to key {}/{}", log_prefix, new_key_index, total));
                            continue; // Retry with new key
                        }
                        RateLimitAction::Exhausted { retry_after } => {
                            log(&format!("[{}] All API keys exhausted, retry in {}m", log_prefix, retry_after.as_secs() / 60));
                            let message = format!(
                                "All API keys exhausted, retry in {}m",
                                retry_after.as_secs() / 60
                            );
                            event_sender.send(SseEvent::Error {
                                message: message.clone(),
                            }).await;
                            run_error = Some(message);
                            break;
                        }
                        RateLimitAction::CooldownReset => {
                            log(&format!("[{}] Cooldown elapsed, retrying with first key...", log_prefix));
                            continue; // Retry with first key
                        }
                    }
                }

                // Try compaction if context exhausted
                if is_context_exhausted_error(&error_msg) && !compaction_attempted {
                    log(&format!("[{}] Context exhausted, attempting compaction...", log_prefix));
                    match compact_context(client, &provider_info.resolved_model, &conversation_history, &compaction_config).await {
                        Ok(compacted) => {
                            log(&format!("[{}] Compaction successful (ratio: {:.2})", log_prefix, compacted.compaction_ratio));
                            conversation_history = compacted.messages;
                            compaction_attempted = true;
                            continue; // Retry with compacted context
                        }
                        Err(compact_err) => {
                            log(&format!("[{}] Compaction failed: {:#}", log_prefix, compact_err));
                        }
                    }
                }

                let message = format!("API error: {:#}", e);
                event_sender.send(SseEvent::Error {
                    message: message.clone(),
                }).await;
                run_error = Some(message);
                break;
            }
        };

        // Track token usage
        if let Some(ref usage) = response.usage {
            session_usage.add(usage);
        }

        let choice = &response.choices[0];

        // Add assistant message to history
        conversation_history.push(Message::Assistant {
            content: choice.message.content.clone(),
            tool_calls: choice.message.tool_calls.clone(),
        });

        // Send response content if present
        if let Some(content) = &choice.message.content {
            if !content.is_empty() {
                let content_preview = if content.len() > 80 {
                    format!("{}...", &content[..80])
                } else {
                    content.clone()
                };
                log(&format!("[{}] Response: \"{}\"", log_prefix, content_preview.replace('\n', " ")));
                event_sender.send(SseEvent::Response {
                    content: content.clone(),
                }).await;
            }
        }

        // Check for tool calls
        let Some(tool_calls) = &choice.message.tool_calls else {
            log(&format!("[{}] No tool calls, loop complete", log_prefix));
            break;
        };

        if tool_calls.is_empty() {
            log(&format!("[{}] Empty tool calls, loop complete", log_prefix));
            break;
        }

        log(&format!("[{}] Processing {} tool call(s)", log_prefix, tool_calls.len()));

        // Execute tool calls
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let arguments = &tool_call.function.arguments;

            log(&format!("[{}] Tool call: {}", log_prefix, tool_name));

            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

            // Send tool call event
            event_sender.send(SseEvent::ToolCall {
                name: tool_name.clone(),
                arguments: arguments.clone(),
            }).await;

            // Execute tool
            let tool_result = if tool_registry.has_tool(tool_name) {
                tool_registry.execute(tool_name, args).await
                    .unwrap_or_else(|e| format!("Error: {}", e))
            } else {
                format!("Error: Unknown tool '{}'", tool_name)
            };

            let result_preview = if tool_result.len() > 80 {
                format!("{}...", &tool_result[..80])
            } else {
                tool_result.clone()
            };
            log(&format!("[{}] Tool result: {} chars ({})", log_prefix, tool_result.len(), result_preview.replace('\n', " ")));

            // Truncate result for display
            let (display_result, truncated) = if tool_result.lines().count() > state.tool_output_limit && state.tool_output_limit > 0 {
                let lines: Vec<&str> = tool_result.lines().take(state.tool_output_limit).collect();
                (lines.join("\n"), true)
            } else {
                (tool_result.clone(), false)
            };

            // Send tool result event
            event_sender.send(SseEvent::ToolResult {
                name: tool_name.clone(),
                result: display_result,
                truncated,
            }).await;

            // Add tool result to history
            conversation_history.push(Message::Tool {
                tool_call_id: tool_call.id.clone(),
                content: tool_result,
            });
        }
    }

    // Stop thinking timer
    thinking_handle.abort();

    // Save updated history to storage and mark query as complete
    let _ = state.storage.set_history(&session_id, &conversation_history).await;
    state.storage.set_runtime_state(&session_id, Vec::new(), None, false).await;

    // Persist events to storage
    for msg in &conversation_history {
        let (event_type, content) = match msg {
            Message::User { .. } => ("user_message", serde_json::to_string(msg).unwrap_or_default()),
            Message::Assistant { .. } => ("assistant_message", serde_json::to_string(msg).unwrap_or_default()),
            Message::Tool { .. } => ("tool_message", serde_json::to_string(msg).unwrap_or_default()),
        };
        let _ = state.storage.append_event(&session_id, event_type, &content).await;
    }

    log(&format!("[{}] Query complete, saved {} messages to session", log_prefix, conversation_history.len()));

    // Clear cancel sender. `state.cancel_tx` is a single global slot, so a scheduled
    // run must leave it alone or it would detach a concurrent interactive query.
    if manage_cancel_slot {
        let mut cancel_guard = state.cancel_tx.lock().await;
        *cancel_guard = None;
    }

    // Send usage event if we have any usage data
    if session_usage.has_usage() {
        let estimated_cost = session_usage.estimate_cost(
            &provider_info.resolved_model,
            &provider_info.provider
        );
        event_sender.send(SseEvent::Usage {
            input_tokens: session_usage.total_input_tokens,
            output_tokens: session_usage.total_output_tokens,
            cached_tokens: session_usage.total_cached_tokens,
            estimated_cost,
        }).await;
    }

    // Send done event
    event_sender.send(SseEvent::Done).await;

    match run_error {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const INLINE_AND_FILE: &str = r#"
[[agent]]
name = "daily-digest"
schedule = "0 9 * * *"
model = "sonnet"
prompt = "Summarize yesterday's commits"
working_dir = "/home/xeb/p/myrepo"

[[agent]]
name = "repo-watch"
schedule = "*/30 * * * *"
prompt_file = "prompts/repo-watch.md"
enabled = false
timeout_secs = 900
"#;

    #[test]
    fn test_blank_to_none_treats_absent_null_and_empty_alike() {
        assert_eq!(blank_to_none(None), None);
        assert_eq!(blank_to_none(Some(String::new())), None);
        assert_eq!(blank_to_none(Some("   ".to_string())), None);
        assert_eq!(
            blank_to_none(Some("  sonnet  ".to_string())),
            Some("sonnet".to_string())
        );
    }

    #[test]
    fn test_resolve_prompt_file_is_relative_to_the_config_directory() {
        let base = Path::new("/home/xeb/agents");
        assert_eq!(
            resolve_prompt_file(base, "prompts/watch.md"),
            PathBuf::from("/home/xeb/agents/prompts/watch.md")
        );
        assert_eq!(
            resolve_prompt_file(base, "/etc/eunice/watch.md"),
            PathBuf::from("/etc/eunice/watch.md")
        );
    }

    #[test]
    fn test_declared_prompt_file_only_for_agents_that_use_one() {
        let base = Path::new("/home/xeb/agents");

        assert_eq!(
            declared_prompt_file(INLINE_AND_FILE, base, "repo-watch"),
            Some(PathBuf::from("/home/xeb/agents/prompts/repo-watch.md"))
        );
        assert_eq!(
            declared_prompt_file(INLINE_AND_FILE, base, "daily-digest"),
            None
        );
        assert_eq!(declared_prompt_file(INLINE_AND_FILE, base, "nope"), None);
        assert_eq!(declared_prompt_file("not : toml", base, "repo-watch"), None);
    }

    #[test]
    fn test_agent_detail_returns_the_full_inline_prompt() {
        let base = Path::new("/home/xeb/agents");
        let detail = agent_detail(INLINE_AND_FILE, base, "daily-digest").unwrap();

        assert_eq!(detail.name, "daily-digest");
        assert_eq!(detail.schedule, "0 9 * * *");
        assert_eq!(detail.model, Some("sonnet".to_string()));
        assert_eq!(detail.prompt, "Summarize yesterday's commits");
        assert_eq!(detail.prompt_file, None);
        assert!(detail.enabled);
        assert_eq!(detail.timeout_secs, 600);
        assert_eq!(detail.working_dir, Some("/home/xeb/p/myrepo".to_string()));

        assert!(agent_detail(INLINE_AND_FILE, base, "missing").is_none());
    }

    #[test]
    fn test_agent_detail_reads_the_prompt_file_contents() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("prompts")).unwrap();
        std::fs::write(
            dir.path().join("prompts/repo-watch.md"),
            "Check the repo for new tags",
        )
        .unwrap();

        let detail = agent_detail(INLINE_AND_FILE, dir.path(), "repo-watch").unwrap();
        assert_eq!(detail.prompt, "Check the repo for new tags");
        assert_eq!(
            detail.prompt_file,
            Some(
                dir.path()
                    .join("prompts/repo-watch.md")
                    .display()
                    .to_string()
            )
        );
        assert!(!detail.enabled);
        assert_eq!(detail.timeout_secs, 900);
    }

    #[test]
    fn test_agent_detail_survives_a_missing_prompt_file() {
        let dir = TempDir::new().unwrap();
        let detail = agent_detail(INLINE_AND_FILE, dir.path(), "repo-watch").unwrap();
        assert_eq!(detail.prompt, "");
        assert!(detail.prompt_file.is_some());
    }

    #[test]
    fn test_save_request_treats_absent_optionals_as_unset() {
        let request: AgentSaveRequest = serde_json::from_str(
            r#"{"fingerprint":"abc","name":"nightly","schedule":"0 2 * * *","prompt":"go"}"#,
        )
        .unwrap();

        assert_eq!(request.original_name, None);
        assert_eq!(request.model, None);
        assert_eq!(request.working_dir, None);
        assert_eq!(request.timeout_secs, 600);
        // An omitted `enabled` must not disable an agent; the file format defaults to true.
        assert!(request.enabled);

        let with_nulls: AgentSaveRequest = serde_json::from_str(
            r#"{"original_name":"nightly","fingerprint":"abc","name":"nightly","schedule":"0 2 * * *",
                "prompt":"go","model":null,"enabled":false,"timeout_secs":30,"working_dir":""}"#,
        )
        .unwrap();

        assert_eq!(with_nulls.original_name, Some("nightly".to_string()));
        assert_eq!(with_nulls.timeout_secs, 30);
        assert!(!with_nulls.enabled);
        assert_eq!(blank_to_none(with_nulls.working_dir), None);
    }

    #[test]
    fn test_edit_responses_serialize_contract_field_names() {
        let ok = serde_json::to_value(AgentEditResponse {
            ok: true,
            error: None,
            conflict: false,
            fingerprint: "deadbeef".to_string(),
        })
        .unwrap();
        assert_eq!(ok["ok"], true);
        assert!(ok["error"].is_null());
        assert_eq!(ok["conflict"], false);
        assert_eq!(ok["fingerprint"], "deadbeef");

        let conflicted = EditError::conflict();
        assert!(conflicted.conflict);
        assert_eq!(conflicted.message, FINGERPRINT_CONFLICT);

        let reload = serde_json::to_string(&AgentReloadResponse {
            ok: false,
            changed: false,
            error: Some("bad cron".to_string()),
        })
        .unwrap();
        assert_eq!(reload, r#"{"ok":false,"changed":false,"error":"bad cron"}"#);

        let missing = serde_json::to_value(AgentGetResponse {
            found: false,
            agent: None,
            fingerprint: String::new(),
        })
        .unwrap();
        assert_eq!(missing["found"], false);
        assert!(missing["agent"].is_null());
    }

    #[test]
    fn test_get_response_carries_every_editable_field() {
        let base = Path::new("/home/xeb/agents");
        let response = AgentGetResponse {
            found: true,
            agent: agent_detail(INLINE_AND_FILE, base, "daily-digest"),
            fingerprint: "abc".to_string(),
        };

        let json = serde_json::to_value(&response).unwrap();
        let agent = &json["agent"];
        for field in [
            "name",
            "schedule",
            "model",
            "prompt",
            "prompt_file",
            "enabled",
            "timeout_secs",
            "working_dir",
        ] {
            assert!(agent.get(field).is_some(), "missing field {}", field);
        }
        assert_eq!(agent.as_object().unwrap().len(), 8);
    }

    #[test]
    fn test_write_atomic_replaces_the_target_and_leaves_no_temp_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("agents.toml");
        std::fs::write(&target, "old").unwrap();

        write_atomic(&target, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|name| name != "agents.toml")
            .collect();
        assert!(leftovers.is_empty(), "left behind {:?}", leftovers);
    }

    #[test]
    fn test_write_atomic_reports_an_unwritable_target_as_an_error() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("no-such-dir").join("agents.toml");

        let error = write_atomic(&target, "body").unwrap_err();
        assert!(
            error.starts_with("failed to write"),
            "unreadable error: {}",
            error
        );
    }

    #[test]
    fn test_backup_config_keeps_the_previous_contents() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("agents.toml");
        std::fs::write(&target, "first").unwrap();

        backup_config(&target).unwrap();
        write_atomic(&target, "second").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "second");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("agents.toml.bak")).unwrap(),
            "first"
        );

        backup_config(&target).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("agents.toml.bak")).unwrap(),
            "second"
        );
    }

    #[test]
    fn test_backup_config_reports_a_missing_source_as_an_error() {
        let dir = TempDir::new().unwrap();
        let error = backup_config(&dir.path().join("agents.toml")).unwrap_err();
        assert!(
            error.starts_with("failed to back up"),
            "unreadable error: {}",
            error
        );
    }

    /// A registry backed by a real agents.toml. No agent declares a `model`, so
    /// neither the registry nor validation reaches provider detection.
    fn on_disk_registry(dir: &TempDir) -> (PathBuf, scheduler::AgentRegistry) {
        let path = dir.path().join("agents.toml");
        std::fs::write(
            &path,
            "# hand-written header\n\n[[agent]]\nname = \"daily-digest\"\nschedule = \"0 9 * * *\"\nprompt = \"summarize\"\n",
        )
        .unwrap();
        let config = crate::agents::load_agents_file(&path, &|_| Ok(())).unwrap();
        let registry = scheduler::AgentRegistry::new(config, "server-model").unwrap();
        (path, registry)
    }

    fn delete_plan(name: &str) -> EditPlan {
        EditPlan {
            mutation: crate::agents::AgentMutation::Delete {
                name: name.to_string(),
            },
            prompt_write: None,
        }
    }

    #[tokio::test]
    async fn test_commit_edit_rejects_a_stale_fingerprint_without_writing() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(&dir);
        let before = std::fs::read_to_string(&path).unwrap();

        let error = commit_edit(&registry, "not-the-current-fingerprint", |_, _| {
            Ok(delete_plan("daily-digest"))
        })
        .await
        .unwrap_err();

        assert!(error.conflict);
        assert_eq!(error.message, FINGERPRINT_CONFLICT);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        assert!(!dir.path().join("agents.toml.bak").exists());
    }

    #[tokio::test]
    async fn test_commit_edit_leaves_the_file_untouched_when_validation_fails() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(&dir);
        let before = std::fs::read_to_string(&path).unwrap();
        let fingerprint = registry.status().await.fingerprint;

        let error = commit_edit(&registry, &fingerprint, |_, _| {
            Ok(EditPlan {
                mutation: crate::agents::AgentMutation::Upsert {
                    original_name: Some("daily-digest".to_string()),
                    spec: crate::agents::AgentSpec {
                        name: "daily-digest".to_string(),
                        schedule: "not a cron".to_string(),
                        model: None,
                        prompt: Some("summarize".to_string()),
                        prompt_file: None,
                        enabled: true,
                        timeout_secs: 600,
                        working_dir: None,
                    },
                },
                prompt_write: None,
            })
        })
        .await
        .unwrap_err();

        assert!(!error.conflict);
        assert!(
            error.message.contains("invalid schedule"),
            "unhelpful error: {}",
            error.message
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        assert!(!dir.path().join("agents.toml.bak").exists());
    }

    #[tokio::test]
    async fn test_commit_edit_writes_backs_up_and_refreshes_the_fingerprint() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(&dir);
        let before = std::fs::read_to_string(&path).unwrap();
        let submitted = registry.status().await.fingerprint;

        let returned = commit_edit(&registry, &submitted, |_, _| {
            Ok(delete_plan("daily-digest"))
        })
        .await
        .unwrap();

        let after = std::fs::read_to_string(&path).unwrap();
        assert!(!after.contains("daily-digest"));
        assert!(after.contains("# hand-written header"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("agents.toml.bak")).unwrap(),
            before
        );

        // The registry must already agree with what is on disk, or the watcher would
        // re-read the file this save just wrote.
        assert_ne!(returned, submitted);
        assert_eq!(registry.status().await.fingerprint, returned);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_commit_edits_do_not_lose_an_update() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("agents.toml");
        std::fs::write(
            &path,
            "[[agent]]\nname = \"one\"\nschedule = \"0 9 * * *\"\nprompt = \"a\"\n\n[[agent]]\nname = \"two\"\nschedule = \"0 10 * * *\"\nprompt = \"b\"\n",
        )
        .unwrap();
        let config = crate::agents::load_agents_file(&path, &|_| Ok(())).unwrap();
        let registry = Arc::new(scheduler::AgentRegistry::new(config, "server-model").unwrap());
        let fingerprint = registry.status().await.fingerprint;

        // Two saves prepared against the same fingerprint, targeting different agents.
        // Whichever lands second must be rejected, not silently overwrite the first.
        let (first, second) = tokio::join!(
            {
                let registry = registry.clone();
                let fingerprint = fingerprint.clone();
                async move {
                    commit_edit(&registry, &fingerprint, |_, _| Ok(delete_plan("one"))).await
                }
            },
            {
                let registry = registry.clone();
                let fingerprint = fingerprint.clone();
                async move {
                    commit_edit(&registry, &fingerprint, |_, _| Ok(delete_plan("two"))).await
                }
            }
        );

        let text = std::fs::read_to_string(&path).unwrap();
        let survivors = [("one", "one"), ("two", "two")]
            .iter()
            .filter(|(_, needle)| text.contains(*needle))
            .count();
        assert_eq!(survivors, 1, "one deletion was lost: {}", text);

        let losers = [&first, &second].iter().filter(|r| r.is_err()).count();
        assert_eq!(losers, 1, "both saves reported success");
        for result in [&first, &second] {
            if let Err(e) = result {
                assert!(e.conflict, "the losing save must be reported as a conflict");
            }
        }
    }

    #[tokio::test]
    async fn test_commit_edit_reports_an_external_rewrite_as_a_conflict() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(&dir);
        let fingerprint = registry.status().await.fingerprint;

        let external = "[[agent]]\nname = \"typed-by-hand\"\nschedule = \"0 9 * * *\"\nprompt = \"x\"\n";
        let error = commit_edit(&registry, &fingerprint, |_, _| {
            // Stands in for an external editor writing between the read and the write.
            std::fs::write(&path, external).unwrap();
            Ok(delete_plan("daily-digest"))
        })
        .await
        .unwrap_err();

        assert!(error.conflict);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        assert!(!dir.path().join("agents.toml.bak").exists());
    }

    #[tokio::test]
    async fn test_commit_edit_reports_a_missing_agent_as_a_plain_error() {
        let dir = TempDir::new().unwrap();
        let (path, registry) = on_disk_registry(&dir);
        let before = std::fs::read_to_string(&path).unwrap();
        let fingerprint = registry.status().await.fingerprint;

        let error = commit_edit(&registry, &fingerprint, |_, _| Ok(delete_plan("ghost")))
            .await
            .unwrap_err();

        assert!(!error.conflict);
        assert_eq!(error.message, "no agent named 'ghost' in the config");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn test_system_prompt_prepended_on_first_turn_only() {
        // New session: system prompt rides in the first user message
        let composed = compose_first_message(Some("You are an ERP investigator."), true, "list tables");
        assert!(composed.starts_with("You are an ERP investigator."));
        assert!(composed.ends_with("list tables"));
        assert!(composed.contains("\n\n---\n\n"));

        // Existing session: prompt passes through untouched
        assert_eq!(
            compose_first_message(Some("You are an ERP investigator."), false, "next question"),
            "next question"
        );

        // No system prompt configured: passthrough on first turn too
        assert_eq!(compose_first_message(None, true, "hello"), "hello");
    }
}
