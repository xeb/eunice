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
use super::scheduler::{self, AgentRegistry};

/// Shared application state
#[allow(dead_code)]
pub struct AppState {
    pub client: Arc<Client>,
    pub provider_info: ProviderInfo,
    pub tool_registry: Arc<ToolRegistry>,
    pub tool_output_limit: usize,
    /// Active query cancellation sender
    pub cancel_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    /// Session storage (SQLite or in-memory)
    pub storage: SessionStorage,
    /// System prompt prepended to the first message of each new session
    pub system_prompt: Option<String>,
    /// Scheduled agents, when an agents file was supplied
    pub agents: Option<Arc<AgentRegistry>>,
}

/// Run the webapp server
pub async fn run_server(
    webapp_config: WebappConfig,
    client: Client,
    provider_info: ProviderInfo,
    system_prompt: Option<String>,
    agents: Option<crate::agents::AgentsConfig>,
) -> Result<()> {
    // Initialize storage: persistent sessions.db by default, in-memory
    // when --no-persist is set or the database cannot be opened
    let storage = if webapp_config.persist {
        match SessionStorage::new_sqlite("sessions.db") {
            Ok(s) => {
                println!("Session persistence: sessions.db");
                s
            }
            Err(e) => {
                eprintln!("Warning: could not open sessions.db ({}); sessions are in-memory only", e);
                SessionStorage::new_memory()
            }
        }
    } else {
        println!("Session persistence: in-memory");
        SessionStorage::new_memory()
    };

    // Create tool registry
    let tool_registry = ToolRegistry::new();
    let tool_count = tool_registry.get_tools().len();

    println!("Tools available: {}", tool_count);

    if let Some(ref sp) = system_prompt {
        println!("System prompt: {} chars", sp.len());
    }

    let agents = match agents {
        Some(config) => {
            let count = config.agents.len();
            let source = config.source_path.display().to_string();
            let registry = AgentRegistry::new(config, &provider_info.resolved_model)?;
            println!("Scheduled agents: {} (from {})", count, source);
            Some(Arc::new(registry))
        }
        None => None,
    };

    let state = Arc::new(AppState {
        client: Arc::new(client),
        provider_info,
        tool_registry: Arc::new(tool_registry),
        tool_output_limit: 50,
        cancel_tx: Arc::new(Mutex::new(None)),
        storage,
        system_prompt,
        agents,
    });

    scheduler::spawn(state.clone());

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
        .route("/api/agents", get(handlers::agents))
        .route("/api/agents/get", post(handlers::get_agent))
        .route("/api/agents/save", post(handlers::save_agent))
        .route("/api/agents/delete", post(handlers::delete_agent))
        .route("/api/agents/reload", post(handlers::reload_agents))
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
