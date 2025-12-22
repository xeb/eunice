use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session state
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub initialized: bool,
}

impl Session {
    fn new(id: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            created_at: now,
            last_activity: now,
            initialized: false,
        }
    }
}

/// Error type for session operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found")]
    NotFound,
    #[error("Session expired")]
    Expired,
    #[error("Session not initialized")]
    NotInitialized,
}

/// Session manager for tracking MCP sessions
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    ttl: Duration,
}

impl SessionManager {
    /// Create a new session manager with the specified TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Create a new session and return its ID
    pub async fn create_session(&self) -> String {
        let id = Uuid::new_v4().to_string();
        let session = Session::new(id.clone());

        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), session);

        id
    }

    /// Validate that a session exists and is not expired
    pub async fn validate_session(&self, id: &str) -> Result<(), SessionError> {
        let sessions = self.sessions.read().await;

        match sessions.get(id) {
            Some(session) => {
                if session.last_activity.elapsed() > self.ttl {
                    Err(SessionError::Expired)
                } else {
                    Ok(())
                }
            }
            None => Err(SessionError::NotFound),
        }
    }

    /// Mark a session as initialized
    pub async fn mark_initialized(&self, id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(id) {
            Some(session) => {
                session.initialized = true;
                session.last_activity = Instant::now();
                Ok(())
            }
            None => Err(SessionError::NotFound),
        }
    }

    /// Check if a session is initialized
    pub async fn is_initialized(&self, id: &str) -> Result<bool, SessionError> {
        let sessions = self.sessions.read().await;

        match sessions.get(id) {
            Some(session) => Ok(session.initialized),
            None => Err(SessionError::NotFound),
        }
    }

    /// Update the last activity time for a session
    pub async fn touch_session(&self, id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(id) {
            Some(session) => {
                session.last_activity = Instant::now();
                Ok(())
            }
            None => Err(SessionError::NotFound),
        }
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id).is_some()
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();

        sessions.retain(|_, session| session.last_activity.elapsed() <= self.ttl);

        before - sessions.len()
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Start a background task to periodically clean up expired sessions
    pub fn start_cleanup_task(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                let cleaned = self.cleanup_expired().await;
                if cleaned > 0 {
                    eprintln!("[mcpz] Cleaned up {} expired sessions", cleaned);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::new(Duration::from_secs(300));
        let id = manager.create_session().await;
        assert!(!id.is_empty());
        assert!(manager.validate_session(&id).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_nonexistent_session() {
        let manager = SessionManager::new(Duration::from_secs(300));
        let result = manager.validate_session("nonexistent").await;
        assert!(matches!(result, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let manager = SessionManager::new(Duration::from_secs(300));
        let id = manager.create_session().await;

        assert!(manager.delete_session(&id).await);
        assert!(!manager.delete_session(&id).await); // Already deleted

        let result = manager.validate_session(&id).await;
        assert!(matches!(result, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn test_touch_session() {
        let manager = SessionManager::new(Duration::from_secs(300));
        let id = manager.create_session().await;

        // Touch should succeed
        assert!(manager.touch_session(&id).await.is_ok());

        // Touch nonexistent should fail
        assert!(matches!(
            manager.touch_session("nonexistent").await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_session_initialization() {
        let manager = SessionManager::new(Duration::from_secs(300));
        let id = manager.create_session().await;

        // Should not be initialized initially
        assert!(!manager.is_initialized(&id).await.unwrap());

        // Mark as initialized
        manager.mark_initialized(&id).await.unwrap();

        // Should now be initialized
        assert!(manager.is_initialized(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_session_count() {
        let manager = SessionManager::new(Duration::from_secs(300));

        assert_eq!(manager.session_count().await, 0);

        let id1 = manager.create_session().await;
        assert_eq!(manager.session_count().await, 1);

        let _id2 = manager.create_session().await;
        assert_eq!(manager.session_count().await, 2);

        manager.delete_session(&id1).await;
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_expired_session() {
        // Create manager with very short TTL
        let manager = SessionManager::new(Duration::from_millis(10));
        let id = manager.create_session().await;

        // Should be valid immediately
        assert!(manager.validate_session(&id).await.is_ok());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be expired now
        let result = manager.validate_session(&id).await;
        assert!(matches!(result, Err(SessionError::Expired)));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let manager = SessionManager::new(Duration::from_millis(10));

        // Create some sessions
        manager.create_session().await;
        manager.create_session().await;
        assert_eq!(manager.session_count().await, 2);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Clean up
        let cleaned = manager.cleanup_expired().await;
        assert_eq!(cleaned, 2);
        assert_eq!(manager.session_count().await, 0);
    }
}
