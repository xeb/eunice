use super::persistence::SessionMetadata;
use super::server::AppState;
use crate::agent;
use crate::compact::{compact_context, extract_retry_delay, is_context_exhausted_error, is_rate_limit_error, CompactionConfig};
use crate::config::DMN_INSTRUCTIONS;
use crate::models::Message;
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
    mode: String,
    agent: Option<String>,
    mcp_servers: Vec<String>,
    tools_count: usize,
    /// Authenticated user from proxy headers (if present)
    authenticated_user: Option<String>,
    /// Whether session persistence is enabled (SQLite)
    persistence_enabled: bool,
}

/// Status endpoint handler
pub async fn status(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<StatusResponse> {
    let mcp_manager = state.mcp_manager.lock().await;
    let (servers, mut tools_count) = if let Some(ref manager) = *mcp_manager {
        let info = manager.get_server_info();
        let servers: Vec<String> = info.iter().map(|(name, _, _)| name.clone()).collect();
        let tools: usize = info.iter().map(|(_, count, _)| count).sum();
        (servers, tools)
    } else {
        (vec![], 0)
    };

    // Add built-in tools to count
    if state.enable_image_tool {
        tools_count += 1;
    }
    if state.enable_search_tool {
        tools_count += 1;
    }

    let mode = if state.research {
        "research"
    } else if state.dmn {
        "dmn"
    } else {
        "standard"
    };

    // Extract authenticated user from proxy headers
    let authenticated_user = extract_user_identity(&headers);

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: state.provider_info.resolved_model.clone(),
        provider: state.provider_info.provider.to_string(),
        mode: mode.to_string(),
        agent: state.agent_name.clone(),
        mcp_servers: servers,
        tools_count,
        authenticated_user,
        persistence_enabled: state.storage.is_persistent(),
    })
}

/// Agent info for config response
#[derive(Serialize)]
pub struct AgentInfo {
    name: String,
    tools: Vec<String>,
    can_invoke: Vec<String>,
}

/// Server info for config response
#[derive(Serialize)]
pub struct ServerInfo {
    name: String,
    transport: String,
    connection: String,
    tool_count: usize,
}

/// Tool info for config response
#[derive(Serialize)]
pub struct ToolInfo {
    name: String,
    server: String,
    description: Option<String>,
}

/// Config response
#[derive(Serialize)]
pub struct ConfigResponse {
    agents: Vec<AgentInfo>,
    servers: Vec<ServerInfo>,
    tools: Vec<ToolInfo>,
}

