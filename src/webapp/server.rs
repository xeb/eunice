use crate::client::Client;
use crate::mcp::McpManager;
use crate::models::{ProviderInfo, WebappConfig};
use crate::orchestrator::AgentOrchestrator;
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

use super::handlers;

/// Shared application state
#[allow(dead_code)]
pub struct AppState {
    pub client: Arc<Client>,
    pub provider_info: ProviderInfo,
    pub mcp_manager: Arc<Mutex<Option<McpManager>>>,
    pub orchestrator: Option<Arc<AgentOrchestrator>>,
    pub agent_name: Option<String>,
    pub tool_output_limit: usize,
    pub verbose: bool,
    pub dmn: bool,
    pub research: bool,
    pub enable_image_tool: bool,
    pub enable_search_tool: bool,
    /// Active query cancellation sender
    pub cancel_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

/// Run the webapp server
pub async fn run_server(
    webapp_config: WebappConfig,
    client: Client,
    provider_info: ProviderInfo,
    mcp_manager: Option<McpManager>,
    orchestrator: Option<AgentOrchestrator>,
    agent_name: Option<String>,
    tool_output_limit: usize,
    verbose: bool,
    dmn: bool,
    research: bool,
    enable_image_tool: bool,
    enable_search_tool: bool,
) -> Result<()> {
    let state = Arc::new(AppState {
        client: Arc::new(client),
        provider_info,
        mcp_manager: Arc::new(Mutex::new(mcp_manager)),
        orchestrator: orchestrator.map(Arc::new),
        agent_name,
        tool_output_limit,
        verbose,
        dmn,
        research,
        enable_image_tool,
        enable_search_tool,
        cancel_tx: Arc::new(Mutex::new(None)),
    });

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/api/status", get(handlers::status))
        .route("/api/config", get(handlers::config))
        .route("/api/query", post(handlers::query))
        .route("/api/cancel", post(handlers::cancel))
        .with_state(state);

    let addr = format!("{}:{}", webapp_config.host, webapp_config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Starting webapp server at http://{}", addr);
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    println!("\nShutting down webapp server...");
}
