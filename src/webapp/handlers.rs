use super::server::{AppState, Session};
use crate::agent;
use crate::compact::{compact_context, is_context_exhausted_error, CompactionConfig};
use crate::config::DMN_INSTRUCTIONS;
use crate::models::Message;
use axum::{
    extract::State,
    response::{Html, Sse},
    Json,
};
use chrono::Local;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

/// Log a webapp event with timestamp
fn log(event: &str) {
    let timestamp = Local::now().format("%H:%M:%S%.3f");
    println!("[{}] [webapp] {}", timestamp, event);
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
}

/// Status endpoint handler
pub async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
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

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: state.provider_info.resolved_model.clone(),
        provider: state.provider_info.provider.to_string(),
        mode: mode.to_string(),
        agent: state.agent_name.clone(),
        mcp_servers: servers,
        tools_count,
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
}

/// Create a new session
pub async fn new_session(State(state): State<Arc<AppState>>) -> Json<NewSessionResponse> {
    let session_id = Uuid::new_v4().to_string();

    // Create session in store
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), Session::new());
    }

    log(&format!("New session created: {}", &session_id[..8]));
    Json(NewSessionResponse { session_id })
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
    State(state): State<Arc<AppState>>,
    Json(request): Json<SessionHistoryRequest>,
) -> Json<SessionHistoryResponse> {
    let sessions = state.sessions.read().await;

    if let Some(session) = sessions.get(&request.session_id) {
        let messages: Vec<HistoryMessage> = session.history.iter().map(|msg| {
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
                    let name = session.history.iter()
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
    } else {
        Json(SessionHistoryResponse {
            exists: false,
            messages: vec![],
        })
    }
}

/// SSE event types
#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Thinking { elapsed_seconds: u64 },
    ToolCall { name: String, arguments: String },
    ToolResult { name: String, result: String, truncated: bool },
    Response { content: String },
    Error { message: String },
    /// Session ID confirmation (sent at start of query)
    SessionId { session_id: String },
    /// Context was compacted due to size limits
    Compacted { message: String },
    Done,
}

/// Query endpoint handler - returns SSE stream
pub async fn query(
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

    // Log incoming query
    let prompt_preview = if request.prompt.len() > 100 {
        format!("{}...", &request.prompt[..100])
    } else {
        request.prompt.clone()
    };
    log(&format!("Query received: \"{}\"", prompt_preview));

    // Get or create session
    let session_id = match request.session_id {
        Some(id) => {
            // Verify session exists, create if not
            let sessions = state.sessions.read().await;
            if sessions.contains_key(&id) {
                log(&format!("Using existing session: {}", &id[..8]));
                id
            } else {
                drop(sessions);
                let new_id = Uuid::new_v4().to_string();
                let mut sessions = state.sessions.write().await;
                sessions.insert(new_id.clone(), Session::new());
                log(&format!("Session {} not found, created new: {}", &id[..8], &new_id[..8]));
                new_id
            }
        }
        None => {
            // Create new session
            let new_id = Uuid::new_v4().to_string();
            let mut sessions = state.sessions.write().await;
            sessions.insert(new_id.clone(), Session::new());
            log(&format!("No session provided, created new: {}", &new_id[..8]));
            new_id
        }
    };

    // Spawn agent task
    tokio::spawn(async move {
        run_agent_with_events(state_clone, prompt, session_id, tx, cancel_rx).await;
    });

    // Convert channel to SSE stream
    let stream = ReceiverStream::new(rx).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        let event_type = match &event {
            SseEvent::Thinking { .. } => "thinking",
            SseEvent::ToolCall { .. } => "tool_call",
            SseEvent::ToolResult { .. } => "tool_result",
            SseEvent::Response { .. } => "response",
            SseEvent::Error { .. } => "error",
            SseEvent::SessionId { .. } => "session_id",
            SseEvent::Compacted { .. } => "compacted",
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
    tx: mpsc::Sender<SseEvent>,
    cancel_rx: watch::Receiver<bool>,
) {
    let session_short = &session_id[..8];
    log(&format!("[{}] Agent loop starting", session_short));

    // Send session ID to client first
    let _ = tx.send(SseEvent::SessionId {
        session_id: session_id.clone(),
    }).await;

    // Load existing history from session store
    let incoming_history = {
        let sessions = state.sessions.read().await;
        sessions.get(&session_id)
            .map(|s| s.history.clone())
            .unwrap_or_default()
    };
    log(&format!("[{}] Loaded {} messages from history", session_short, incoming_history.len()));

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

    let tools_option = if tools.is_empty() { None } else { Some(tools) };

    // Build conversation history from incoming history + new user message
    let mut conversation_history: Vec<Message> = incoming_history;
    conversation_history.push(Message::User {
        content: final_prompt.clone(),
    });

    // Compaction config
    let compaction_config = CompactionConfig::default();
    let mut compaction_attempted = false;

    // Thinking timer
    let tx_thinking = tx.clone();
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
        log(&format!("[{}] Loop iteration {}", session_short, loop_iteration));

        // Check for cancellation
        if *cancel_rx.borrow() {
            log(&format!("[{}] Query cancelled by user", session_short));
            let _ = tx.send(SseEvent::Error {
                message: "Query cancelled".to_string(),
            }).await;
            break;
        }

        // Call LLM
        log(&format!("[{}] Calling LLM ({}) with {} messages",
            session_short,
            state.provider_info.resolved_model,
            conversation_history.len()
        ));

        let response = state
            .client
            .chat_completion(
                &state.provider_info.resolved_model,
                serde_json::to_value(&conversation_history).unwrap(),
                tools_option.as_deref(),
                state.enable_image_tool,
            )
            .await;

        let response = match response {
            Ok(r) => {
                log(&format!("[{}] LLM response received", session_short));
                r
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                log(&format!("[{}] LLM error: {}", session_short, error_msg));

                // Check if this is a context exhaustion error and we haven't tried compaction yet
                if is_context_exhausted_error(&error_msg) && !compaction_attempted {
                    compaction_attempted = true;
                    log(&format!("[{}] Attempting context compaction", session_short));

                    // Attempt compaction
                    match compact_context(
                        &state.client,
                        &state.provider_info.resolved_model,
                        &conversation_history,
                        &compaction_config,
                    ).await {
                        Ok(compacted) => {
                            let compaction_msg = if compacted.used_full_summarization {
                                format!("Context compacted via summarization (ratio: {:.1}%)", compacted.compaction_ratio * 100.0)
                            } else {
                                format!("Context compacted via truncation (ratio: {:.1}%)", compacted.compaction_ratio * 100.0)
                            };
                            log(&format!("[{}] {}", session_short, compaction_msg));

                            let _ = tx.send(SseEvent::Compacted {
                                message: compaction_msg,
                            }).await;

                            // Replace history with compacted version and retry
                            conversation_history = compacted.messages;
                            continue;
                        }
                        Err(compact_err) => {
                            log(&format!("[{}] Compaction failed: {}", session_short, compact_err));
                            let _ = tx.send(SseEvent::Error {
                                message: format!("Context exhausted and compaction failed: {}", compact_err),
                            }).await;
                            break;
                        }
                    }
                } else {
                    let _ = tx.send(SseEvent::Error {
                        message: format!("API error: {}", e),
                    }).await;
                    break;
                }
            }
        };

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
                log(&format!("[{}] Response: \"{}\"", session_short, content_preview.replace('\n', " ")));
                let _ = tx.send(SseEvent::Response {
                    content: content.clone(),
                }).await;
            }
        }

        // Check for tool calls
        let Some(tool_calls) = &choice.message.tool_calls else {
            log(&format!("[{}] No tool calls, loop complete", session_short));
            break;
        };

        if tool_calls.is_empty() {
            log(&format!("[{}] Empty tool calls, loop complete", session_short));
            break;
        }

        log(&format!("[{}] Processing {} tool call(s)", session_short, tool_calls.len()));

        // Execute tool calls
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let arguments = &tool_call.function.arguments;

            log(&format!("[{}] Tool call: {}", session_short, tool_name));

            // Send tool call event
            let _ = tx.send(SseEvent::ToolCall {
                name: tool_name.clone(),
                arguments: arguments.clone(),
            }).await;

            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

            // Execute tool
            let result = if tool_name == agent::INTERPRET_IMAGE_TOOL_NAME {
                agent::execute_interpret_image(&state.client, &state.provider_info.resolved_model, args).await
            } else if tool_name == agent::SEARCH_QUERY_TOOL_NAME {
                agent::execute_search_query(args).await
            } else if let Some(ref mut manager) = *mcp_guard {
                manager.execute_tool(tool_name, args).await
            } else {
                Ok("Error: No MCP manager available".to_string())
            }
            .unwrap_or_else(|e| format!("Error: {}", e));

            let result_preview = if result.len() > 80 {
                format!("{}...", &result[..80])
            } else {
                result.clone()
            };
            log(&format!("[{}] Tool result: {} chars ({})", session_short, result.len(), result_preview.replace('\n', " ")));

            // Truncate result for display
            let (display_result, truncated) = if result.lines().count() > state.tool_output_limit && state.tool_output_limit > 0 {
                let lines: Vec<&str> = result.lines().take(state.tool_output_limit).collect();
                (lines.join("\n"), true)
            } else {
                (result.clone(), false)
            };

            // Send tool result event
            let _ = tx.send(SseEvent::ToolResult {
                name: tool_name.clone(),
                result: display_result,
                truncated,
            }).await;

            // Add tool result to history
            conversation_history.push(Message::Tool {
                tool_call_id: tool_call.id.clone(),
                content: result,
            });
        }
    }

    // Stop thinking timer
    thinking_handle.abort();

    // Save updated history to session store
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.history = conversation_history.clone();
        }
    }

    log(&format!("[{}] Query complete, saved {} messages to session", session_short, conversation_history.len()));

    // Clear cancel sender
    {
        let mut cancel_guard = state.cancel_tx.lock().await;
        *cancel_guard = None;
    }

    // Send done event
    let _ = tx.send(SseEvent::Done).await;
}