/// Config endpoint handler - returns detailed configuration
pub async fn config(State(state): State<Arc<AppState>>) -> Json<ConfigResponse> {
    let mcp_manager = state.mcp_manager.lock().await;

    // Get agents from orchestrator
    let agents: Vec<AgentInfo> = if let Some(ref orch) = state.orchestrator {
        orch.agent_names()
            .iter()
            .filter_map(|name| {
                orch.get_agent(name).map(|agent| AgentInfo {
                    name: name.clone(),
                    tools: agent.tools.clone(),
                    can_invoke: agent.can_invoke.clone(),
                })
            })
            .collect()
    } else {
        vec![]
    };

    // Get servers and tools from MCP manager
    let (servers, mut tools) = if let Some(ref manager) = *mcp_manager {
        let server_info = manager.get_server_info();
        let servers: Vec<ServerInfo> = server_info
            .iter()
            .map(|(name, count, _)| ServerInfo {
                name: name.clone(),
                transport: "stdio".to_string(), // Could be enhanced to detect HTTP
                connection: "connected".to_string(),
                tool_count: *count,
            })
            .collect();

        let tools: Vec<ToolInfo> = manager
            .get_tools()
            .iter()
            .map(|tool| {
                // Extract server name from tool prefix (e.g., "shell_execute" -> "shell")
                let server = tool
                    .function
                    .name
                    .split('_')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                ToolInfo {
                    name: tool.function.name.clone(),
                    server,
                    description: Some(tool.function.description.clone()),
                }
            })
            .collect();

        (servers, tools)
    } else {
        (vec![], vec![])
    };

    // Add built-in tools if enabled
    if state.enable_image_tool {
        tools.push(ToolInfo {
            name: agent::INTERPRET_IMAGE_TOOL_NAME.to_string(),
            server: "built-in".to_string(),
            description: Some("Analyze images and PDFs using multimodal AI".to_string()),
        });
    }
    if state.enable_search_tool {
        tools.push(ToolInfo {
            name: agent::SEARCH_QUERY_TOOL_NAME.to_string(),
            server: "built-in".to_string(),
            description: Some("Web search using Gemini with Google Search grounding".to_string()),
        });
    }

    Json(ConfigResponse {
        agents,
        servers,
        tools,
    })
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
            // Check if the session belongs to this user
            if let Ok(Some(session)) = state.storage.get_session(&request.session_id).await {
                // If session has a user_id, it must match the authenticated user
                if let Some(ref session_user) = session.user_id {
                    if session_user != user {
                        // Session belongs to a different user - deny access
                        return Json(SessionHistoryResponse { exists: false, messages: vec![] });
                    }
                }
                // Anonymous sessions (user_id = None) can be accessed by anyone
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
/// Creates a new session and returns its ID and name
pub async fn clear_session(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<ClearSessionResponse> {
    // Check for authenticated user
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

/// Request for session events (reconnection)
#[derive(Deserialize)]
pub struct SessionEventsRequest {
    session_id: String,
}

/// Session events endpoint - replay stored events and subscribe to live events if query is running
/// This allows users to reconnect and see events that happened while they were disconnected
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
    // If access denied, return empty events
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
                        break;  // Receiver dropped
                    }
                }
                log(&format!("[{}] Live event subscription ended", session_short_owned));
            });
        }
    }

    // First send replay events, then live events
    let tx_replay = tx.clone();
    tokio::spawn(async move {
        // Send all stored events first
        for event in stored_events {
            if tx_replay.send(event).await.is_err() {
                return;  // Receiver dropped
            }
        }
        // If no live events, send Done to close the stream
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
            SseEvent::AgentInvoke { .. } => "agent_invoke",
            SseEvent::AgentResult { .. } => "agent_result",
            SseEvent::Response { .. } => "response",
            SseEvent::Error { .. } => "error",
            SseEvent::SessionId { .. } => "session_id",
            SseEvent::Compacted { .. } => "compacted",
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

/// SSE event types
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Thinking { elapsed_seconds: u64 },
    ToolCall { name: String, arguments: String },
    ToolResult { name: String, result: String, truncated: bool },
    /// Agent being invoked (multi-agent mode)
    AgentInvoke { agent_name: String, task: String },
    /// Agent invocation completed (multi-agent mode)
    AgentResult { agent_name: String, result: String, truncated: bool },
    Response { content: String },
    Error { message: String },
    /// Session ID confirmation (sent at start of query)
    SessionId { session_id: String },
    /// Context was compacted due to size limits
    Compacted { message: String },
    /// Token usage summary for this query
    Usage { input_tokens: u64, output_tokens: u64, cached_tokens: u64, estimated_cost: f64 },
    Done,
}

/// Helper to send events to multiple destinations
/// - mpsc channel (current SSE stream subscriber)
/// - session store (for replay on reconnect)
/// - broadcast channel (for re-subscribers while query is running)
struct EventSender {
    tx: mpsc::Sender<SseEvent>,
    state: Arc<AppState>,
    session_id: String,
    broadcast_tx: broadcast::Sender<SseEvent>,
}

impl EventSender {
    async fn send(&self, event: SseEvent) {
        // Store in session for replay
        self.state.storage.push_runtime_event(&self.session_id, event.clone()).await;

        // Broadcast to any re-subscribers (ignore errors - no subscribers is fine)
        let _ = self.broadcast_tx.send(event.clone());

        // Send to current SSE stream (ignore errors - client may have disconnected)
        let _ = self.tx.send(event).await;
    }

    /// Clone the mpsc sender for use in spawned tasks (e.g., thinking timer)
    fn tx_clone(&self) -> mpsc::Sender<SseEvent> {
        self.tx.clone()
    }
}

use crate::display_sink::{DisplayEvent, DisplaySink};

/// Display sink for webapp that converts DisplayEvents to SseEvents
pub struct WebappDisplaySink {
    tx: mpsc::Sender<SseEvent>,
    #[allow(dead_code)]  // May be used for future truncation
    tool_output_limit: usize,
}

impl WebappDisplaySink {
    pub fn new(tx: mpsc::Sender<SseEvent>, tool_output_limit: usize) -> Self {
        Self { tx, tool_output_limit }
    }
}

impl DisplaySink for WebappDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        let sse_event = match event {
            DisplayEvent::ThinkingStart => {
                // Thinking is handled by a separate timer in the webapp
                return;
            }
            DisplayEvent::ThinkingStop => {
                return;
            }
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
                    name: String::new(),  // Name is sent with ToolCall
                    result: display_result,
                    truncated,
                }
            }
            DisplayEvent::AgentInvoke { agent_name, task } => {
                SseEvent::AgentInvoke { agent_name, task }
            }
            DisplayEvent::AgentResult { agent_name, result, limit } => {
                let (display_result, truncated) = if result.lines().count() > limit && limit > 0 {
                    let lines: Vec<&str> = result.lines().take(limit).collect();
                    (lines.join("\n"), true)
                } else {
                    (result, false)
                };
                SseEvent::AgentResult {
                    agent_name,
                    result: display_result,
                    truncated,
                }
            }
            DisplayEvent::Response { content } => {
                SseEvent::Response { content }
            }
            DisplayEvent::Error { message } => {
                SseEvent::Error { message }
            }
            DisplayEvent::Debug { .. } => {
                // Don't send debug events to webapp
                return;
            }
            DisplayEvent::Newline => {
                return;
            }
        };

        // Use blocking send since write_event is not async
        let _ = self.tx.try_send(sse_event);
    }

    fn is_verbose(&self) -> bool {
        false
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

    // Get or create session using storage layer
    // IMPORTANT: Always respect request.session_id if provided - user chose that session!
    let (session_id, session_name) = match request.session_id {
        Some(ref id) if !id.is_empty() => {
            // User specified a session - validate ownership before using
            if let Some(ref user) = authenticated_user {
                // Check if the session belongs to this user
                if let Ok(Some(session)) = state.storage.get_session(id).await {
                    if let Some(ref session_user) = session.user_id {
                        if session_user != user {
                            // Session belongs to a different user - create new session for this user
                            log(&format!("Session {} belongs to different user, creating new session", &id[..8.min(id.len())]));
                            match state.storage.create_session(Some(user)).await {
                                Ok(new_session) => {
                                    log(&format!("Created new session: {} ({})", &new_session.id[..8], new_session.name));
                                    (new_session.id, new_session.name)
                                }
                                Err(_) => (uuid::Uuid::new_v4().to_string(), "unknown".to_string()),
                            }
                        } else {
                            // Session belongs to this user - use it
                            (session.id, session.name)
                        }
                    } else {
                        // Anonymous session - ensure and use it
                        let user_ref = authenticated_user.as_deref();
                        match state.storage.ensure_session(id, user_ref).await {
                            Ok(session) => (session.id, session.name),
                            Err(_) => (id.clone(), "unknown".to_string()),
                        }
                    }
                } else {
                    // Session doesn't exist - create it
                    let user_ref = authenticated_user.as_deref();
                    match state.storage.ensure_session(id, user_ref).await {
                        Ok(session) => (session.id, session.name),
                        Err(_) => (id.clone(), "unknown".to_string()),
                    }
                }
            } else {
                // Anonymous user - just use the session
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
            // No session specified - get or create one
            if let Some(ref user) = authenticated_user {
                // Get or create session for authenticated user
                match state.storage.get_or_create_user_session(user).await {
                    Ok(session) => (session.id, session.name),
                    Err(e) => {
                        log(&format!("Failed to get/create session for {}: {}", user, e));
                        // Create fallback session
                        match state.storage.create_session(Some(user)).await {
                            Ok(s) => (s.id, s.name),
                            Err(_) => (uuid::Uuid::new_v4().to_string(), "unknown".to_string()),
                        }
                    }
                }
            } else {
                // Create new anonymous session
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

    // Log session info
    log(&format!(
        "Session: {} ({}) for {}",
        &session_id[..8.min(session_id.len())],
        session_name,
        user_info
    ));

    // Create broadcast channel for this query (for re-subscribers)
    let (broadcast_tx, _) = broadcast::channel::<SseEvent>(100);

    // Set up session for this query: clear old events, store broadcast sender, mark as running
    state.storage.clear_runtime_events(&session_id).await;
    state.storage.set_runtime_state(
        &session_id,
        Vec::new(),
        Some(broadcast_tx.clone()),
        true,
    ).await;

    // Create event sender that stores events and broadcasts to re-subscribers
    let event_sender = EventSender {
        tx,
        state: state.clone(),
        session_id: session_id.clone(),
        broadcast_tx,
    };

    // Spawn agent task
    tokio::spawn(async move {
        run_agent_with_events(state_clone, prompt, session_id, session_name, event_sender, cancel_rx).await;
    });

    // Convert channel to SSE stream
    let stream = ReceiverStream::new(rx).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        let event_type = match &event {
            SseEvent::Thinking { .. } => "thinking",
            SseEvent::ToolCall { .. } => "tool_call",
            SseEvent::ToolResult { .. } => "tool_result",
            SseEvent::AgentInvoke { .. } => "agent_invoke",
            SseEvent::AgentResult { .. } => "agent_result",
            SseEvent::Response { .. } => "response",
            SseEvent::Error { .. } => "error",
            SseEvent::SessionId { .. } => "session_id",
            SseEvent::Compacted { .. } => "compacted",
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

/// Run the agent loop and emit events
async fn run_agent_with_events(
    state: Arc<AppState>,
    prompt: String,
    session_id: String,
    session_name: String,
    event_sender: EventSender,
    cancel_rx: watch::Receiver<bool>,
) {
    let session_short = &session_id[..8];
    let log_prefix = format!("{} ({})", session_short, session_name);
    log(&format!("[{}] Agent loop starting", log_prefix));

    // Send session ID to client first
    event_sender.send(SseEvent::SessionId {
        session_id: session_id.clone(),
    }).await;

    // Load existing history from storage
    let incoming_history = state.storage.get_history(&session_id).await.unwrap_or_default();
    log(&format!("[{}] Loaded {} messages from history", log_prefix, incoming_history.len()));

    // Prepare the prompt with DMN instructions if needed (only for first message in session)
    let final_prompt = if state.dmn && incoming_history.is_empty() {
        format!(
            "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in DMN (Default Mode Network) autonomous batch mode. Execute the user request above completely using your available MCP tools. Do not stop for confirmation.",
            DMN_INSTRUCTIONS, prompt
        )
    } else {
        prompt
    };

    // Get MCP manager
    let mut mcp_guard = state.mcp_manager.lock().await;

    // Build tool list
    let mut tools = mcp_guard
        .as_ref()
        .map(|m| m.get_tools())
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.function.name.is_empty())
        .collect::<Vec<_>>();

    if state.enable_image_tool {
        tools.push(agent::get_interpret_image_tool_spec());
    }
    if state.enable_search_tool {
        tools.push(agent::get_search_query_tool_spec());
    }

    // Add invoke tools if orchestrator is present (multi-agent mode)
    let current_agent = state.agent_name.as_deref().unwrap_or("root");
    if let Some(ref orchestrator) = state.orchestrator {
        let invoke_tools = orchestrator.get_invoke_tools(current_agent);
        log(&format!("[{}] Adding {} invoke tools for agent '{}'", log_prefix, invoke_tools.len(), current_agent));
        tools.extend(invoke_tools);
    }

    let tools_option = if tools.is_empty() { None } else { Some(tools) };

    // Build conversation history from incoming history + new user message
    let mut conversation_history: Vec<Message> = incoming_history;
    conversation_history.push(Message::User {
        content: final_prompt.clone(),
    });

    // Compaction config
    let compaction_config = CompactionConfig::default();
    let mut compaction_attempted = false;
    let mut rate_limit_retries = 0u32;
    const MAX_RATE_LIMIT_RETRIES: u32 = 3;

    // Token usage tracking
    let mut session_usage = SessionUsage::new();

    // Thinking timer - uses raw mpsc sender (not stored/broadcast since it's high-frequency)
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
            break;
        }

        // Call LLM
        log(&format!("[{}] Calling LLM ({}) with {} messages",
            log_prefix,
            state.provider_info.resolved_model,
            conversation_history.len()
        ));

        let response = state
            .client
            .chat_completion(
                &state.provider_info.resolved_model,
                serde_json::to_value(&conversation_history).unwrap(),
                tools_option.as_deref(),
            )
            .await;

        let response = match response {
            Ok(r) => {
                log(&format!("[{}] LLM response received", log_prefix));
                r
            }
            Err(e) => {
                // Use {:#} to get full error chain from anyhow
                let error_msg = format!("{:#}", e);
                log(&format!("[{}] LLM error: {}", log_prefix, error_msg));

                // Check for rate limit errors first - these should be retried, not compacted
                if is_rate_limit_error(&error_msg) && rate_limit_retries < MAX_RATE_LIMIT_RETRIES {
                    rate_limit_retries += 1;
                    let delay = extract_retry_delay(&error_msg).unwrap_or(10).min(60);
                    log(&format!("[{}] Rate limit hit, retrying in {}s (attempt {}/{})",
                        log_prefix, delay, rate_limit_retries, MAX_RATE_LIMIT_RETRIES));

                    event_sender.send(SseEvent::Compacted {
                        message: format!("Rate limit hit. Retrying in {}s...", delay),
                    }).await;

                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }

                // Check if this is a context exhaustion error and we haven't tried compaction yet
                if is_context_exhausted_error(&error_msg) && !compaction_attempted {
                    compaction_attempted = true;
                    log(&format!("[{}] Attempting context compaction ({} messages)",
                        log_prefix, conversation_history.len()));

                    // Attempt compaction
                    match compact_context(
                        &state.client,
                        &state.provider_info.resolved_model,
                        &conversation_history,
                        &compaction_config,
                    ).await {
                        Ok(compacted) => {
                            let compaction_msg = if compacted.used_full_summarization {
                                format!("Context compacted via summarization ({} → {} messages, {:.1}%)",
                                    conversation_history.len(), compacted.messages.len(), compacted.compaction_ratio * 100.0)
                            } else {
                                format!("Context compacted via trimming ({} → {} messages, {:.1}%)",
                                    conversation_history.len(), compacted.messages.len(), compacted.compaction_ratio * 100.0)
                            };
                            log(&format!("[{}] {}", log_prefix, compaction_msg));

                            event_sender.send(SseEvent::Compacted {
                                message: compaction_msg,
                            }).await;

                            // Replace history with compacted version and retry
                            conversation_history = compacted.messages;
                            continue;
                        }
                        Err(compact_err) => {
                            log(&format!("[{}] Compaction failed: {:#}", log_prefix, compact_err));
                            event_sender.send(SseEvent::Error {
                                message: format!("Context exhausted and compaction failed: {:#}", compact_err),
                            }).await;
                            break;
                        }
                    }
                } else {
                    event_sender.send(SseEvent::Error {
                        message: format!("API error: {:#}", e),
                    }).await;
                    break;
                }
            }
        };

        // Track token usage from response
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

            // Check if this is an invoke call (multi-agent mode)
            let is_invoke = state.orchestrator.as_ref()
                .map(|o| o.is_invoke_tool(tool_name))
                .unwrap_or(false);

            let result = if is_invoke {
                // Handle agent invocation
                let target_agent = state.orchestrator.as_ref()
                    .and_then(|o| o.get_invoke_target(tool_name))
                    .unwrap_or("unknown");
                let task = args.get("task").and_then(|t| t.as_str()).unwrap_or("");

                log(&format!("[{}] Invoking agent '{}' with task: {}", log_prefix, target_agent, task));

                // Send agent invoke event
                event_sender.send(SseEvent::AgentInvoke {
                    agent_name: target_agent.to_string(),
                    task: task.to_string(),
                }).await;

                // Create display sink for the subagent
                let display = Arc::new(WebappDisplaySink::new(event_sender.tx_clone(), state.tool_output_limit));

                // Execute the invoke through orchestrator
                let invoke_result = if let (Some(ref orchestrator), Some(ref mut manager)) = (&state.orchestrator, mcp_guard.as_mut()) {
                    orchestrator.handle_invoke_webapp(
                        &state.client,
                        &state.provider_info.resolved_model,
                        tool_name,
                        &args,
                        manager,
                        state.tool_output_limit,
                        display,
                        0,  // depth
                        current_agent,
                    ).await
                } else {
                    "Error: Orchestrator not available".to_string()
                };

                // Send agent result event
                let (display_result, truncated) = if invoke_result.lines().count() > state.tool_output_limit && state.tool_output_limit > 0 {
                    let lines: Vec<&str> = invoke_result.lines().take(state.tool_output_limit).collect();
                    (lines.join("\n"), true)
                } else {
                    (invoke_result.clone(), false)
                };

                event_sender.send(SseEvent::AgentResult {
                    agent_name: target_agent.to_string(),
                    result: display_result,
                    truncated,
                }).await;

                invoke_result
            } else {
                // Regular tool call
                event_sender.send(SseEvent::ToolCall {
                    name: tool_name.clone(),
                    arguments: arguments.clone(),
                }).await;

                // Execute tool
                let tool_result = if tool_name == agent::INTERPRET_IMAGE_TOOL_NAME {
                    agent::execute_interpret_image(&state.client, &state.provider_info.resolved_model, args).await
                } else if tool_name == agent::SEARCH_QUERY_TOOL_NAME {
                    agent::execute_search_query(args).await
                } else if let Some(ref mut manager) = *mcp_guard {
                    manager.execute_tool(tool_name, args).await
                } else {
                    Ok("Error: No MCP manager available".to_string())
                }
                .unwrap_or_else(|e| format!("Error: {}", e));

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

                tool_result
            };

            // Add tool result to history
            conversation_history.push(Message::Tool {
                tool_call_id: tool_call.id.clone(),
                content: result,
            });
        }
    }

    // Stop thinking timer
    thinking_handle.abort();

    // Save updated history to storage and mark query as complete
    let _ = state.storage.set_history(&session_id, &conversation_history).await;
    state.storage.set_runtime_state(&session_id, Vec::new(), None, false).await;

    // Persist events to storage (for SQLite mode)
    for msg in &conversation_history {
        let (event_type, content) = match msg {
            Message::User { .. } => ("user_message", serde_json::to_string(msg).unwrap_or_default()),
            Message::Assistant { .. } => ("assistant_message", serde_json::to_string(msg).unwrap_or_default()),
            Message::Tool { .. } => ("tool_message", serde_json::to_string(msg).unwrap_or_default()),
        };
        let _ = state.storage.append_event(&session_id, event_type, &content).await;
    }

    log(&format!("[{}] Query complete, saved {} messages to session", log_prefix, conversation_history.len()));

    // Clear cancel sender
    {
        let mut cancel_guard = state.cancel_tx.lock().await;
        *cancel_guard = None;
    }

    // Send usage event if we have any usage data
    if session_usage.has_usage() {
        let estimated_cost = session_usage.estimate_cost(
            &state.provider_info.resolved_model,
            &state.provider_info.provider
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
}
