use anyhow::{Context, Result};
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::super::servers::common::McpServer;

use super::handlers::{handle_delete, handle_get, handle_post, AppState};
use super::session::SessionManager;
use super::tls::TlsConfig;

/// HTTP server configuration
pub struct HttpServerConfig {
    pub port: u16,
    pub host: IpAddr,
    pub tls_enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub allowed_origins: Vec<String>,
    pub session_ttl: Duration,
    pub verbose: bool,
}

impl HttpServerConfig {
    pub fn new(
        port: u16,
        host: IpAddr,
        tls_enabled: bool,
        cert_path: Option<PathBuf>,
        key_path: Option<PathBuf>,
        origins: Option<String>,
        verbose: bool,
    ) -> Self {
        let allowed_origins = origins
            .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
            .unwrap_or_default();

        Self {
            port,
            host,
            tls_enabled,
            cert_path,
            key_path,
            allowed_origins,
            session_ttl: Duration::from_secs(3600), // 1 hour default
            verbose,
        }
    }
}

/// Run an MCP server over HTTP transport
pub async fn run_http_server<S: McpServer + Send + Sync + 'static>(
    mcp_server: S,
    config: HttpServerConfig,
) -> Result<()> {
    let addr = SocketAddr::new(config.host, config.port);

    // Print security warnings
    print_security_warnings(&config);

    // Create session manager
    let sessions = Arc::new(SessionManager::new(config.session_ttl));

    // Start session cleanup task
    sessions.clone().start_cleanup_task(Duration::from_secs(60));

    // Create app state
    let state = Arc::new(AppState::new(
        mcp_server,
        sessions,
        config.allowed_origins.clone(),
        config.verbose,
    ));

    // Build router
    let app = Router::new()
        .route("/mcp", post(handle_post::<S>))
        .route("/mcp", get(handle_get::<S>))
        .route("/mcp", delete(handle_delete::<S>))
        .with_state(state);

    if config.tls_enabled {
        run_https_server(app, addr, &config).await
    } else {
        run_http_server_plain(app, addr, &config).await
    }
}

/// Run plain HTTP server
async fn run_http_server_plain(
    app: Router,
    addr: SocketAddr,
    _config: &HttpServerConfig,
) -> Result<()> {
    eprintln!("[mcpz] Listening on http://{}/mcp", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}

/// Run HTTPS server with TLS
async fn run_https_server(
    app: Router,
    addr: SocketAddr,
    config: &HttpServerConfig,
) -> Result<()> {
    // Load or generate TLS config
    let tls_config = TlsConfig::load_or_generate(
        config.cert_path.as_deref(),
        config.key_path.as_deref(),
    )?;

    // Print certificate info
    if tls_config.is_self_signed {
        eprintln!("[mcpz] Using self-signed certificate");
        if let Ok(fingerprint) = tls_config.fingerprint() {
            eprintln!("[mcpz] Fingerprint: SHA256:{}", fingerprint);
        }
    } else {
        eprintln!(
            "[mcpz] Using certificate: {:?}",
            config.cert_path.as_ref().unwrap()
        );
    }

    eprintln!("[mcpz] Listening on https://{}/mcp", addr);

    // Build rustls config
    let rustls_config = tls_config.build_rustls_config()?;

    // Create TLS acceptor config for axum-server
    let tls_acceptor = axum_server::tls_rustls::RustlsConfig::from_config(rustls_config);

    // Run server
    axum_server::bind_rustls(addr, tls_acceptor)
        .serve(app.into_make_service())
        .await
        .context("HTTPS server error")?;

    Ok(())
}

/// Print security warnings based on configuration
fn print_security_warnings(config: &HttpServerConfig) {
    let is_localhost = config.host.is_loopback();

    if !is_localhost {
        eprintln!("WARNING: Binding to {} exposes this server to all network interfaces.", config.host);
        eprintln!("         Ensure proper authentication and firewall rules are in place.");

        if !config.tls_enabled {
            eprintln!("WARNING: Running without TLS on a public interface.");
            eprintln!("         Consider using --tls for encrypted connections.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_http_server_config_new() {
        let config = HttpServerConfig::new(
            3000,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            false,
            None,
            None,
            Some("https://example.com, https://other.com".to_string()),
            false,
        );

        assert_eq!(config.port, 3000);
        assert!(config.host.is_loopback());
        assert!(!config.tls_enabled);
        assert_eq!(config.allowed_origins.len(), 2);
        assert!(config.allowed_origins.contains(&"https://example.com".to_string()));
        assert!(config.allowed_origins.contains(&"https://other.com".to_string()));
    }

    #[test]
    fn test_http_server_config_no_origins() {
        let config = HttpServerConfig::new(
            8080,
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            true,
            None,
            None,
            None,
            true,
        );

        assert_eq!(config.port, 8080);
        assert!(!config.host.is_loopback());
        assert!(config.tls_enabled);
        assert!(config.allowed_origins.is_empty());
        assert!(config.verbose);
    }
}
