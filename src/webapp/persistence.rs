//! Session persistence layer for the webapp.
//!
//! Provides SQLite-based session storage when mcpz is installed,
//! with fallback to in-memory storage otherwise.

use anyhow::Result;
use rand::seq::SliceRandom;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::models::Message;

use super::handlers::SseEvent;

/// Cyberpunk adjectives (William Gibson inspired)
const ADJECTIVES: &[&str] = &[
    "chrome", "neon", "black", "ice", "sprawl", "ghost", "razor", "wire",
    "synth", "void", "grid", "pulse", "cyber", "night", "data", "flux",
    "zero", "null", "vector", "matrix", "edge", "shadow", "burn", "rust",
    "static", "echo", "signal", "glitch", "proxy", "node", "hex", "byte",
];

/// Cyberpunk nouns (William Gibson inspired)
const NOUNS: &[&str] = &[
    "molly", "case", "armitage", "wintermute", "neuromancer", "construct",
    "runner", "cowboy", "jockey", "samurai", "ronin", "decker", "console",
    "sprawl", "chiba", "zaibatsu", "yakuza", "orbital", "spindle", "nexus",
    "flatline", "icebreaker", "daemon", "phantom", "spectre", "wraith",
    "voodoo", "hoodoo", "mojo", "chrome", "mirrorshades", "sendai",
];

/// Session record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub name: String,
    pub user_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Session metadata for list display
#[derive(Debug, Clone, Serialize)]
pub struct SessionMetadata {
    pub id: String,
    pub name: String,
    pub turn_count: usize,
    pub updated_at: i64,
    pub relative_time: String,
}

/// In-memory session for runtime use
pub struct MemorySession {
    pub id: String,
    pub name: String,
    pub user_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub history: Vec<Message>,
    pub events: Vec<SseEvent>,
    pub event_tx: Option<broadcast::Sender<SseEvent>>,
    pub query_running: bool,
}

impl MemorySession {
    pub fn new(id: String, name: String, user_id: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            name,
            user_id,
            created_at: now,
            updated_at: now,
            history: Vec::new(),
            events: Vec::new(),
            event_tx: None,
            query_running: false,
        }
    }
}

/// Storage backend abstraction
pub enum SessionStorage {
    /// In-memory storage (fallback when mcpz not available)
    Memory(Arc<RwLock<HashMap<String, MemorySession>>>),
    /// SQLite storage (when mcpz is available)
    Sqlite {
        conn: Arc<Mutex<Connection>>,
        /// Runtime state (events, broadcast channels) still in memory
        runtime: Arc<RwLock<HashMap<String, RuntimeState>>>,
    },
}

/// Runtime state for SQLite sessions (not persisted)
pub struct RuntimeState {
    pub events: Vec<SseEvent>,
    pub event_tx: Option<broadcast::Sender<SseEvent>>,
    pub query_running: bool,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            event_tx: None,
            query_running: false,
        }
    }
}

