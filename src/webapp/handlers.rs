use super::server::AppState;
use crate::agent;
use crate::config::DMN_INSTRUCTIONS;
use crate::models::Message;
use axum::{
    extract::State,
    response::{Html, Sse},
    Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

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
    let (servers, tools_count) = if let Some(ref manager) = *mcp_manager {
        let info = manager.get_server_info();
        let servers: Vec<String> = info.iter().map(|(name, _, _)| name.clone()).collect();
        let tools: usize = info.iter().map(|(_, count, _)| count).sum();
        (servers, tools)
    } else {
        (vec![], 0)
    };

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
    let (servers, tools) = if let Some(ref manager) = *mcp_manager {
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
    let prompt = request.prompt;

    // Spawn agent task
    tokio::spawn(async move {
        run_agent_with_events(state_clone, prompt, tx, cancel_rx).await;
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
    tx: mpsc::Sender<SseEvent>,
    cancel_rx: watch::Receiver<bool>,
) {
    // Prepare the prompt with DMN instructions if needed
    let final_prompt = if state.dmn {
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

    // Build conversation history
    let mut conversation_history: Vec<Message> = vec![Message::User {
        content: final_prompt.clone(),
    }];

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
    loop {
        // Check for cancellation
        if *cancel_rx.borrow() {
            let _ = tx.send(SseEvent::Error {
                message: "Query cancelled".to_string(),
            }).await;
            break;
        }

        // Call LLM
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
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(SseEvent::Error {
                    message: format!("API error: {}", e),
                }).await;
                break;
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
                let _ = tx.send(SseEvent::Response {
                    content: content.clone(),
                }).await;
            }
        }

        // Check for tool calls
        let Some(tool_calls) = &choice.message.tool_calls else {
            break;
        };

        if tool_calls.is_empty() {
            break;
        }

        // Execute tool calls
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let arguments = &tool_call.function.arguments;

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

    // Clear cancel sender
    {
        let mut cancel_guard = state.cancel_tx.lock().await;
        *cancel_guard = None;
    }

    // Send done event
    let _ = tx.send(SseEvent::Done).await;
}
