use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
};
use futures::stream;
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use super::super::servers::common::{JsonRpcRequest, McpServer};

use super::session::{SessionError, SessionManager};

/// Custom header name for MCP session ID
pub const MCP_SESSION_ID_HEADER: &str = "mcp-session-id";

/// Application state shared across handlers
pub struct AppState<S: McpServer + Send + Sync + 'static> {
    pub mcp_server: Arc<S>,
    pub sessions: Arc<SessionManager>,
    pub allowed_origins: Vec<String>,
    pub verbose: bool,
}

impl<S: McpServer + Send + Sync + 'static> AppState<S> {
    pub fn new(
        mcp_server: S,
        sessions: Arc<SessionManager>,
        allowed_origins: Vec<String>,
        verbose: bool,
    ) -> Self {
        Self {
            mcp_server: Arc::new(mcp_server),
            sessions,
            allowed_origins,
            verbose,
        }
    }

    fn log(&self, message: &str) {
        if self.verbose {
            eprintln!("[mcpz] {}", message);
        }
    }
}

/// SSE event for streaming responses
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
struct SseEvent {
    data: String,
}

/// Validate Origin header to prevent DNS rebinding attacks
fn validate_origin(headers: &HeaderMap, allowed_origins: &[String]) -> Result<(), StatusCode> {
    // Get Origin header
    let origin = match headers.get(header::ORIGIN) {
        Some(o) => match o.to_str() {
            Ok(s) => s,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        },
        // No origin header - likely same-origin or non-browser client
        None => return Ok(()),
    };

    // Always allow localhost variants
    if origin.starts_with("http://localhost")
        || origin.starts_with("http://127.0.0.1")
        || origin.starts_with("https://localhost")
        || origin.starts_with("https://127.0.0.1")
    {
        return Ok(());
    }

    // Check against allowed list
    if allowed_origins.contains(&origin.to_string()) || allowed_origins.contains(&"*".to_string())
    {
        return Ok(());
    }

    Err(StatusCode::FORBIDDEN)
}

/// Extract session ID from headers
fn get_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(MCP_SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// POST /mcp - Handle JSON-RPC requests
pub async fn handle_post<S: McpServer + Send + Sync + 'static>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, StatusCode> {
    // 1. Validate Origin header
    validate_origin(&headers, &state.allowed_origins)?;

    state.log(&format!("POST /mcp: {}", body));

    // 2. Parse JSON-RPC request
    let request: JsonRpcRequest = serde_json::from_str(&body).map_err(|e| {
        state.log(&format!("Parse error: {}", e));
        StatusCode::BAD_REQUEST
    })?;

    // 3. Handle session
    let session_id = if request.method == "initialize" {
        // Create new session for initialize request
        let id = state.sessions.create_session().await;
        state.log(&format!("Created session: {}", id));
        id
    } else {
        // Validate existing session for all other requests
        let id = get_session_id(&headers).ok_or_else(|| {
            state.log("Missing session ID header");
            StatusCode::BAD_REQUEST
        })?;

        match state.sessions.validate_session(&id).await {
            Ok(()) => {
                state.sessions.touch_session(&id).await.ok();
                id
            }
            Err(SessionError::NotFound) => {
                state.log(&format!("Session not found: {}", id));
                return Err(StatusCode::NOT_FOUND);
            }
            Err(SessionError::Expired) => {
                state.log(&format!("Session expired: {}", id));
                return Err(StatusCode::NOT_FOUND);
            }
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    };

    // 4. Dispatch to MCP server
    let response = match state.mcp_server.handle_request(request) {
        Some(resp) => resp,
        None => {
            // Notification - no response needed
            state.log("Notification processed, no response");
            return Ok((
                StatusCode::ACCEPTED,
                [(MCP_SESSION_ID_HEADER, session_id)],
            )
                .into_response());
        }
    };

    // 5. Mark session as initialized after successful initialize
    if response.result.is_some() {
        if let Some(result) = &response.result {
            if result.get("protocolVersion").is_some() {
                // This is an initialize response
                state.sessions.mark_initialized(&session_id).await.ok();
                state.log(&format!("Session {} initialized", session_id));
            }
        }
    }

    // 6. Return JSON response with session ID header
    let response_json = serde_json::to_string(&response).map_err(|e| {
        state.log(&format!("Serialize error: {}", e));
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state.log(&format!("Response: {}", response_json));

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE.as_str(), "application/json"),
            (MCP_SESSION_ID_HEADER, &session_id),
        ],
        response_json,
    )
        .into_response())
}

/// GET /mcp - Open SSE stream for server-initiated messages
pub async fn handle_get<S: McpServer + Send + Sync + 'static>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Validate Origin
    validate_origin(&headers, &state.allowed_origins)?;

    // Validate session
    let session_id = get_session_id(&headers).ok_or(StatusCode::BAD_REQUEST)?;

    match state.sessions.validate_session(&session_id).await {
        Ok(()) => {
            state.sessions.touch_session(&session_id).await.ok();
        }
        Err(SessionError::NotFound | SessionError::Expired) => {
            return Err(StatusCode::NOT_FOUND);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    state.log(&format!("GET /mcp: SSE stream opened for session {}", session_id));

    // Return empty SSE stream (we don't have server-initiated messages yet)
    // The stream stays open but doesn't send anything
    let stream = stream::pending::<Result<axum::response::sse::Event, Infallible>>();

    Ok(Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("ping"),
        )
        .into_response())
}

/// DELETE /mcp - Terminate session
pub async fn handle_delete<S: McpServer + Send + Sync + 'static>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> StatusCode {
    // Validate Origin
    if validate_origin(&headers, &state.allowed_origins).is_err() {
        return StatusCode::FORBIDDEN;
    }

    // Get session ID
    let session_id = match get_session_id(&headers) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST,
    };

    // Delete session
    if state.sessions.delete_session(&session_id).await {
        state.log(&format!("DELETE /mcp: Session {} terminated", session_id));
        StatusCode::OK
    } else {
        state.log(&format!("DELETE /mcp: Session {} not found", session_id));
        StatusCode::NOT_FOUND
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_validate_origin_no_header() {
        let headers = HeaderMap::new();
        assert!(validate_origin(&headers, &vec![]).is_ok());
    }

    #[test]
    fn test_validate_origin_localhost() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://localhost:3000"),
        );
        assert!(validate_origin(&headers, &vec![]).is_ok());

        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:8080"),
        );
        assert!(validate_origin(&headers, &vec![]).is_ok());

        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://localhost"),
        );
        assert!(validate_origin(&headers, &vec![]).is_ok());
    }

    #[test]
    fn test_validate_origin_blocked() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.com"),
        );
        assert_eq!(
            validate_origin(&headers, &vec![]),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn test_validate_origin_allowed() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://myapp.com"),
        );
        let allowed = vec!["https://myapp.com".to_string()];
        assert!(validate_origin(&headers, &allowed).is_ok());
    }

    #[test]
    fn test_validate_origin_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://anything.com"),
        );
        let allowed = vec!["*".to_string()];
        assert!(validate_origin(&headers, &allowed).is_ok());
    }

    #[test]
    fn test_get_session_id() {
        let mut headers = HeaderMap::new();
        assert!(get_session_id(&headers).is_none());

        headers.insert(
            MCP_SESSION_ID_HEADER,
            HeaderValue::from_static("test-session-123"),
        );
        assert_eq!(
            get_session_id(&headers),
            Some("test-session-123".to_string())
        );
    }
}