impl SessionStorage {
    /// Initialize storage based on mcpz availability
    pub fn new(mcpz_available: bool) -> Result<Self> {
        if mcpz_available {
            let conn = Connection::open("sessions.db")?;
            Self::init_schema(&conn)?;
            Ok(SessionStorage::Sqlite {
                conn: Arc::new(Mutex::new(conn)),
                runtime: Arc::new(RwLock::new(HashMap::new())),
            })
        } else {
            Ok(SessionStorage::Memory(Arc::new(RwLock::new(HashMap::new()))))
        }
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                user_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                sequence_num INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(session_id, sequence_num)
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id, sequence_num);
            "#,
        )?;
        Ok(())
    }

    /// Check if using SQLite persistence
    pub fn is_persistent(&self) -> bool {
        matches!(self, SessionStorage::Sqlite { .. })
    }

    /// Generate a unique cyberpunk session name
    fn generate_name(conn: &Connection) -> Result<String> {
        let mut rng = rand::thread_rng();

        for _ in 0..100 {
            let adj = ADJECTIVES.choose(&mut rng).unwrap();
            let noun = NOUNS.choose(&mut rng).unwrap();
            let name = format!("{}-{}", adj, noun);

            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE name = ?)",
                [&name],
                |row| row.get(0),
            )?;

            if !exists {
                return Ok(name);
            }
        }

        // Fallback: append random suffix
        let adj = ADJECTIVES.choose(&mut rng).unwrap();
        let noun = NOUNS.choose(&mut rng).unwrap();
        let suffix = &uuid::Uuid::new_v4().to_string()[..4];
        Ok(format!("{}-{}-{}", adj, noun, suffix))
    }

    /// Generate a name for in-memory sessions (no uniqueness check needed)
    fn generate_memory_name() -> String {
        let mut rng = rand::thread_rng();
        let adj = ADJECTIVES.choose(&mut rng).unwrap();
        let noun = NOUNS.choose(&mut rng).unwrap();
        format!("{}-{}", adj, noun)
    }

    /// Create a new session
    pub async fn create_session(&self, user_id: Option<&str>) -> Result<SessionRecord> {
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();

        match self {
            SessionStorage::Memory(store) => {
                let name = Self::generate_memory_name();
                let session = MemorySession::new(id.clone(), name.clone(), user_id.map(String::from));
                store.write().await.insert(id.clone(), session);
                Ok(SessionRecord {
                    id,
                    name,
                    user_id: user_id.map(String::from),
                    created_at: now,
                    updated_at: now,
                })
            }
            SessionStorage::Sqlite { conn, runtime } => {
                let conn = conn.lock().await;
                let name = Self::generate_name(&conn)?;
                conn.execute(
                    "INSERT INTO sessions (id, name, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                    params![&id, &name, user_id, now, now],
                )?;
                runtime.write().await.insert(id.clone(), RuntimeState::new());
                Ok(SessionRecord {
                    id,
                    name,
                    user_id: user_id.map(String::from),
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                Ok(store.get(session_id).map(|s| SessionRecord {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    user_id: s.user_id.clone(),
                    created_at: s.created_at,
                    updated_at: s.updated_at,
                }))
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let mut stmt = conn.prepare(
                    "SELECT id, name, user_id, created_at, updated_at FROM sessions WHERE id = ?",
                )?;
                let result = stmt.query_row([session_id], |row| {
                    Ok(SessionRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        user_id: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                });
                match result {
                    Ok(record) => Ok(Some(record)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// List sessions for a user (or all if user_id is None)
    pub async fn list_sessions(&self, user_id: Option<&str>) -> Result<Vec<SessionMetadata>> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                let now = chrono::Utc::now().timestamp();
                let mut sessions: Vec<SessionMetadata> = store
                    .values()
                    .filter(|s| {
                        user_id.is_none() || s.user_id.as_deref() == user_id
                    })
                    .map(|s| {
                        let turn_count = s.history.iter().filter(|m| {
                            matches!(m, Message::User { .. } | Message::Assistant { .. })
                        }).count();
                        SessionMetadata {
                            id: s.id.clone(),
                            name: s.name.clone(),
                            turn_count,
                            updated_at: s.updated_at,
                            relative_time: format_relative_time(s.updated_at, now),
                        }
                    })
                    .collect();
                sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                Ok(sessions)
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let now = chrono::Utc::now().timestamp();

                let mut sessions = Vec::new();

                if let Some(uid) = user_id {
                    let mut stmt = conn.prepare(
                        "SELECT s.id, s.name, s.updated_at,
                                (SELECT COUNT(*) FROM events e WHERE e.session_id = s.id
                                 AND e.event_type IN ('user_message', 'assistant_message')) as turn_count
                         FROM sessions s WHERE s.user_id = ? ORDER BY s.updated_at DESC"
                    )?;
                    let rows = stmt.query_map([uid], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, name, updated_at, turn_count) = row?;
                        sessions.push(SessionMetadata {
                            id,
                            name,
                            turn_count: turn_count as usize,
                            updated_at,
                            relative_time: format_relative_time(updated_at, now),
                        });
                    }
                } else {
                    let mut stmt = conn.prepare(
                        "SELECT s.id, s.name, s.updated_at,
                                (SELECT COUNT(*) FROM events e WHERE e.session_id = s.id
                                 AND e.event_type IN ('user_message', 'assistant_message')) as turn_count
                         FROM sessions s ORDER BY s.updated_at DESC"
                    )?;
                    let rows = stmt.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, name, updated_at, turn_count) = row?;
                        sessions.push(SessionMetadata {
                            id,
                            name,
                            turn_count: turn_count as usize,
                            updated_at,
                            relative_time: format_relative_time(updated_at, now),
                        });
                    }
                }

                Ok(sessions)
            }
        }
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                Ok(store.remove(session_id).is_some())
            }
            SessionStorage::Sqlite { conn, runtime } => {
                let conn = conn.lock().await;
                let deleted = conn.execute("DELETE FROM sessions WHERE id = ?", [session_id])?;
                runtime.write().await.remove(session_id);
                Ok(deleted > 0)
            }
        }
    }

    /// Append an event to a session
    pub async fn append_event(&self, session_id: &str, event_type: &str, content: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.updated_at = now;
                }
                Ok(())
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                // Get next sequence number
                let seq: i64 = conn.query_row(
                    "SELECT COALESCE(MAX(sequence_num), 0) + 1 FROM events WHERE session_id = ?",
                    [session_id],
                    |row| row.get(0),
                )?;
                conn.execute(
                    "INSERT INTO events (session_id, event_type, sequence_num, content, created_at) VALUES (?, ?, ?, ?, ?)",
                    params![session_id, event_type, seq, content, now],
                )?;
                conn.execute(
                    "UPDATE sessions SET updated_at = ? WHERE id = ?",
                    params![now, session_id],
                )?;
                Ok(())
            }
        }
    }

    /// Get conversation history for a session
    pub async fn get_history(&self, session_id: &str) -> Result<Vec<Message>> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                Ok(store.get(session_id).map(|s| s.history.clone()).unwrap_or_default())
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let mut stmt = conn.prepare(
                    "SELECT event_type, content FROM events WHERE session_id = ?
                     AND event_type IN ('user_message', 'assistant_message', 'tool_message')
                     ORDER BY sequence_num ASC",
                )?;
                let rows = stmt.query_map([session_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;

                let mut history = Vec::new();
                for row in rows {
                    let (event_type, content) = row?;
                    if let Ok(msg) = serde_json::from_str::<Message>(&content) {
                        history.push(msg);
                    } else if event_type == "user_message" {
                        history.push(Message::User { content });
                    }
                }
                Ok(history)
            }
        }
    }

    /// Update history for a session (for in-memory, or save to SQLite)
    pub async fn set_history(&self, session_id: &str, history: &[Message]) -> Result<()> {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.history = history.to_vec();
                    session.updated_at = chrono::Utc::now().timestamp();
                }
                Ok(())
            }
            SessionStorage::Sqlite { conn, .. } => {
                // For SQLite, history is derived from events
                // Just update the timestamp
                let now = chrono::Utc::now().timestamp();
                let conn = conn.lock().await;
                conn.execute(
                    "UPDATE sessions SET updated_at = ? WHERE id = ?",
                    params![now, session_id],
                )?;
                Ok(())
            }
        }
    }

    /// Get runtime state (events, broadcast channel, query_running)
    pub async fn get_runtime_state(&self, session_id: &str) -> Option<(Vec<SseEvent>, Option<broadcast::Sender<SseEvent>>, bool)> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                store.get(session_id).map(|s| {
                    (s.events.clone(), s.event_tx.clone(), s.query_running)
                })
            }
            SessionStorage::Sqlite { runtime, .. } => {
                let runtime = runtime.read().await;
                runtime.get(session_id).map(|s| {
                    (s.events.clone(), s.event_tx.clone(), s.query_running)
                })
            }
        }
    }

    /// Set runtime state for a session
    pub async fn set_runtime_state(
        &self,
        session_id: &str,
        events: Vec<SseEvent>,
        event_tx: Option<broadcast::Sender<SseEvent>>,
        query_running: bool,
    ) {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.events = events;
                    session.event_tx = event_tx;
                    session.query_running = query_running;
                }
            }
            SessionStorage::Sqlite { runtime, .. } => {
                let mut runtime = runtime.write().await;
                let state = runtime.entry(session_id.to_string()).or_insert_with(RuntimeState::new);
                state.events = events;
                state.event_tx = event_tx;
                state.query_running = query_running;
            }
        }
    }

    /// Push an event to runtime state
    pub async fn push_runtime_event(&self, session_id: &str, event: SseEvent) {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.events.push(event);
                }
            }
            SessionStorage::Sqlite { runtime, .. } => {
                let mut runtime = runtime.write().await;
                if let Some(state) = runtime.get_mut(session_id) {
                    state.events.push(event);
                }
            }
        }
    }

    /// Clear runtime events
    pub async fn clear_runtime_events(&self, session_id: &str) {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.events.clear();
                }
            }
            SessionStorage::Sqlite { runtime, .. } => {
                let mut runtime = runtime.write().await;
                if let Some(state) = runtime.get_mut(session_id) {
                    state.events.clear();
                }
            }
        }
    }

    /// Ensure a session exists (create if not)
    pub async fn ensure_session(&self, session_id: &str, user_id: Option<&str>) -> Result<SessionRecord> {
        if let Some(session) = self.get_session(session_id).await? {
            return Ok(session);
        }

        // Session doesn't exist, create it with the given ID
        let now = chrono::Utc::now().timestamp();

        match self {
            SessionStorage::Memory(store) => {
                let name = Self::generate_memory_name();
                let session = MemorySession::new(session_id.to_string(), name.clone(), user_id.map(String::from));
                store.write().await.insert(session_id.to_string(), session);
                Ok(SessionRecord {
                    id: session_id.to_string(),
                    name,
                    user_id: user_id.map(String::from),
                    created_at: now,
                    updated_at: now,
                })
            }
            SessionStorage::Sqlite { conn, runtime } => {
                let conn = conn.lock().await;
                let name = Self::generate_name(&conn)?;
                conn.execute(
                    "INSERT INTO sessions (id, name, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                    params![session_id, &name, user_id, now, now],
                )?;
                runtime.write().await.insert(session_id.to_string(), RuntimeState::new());
                Ok(SessionRecord {
                    id: session_id.to_string(),
                    name,
                    user_id: user_id.map(String::from),
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }

    /// Get or create session for authenticated user
    pub async fn get_or_create_user_session(&self, user_id: &str) -> Result<SessionRecord> {
        match self {
            SessionStorage::Memory(store) => {
                // Check if user has any session
                {
                    let store = store.read().await;
                    for session in store.values() {
                        if session.user_id.as_deref() == Some(user_id) {
                            return Ok(SessionRecord {
                                id: session.id.clone(),
                                name: session.name.clone(),
                                user_id: session.user_id.clone(),
                                created_at: session.created_at,
                                updated_at: session.updated_at,
                            });
                        }
                    }
                }
                // Create new session for user
                self.create_session(Some(user_id)).await
            }
            SessionStorage::Sqlite { conn, runtime } => {
                let conn_guard = conn.lock().await;
                // Find most recent session for user
                let result = conn_guard.query_row(
                    "SELECT id, name, user_id, created_at, updated_at FROM sessions
                     WHERE user_id = ? ORDER BY updated_at DESC LIMIT 1",
                    [user_id],
                    |row| {
                        Ok(SessionRecord {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            user_id: row.get(2)?,
                            created_at: row.get(3)?,
                            updated_at: row.get(4)?,
                        })
                    },
                );

                match result {
                    Ok(record) => {
                        // Ensure runtime state exists
                        runtime.write().await.entry(record.id.clone()).or_insert_with(RuntimeState::new);
                        Ok(record)
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        drop(conn_guard);
                        self.create_session(Some(user_id)).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// Clear history for a user's session (for NEW button with authenticated users)
    pub async fn clear_user_session(&self, user_id: &str) -> Result<()> {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                for session in store.values_mut() {
                    if session.user_id.as_deref() == Some(user_id) {
                        session.history.clear();
                        session.events.clear();
                        session.updated_at = chrono::Utc::now().timestamp();
                        break;
                    }
                }
                Ok(())
            }
            SessionStorage::Sqlite { conn, runtime } => {
                // For SQLite, we create a new session instead of clearing
                // This preserves the old session in history
                let conn = conn.lock().await;
                if let Ok(old_id) = conn.query_row::<String, _, _>(
                    "SELECT id FROM sessions WHERE user_id = ? ORDER BY updated_at DESC LIMIT 1",
                    [user_id],
                    |row| row.get(0),
                ) {
                    runtime.write().await.remove(&old_id);
                }
                drop(conn);
                // Create new session for user
                self.create_session(Some(user_id)).await?;
                Ok(())
            }
        }
    }
}

/// Format a Unix timestamp as relative time
pub fn format_relative_time(timestamp: i64, now: i64) -> String {
    let diff = now - timestamp;

    match diff {
        d if d < 0 => "just now".to_string(),
        d if d < 60 => "just now".to_string(),
        d if d < 3600 => {
            let mins = d / 60;
            if mins == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", mins)
            }
        }
        d if d < 86400 => {
            let hours = d / 3600;
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{} hours ago", hours)
            }
        }
        d if d < 604800 => {
            let days = d / 86400;
            if days == 1 {
                "1 day ago".to_string()
            } else {
                format!("{} days ago", days)
            }
        }
        d if d < 2592000 => {
            let weeks = d / 604800;
            if weeks == 1 {
                "1 week ago".to_string()
            } else {
                format!("{} weeks ago", weeks)
            }
        }
        _ => {
            let months = diff / 2592000;
            if months == 1 {
                "1 month ago".to_string()
            } else {
                format!("{} months ago", months)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Message;

    #[test]
    fn test_format_relative_time() {
        let now = 1000000;
        assert_eq!(format_relative_time(now, now), "just now");
        assert_eq!(format_relative_time(now - 30, now), "just now");
        assert_eq!(format_relative_time(now - 60, now), "1 minute ago");
        assert_eq!(format_relative_time(now - 120, now), "2 minutes ago");
        assert_eq!(format_relative_time(now - 3600, now), "1 hour ago");
        assert_eq!(format_relative_time(now - 7200, now), "2 hours ago");
        assert_eq!(format_relative_time(now - 86400, now), "1 day ago");
        assert_eq!(format_relative_time(now - 172800, now), "2 days ago");
    }

    #[test]
    fn test_generate_memory_name() {
        let name = SessionStorage::generate_memory_name();
        assert!(name.contains('-'));
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert!(ADJECTIVES.contains(&parts[0]));
        assert!(NOUNS.contains(&parts[1]));
    }

    #[test]
    fn test_cyberpunk_word_lists() {
        assert_eq!(ADJECTIVES.len(), 32);
        assert_eq!(NOUNS.len(), 32);
        // Total combinations
        assert_eq!(ADJECTIVES.len() * NOUNS.len(), 1024);
    }

    #[tokio::test]
    async fn test_memory_storage_create_session() {
        let storage = SessionStorage::new(false).unwrap();
        assert!(!storage.is_persistent());

        let session = storage.create_session(None).await.unwrap();
        assert!(!session.id.is_empty());
        assert!(session.name.contains('-'));
        assert!(session.user_id.is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_create_session_with_user() {
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(Some("user@example.com")).await.unwrap();
        assert_eq!(session.user_id, Some("user@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_memory_storage_get_session() {
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(None).await.unwrap();

        let retrieved = storage.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_memory_storage_get_nonexistent_session() {
        let storage = SessionStorage::new(false).unwrap();
        let retrieved = storage.get_session("nonexistent-id").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_set_and_get_history() {
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(None).await.unwrap();

        // Set history
        let history = vec![
            Message::User { content: "Hello".to_string() },
            Message::Assistant { content: Some("Hi there!".to_string()), tool_calls: None },
        ];
        storage.set_history(&session.id, &history).await.unwrap();

        // Get history
        let retrieved = storage.get_history(&session.id).await.unwrap();
        assert_eq!(retrieved.len(), 2);

        match &retrieved[0] {
            Message::User { content } => assert_eq!(content, "Hello"),
            _ => panic!("Expected User message"),
        }
        match &retrieved[1] {
            Message::Assistant { content, .. } => assert_eq!(content, &Some("Hi there!".to_string())),
            _ => panic!("Expected Assistant message"),
        }
    }

    #[tokio::test]
    async fn test_memory_storage_multiple_sessions_independent_history() {
        let storage = SessionStorage::new(false).unwrap();

        // Create two sessions
        let session1 = storage.create_session(None).await.unwrap();
        let session2 = storage.create_session(None).await.unwrap();

        // Set different history for each
        let history1 = vec![Message::User { content: "Session 1 message".to_string() }];
        let history2 = vec![
            Message::User { content: "Session 2 message".to_string() },
            Message::Assistant { content: Some("Session 2 response".to_string()), tool_calls: None },
        ];

        storage.set_history(&session1.id, &history1).await.unwrap();
        storage.set_history(&session2.id, &history2).await.unwrap();

        // Verify each session has its own history
        let retrieved1 = storage.get_history(&session1.id).await.unwrap();
        let retrieved2 = storage.get_history(&session2.id).await.unwrap();

        assert_eq!(retrieved1.len(), 1);
        assert_eq!(retrieved2.len(), 2);

        match &retrieved1[0] {
            Message::User { content } => assert_eq!(content, "Session 1 message"),
            _ => panic!("Expected User message"),
        }
        match &retrieved2[0] {
            Message::User { content } => assert_eq!(content, "Session 2 message"),
            _ => panic!("Expected User message"),
        }
    }

    #[tokio::test]
    async fn test_memory_storage_list_sessions() {
        let storage = SessionStorage::new(false).unwrap();

        // Create sessions
        let _session1 = storage.create_session(None).await.unwrap();
        let _session2 = storage.create_session(None).await.unwrap();

        let sessions = storage.list_sessions(None).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_storage_list_sessions_by_user() {
        let storage = SessionStorage::new(false).unwrap();

        // Create sessions for different users
        let _session1 = storage.create_session(Some("user1@example.com")).await.unwrap();
        let _session2 = storage.create_session(Some("user1@example.com")).await.unwrap();
        let _session3 = storage.create_session(Some("user2@example.com")).await.unwrap();

        // List sessions for user1
        let user1_sessions = storage.list_sessions(Some("user1@example.com")).await.unwrap();
        assert_eq!(user1_sessions.len(), 2);

        // List sessions for user2
        let user2_sessions = storage.list_sessions(Some("user2@example.com")).await.unwrap();
        assert_eq!(user2_sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_storage_delete_session() {
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(None).await.unwrap();

        // Verify session exists
        assert!(storage.get_session(&session.id).await.unwrap().is_some());

        // Delete session
        let deleted = storage.delete_session(&session.id).await.unwrap();
        assert!(deleted);

        // Verify session is gone
        assert!(storage.get_session(&session.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_delete_nonexistent_session() {
        let storage = SessionStorage::new(false).unwrap();
        let deleted = storage.delete_session("nonexistent-id").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_memory_storage_get_or_create_user_session() {
        let storage = SessionStorage::new(false).unwrap();
        let user_id = "user@example.com";

        // First call creates a new session
        let session1 = storage.get_or_create_user_session(user_id).await.unwrap();
        assert_eq!(session1.user_id, Some(user_id.to_string()));

        // Second call returns the same session
        let session2 = storage.get_or_create_user_session(user_id).await.unwrap();
        assert_eq!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_memory_storage_switching_sessions_preserves_history() {
        let storage = SessionStorage::new(false).unwrap();

        // Create two sessions with history
        let session1 = storage.create_session(None).await.unwrap();
        let session2 = storage.create_session(None).await.unwrap();

        let history1 = vec![
            Message::User { content: "First session query".to_string() },
            Message::Assistant { content: Some("First session response".to_string()), tool_calls: None },
        ];
        let history2 = vec![
            Message::User { content: "Second session query".to_string() },
            Message::Assistant { content: Some("Second session response".to_string()), tool_calls: None },
        ];

        storage.set_history(&session1.id, &history1).await.unwrap();
        storage.set_history(&session2.id, &history2).await.unwrap();

        // Simulate switching: get history for session1
        let retrieved1 = storage.get_history(&session1.id).await.unwrap();
        assert_eq!(retrieved1.len(), 2);
        match &retrieved1[0] {
            Message::User { content } => assert!(content.contains("First session")),
            _ => panic!("Expected User message"),
        }

        // Switch to session2
        let retrieved2 = storage.get_history(&session2.id).await.unwrap();
        assert_eq!(retrieved2.len(), 2);
        match &retrieved2[0] {
            Message::User { content } => assert!(content.contains("Second session")),
            _ => panic!("Expected User message"),
        }

        // Switch back to session1 - history should be preserved
        let retrieved1_again = storage.get_history(&session1.id).await.unwrap();
        assert_eq!(retrieved1_again.len(), 2);
        match &retrieved1_again[0] {
            Message::User { content } => assert!(content.contains("First session")),
            _ => panic!("Expected User message"),
        }
    }
}
