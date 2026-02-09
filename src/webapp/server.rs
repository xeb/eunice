use crate::client::Client;
use crate::models::{ProviderInfo, WebappConfig};
use crate::tools::ToolRegistry;
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

use super::handlers;
use super::persistence::SessionStorage;

/// Shared application state
#[allow(dead_code)]
pub struct AppState {
    pub client: Arc<Client>,
    pub provider_info: ProviderInfo,
    pub tool_registry: Arc<ToolRegistry>,
    pub tool_output_limit: usize,
    pub verbose: bool,
    /// Active query cancellation sender
    pub cancel_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    /// Session storage (SQLite or in-memory)
    pub storage: SessionStorage,
}

/// Run the webapp server
pub async fn run_server(
    webapp_config: WebappConfig,
    client: Client,
    provider_info: ProviderInfo,
    verbose: bool,
) -> Result<()> {
    // Initialize storage (always use in-memory for v1.0.0)
    let storage = SessionStorage::new(false)?;

    println!("Session persistence: in-memory");

    // Create tool registry
    let tool_registry = ToolRegistry::new();
    let tool_count = tool_registry.get_tools().len();

    println!("Tools available: {}", tool_count);

    let state = Arc::new(AppState {
        client: Arc::new(client),
        provider_info,
        tool_registry: Arc::new(tool_registry),
        tool_output_limit: 50,
        verbose,
        cancel_tx: Arc::new(Mutex::new(None)),
        storage,
    });

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/api/status", get(handlers::status))
        .route("/api/config", get(handlers::config))
        .route("/api/query", post(handlers::query))
        .route("/api/cancel", post(handlers::cancel))
        .route("/api/sessions", get(handlers::list_sessions))
        .route("/api/session/new", post(handlers::new_session))
        .route("/api/session/delete", post(handlers::delete_session))
        .route("/api/session/history", post(handlers::get_session_history))
        .route("/api/session/clear", post(handlers::clear_session))
        .route("/api/session/events", post(handlers::session_events))
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
