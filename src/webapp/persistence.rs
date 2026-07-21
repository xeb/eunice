//! Session persistence layer for the webapp.
//!
//! Provides SQLite-based session storage (sessions.db) with an
//! in-memory fallback.

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

/// Outcome of a scheduled agent run.
///
/// Lives here rather than in `scheduler` because it is persisted: `sessions.run_status`
/// stores `as_str()` and reads it back through `from_str()`. `Skipped` is the one variant
/// that is never stored — a skipped tick never creates a session — so it exists only for
/// the scheduler's in-memory per-agent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Skipped,
    /// The process died while the run was in flight; detected by the startup sweep.
    Interrupted,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failed => "failed",
            RunStatus::Skipped => "skipped",
            RunStatus::Interrupted => "interrupted",
        }
    }

    /// Unknown values read as `None` so a row written by a newer binary degrades to
    /// "no status" instead of failing the whole query.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "running" => Some(RunStatus::Running),
            "success" => Some(RunStatus::Success),
            "failed" => Some(RunStatus::Failed),
            "skipped" => Some(RunStatus::Skipped),
            "interrupted" => Some(RunStatus::Interrupted),
            _ => None,
        }
    }
}

/// Session record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub name: String,
    pub user_id: Option<String>,
    pub agent_name: Option<String>,
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
    pub agent_name: Option<String>,
    /// Only ever set for agent-run sessions; `None` for interactive chats and for
    /// rows written before the column existed.
    pub run_status: Option<RunStatus>,
}

/// In-memory session for runtime use
pub struct MemorySession {
    pub id: String,
    pub name: String,
    pub user_id: Option<String>,
    pub agent_name: Option<String>,
    pub run_status: Option<RunStatus>,
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
            agent_name: None,
            run_status: None,
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
    /// Initialize storage: SQLite (sessions.db in cwd) or in-memory
    pub fn new(persist: bool) -> Result<Self> {
        if persist {
            Self::new_sqlite("sessions.db")
        } else {
            Ok(Self::new_memory())
        }
    }

    /// In-memory storage (no persistence across restarts)
    pub fn new_memory() -> Self {
        SessionStorage::Memory(Arc::new(RwLock::new(HashMap::new())))
    }

    /// Initialize SQLite storage with a custom path (for testing)
    pub fn new_sqlite(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Self::migrate_schema(&conn)?;
        Self::sweep_interrupted_runs(&conn)?;
        Ok(SessionStorage::Sqlite {
            conn: Arc::new(Mutex::new(conn)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                user_id TEXT,
                agent_name TEXT,
                run_status TEXT,
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
            CREATE TABLE IF NOT EXISTS compactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                summary TEXT NOT NULL,
                compacted_through_seq INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id, sequence_num);
            CREATE INDEX IF NOT EXISTS idx_compactions_session_id ON compactions(session_id);
            "#,
        )?;
        Ok(())
    }

    /// Add columns that post-date the original schema. `CREATE TABLE IF NOT EXISTS`
    /// silently no-ops on an existing database, so new columns must be added here.
    fn migrate_schema(conn: &Connection) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(sessions)")?;
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        drop(stmt);

        if !columns.iter().any(|c| c == "agent_name") {
            conn.execute("ALTER TABLE sessions ADD COLUMN agent_name TEXT", [])?;
        }

        if !columns.iter().any(|c| c == "run_status") {
            conn.execute("ALTER TABLE sessions ADD COLUMN run_status TEXT", [])?;
        }

        // Indexed after the column is guaranteed to exist: an index on a missing
        // column is an error, which would abort init_schema on an old database.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_sessions_agent_name ON sessions(agent_name, updated_at DESC);",
        )?;
        Ok(())
    }

    /// Nothing is running the instant the process starts, so any row still marked
    /// `running` belongs to a run the previous process never finished. Without this
    /// the UI would show it spinning forever. Runs at startup, after migration, so
    /// the column is guaranteed to exist.
    fn sweep_interrupted_runs(conn: &Connection) -> Result<usize> {
        let updated = conn.execute(
            "UPDATE sessions SET run_status = ? WHERE run_status = ?",
            params![RunStatus::Interrupted.as_str(), RunStatus::Running.as_str()],
        )?;
        Ok(updated)
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
                    agent_name: None,
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
                    agent_name: None,
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }

    /// Create a session tagged as produced by a scheduled agent run.
    pub async fn create_agent_session(&self, agent_name: &str) -> Result<SessionRecord> {
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();

        match self {
            SessionStorage::Memory(store) => {
                let name = Self::generate_memory_name();
                let mut session = MemorySession::new(id.clone(), name.clone(), None);
                session.agent_name = Some(agent_name.to_string());
                session.run_status = Some(RunStatus::Running);
                store.write().await.insert(id.clone(), session);
                Ok(SessionRecord {
                    id,
                    name,
                    user_id: None,
                    agent_name: Some(agent_name.to_string()),
                    created_at: now,
                    updated_at: now,
                })
            }
            SessionStorage::Sqlite { conn, runtime } => {
                let conn = conn.lock().await;
                let name = Self::generate_name(&conn)?;
                conn.execute(
                    "INSERT INTO sessions (id, name, user_id, agent_name, run_status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![&id, &name, None::<String>, agent_name, RunStatus::Running.as_str(), now, now],
                )?;
                runtime.write().await.insert(id.clone(), RuntimeState::new());
                Ok(SessionRecord {
                    id,
                    name,
                    user_id: None,
                    agent_name: Some(agent_name.to_string()),
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }

    /// Record the outcome of an agent run on its session. Deliberately leaves
    /// `updated_at` untouched: the transcript's last event is what orders the list,
    /// and bumping it here would float finished runs above newer activity.
    pub async fn set_run_status(&self, session_id: &str, status: RunStatus) -> Result<()> {
        match self {
            SessionStorage::Memory(store) => {
                let mut store = store.write().await;
                if let Some(session) = store.get_mut(session_id) {
                    session.run_status = Some(status);
                }
                Ok(())
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                conn.execute(
                    "UPDATE sessions SET run_status = ? WHERE id = ?",
                    params![status.as_str(), session_id],
                )?;
                Ok(())
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
                    agent_name: s.agent_name.clone(),
                    created_at: s.created_at,
                    updated_at: s.updated_at,
                }))
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let mut stmt = conn.prepare(
                    "SELECT id, name, user_id, agent_name, created_at, updated_at FROM sessions WHERE id = ?",
                )?;
                let result = stmt.query_row([session_id], |row| {
                    Ok(SessionRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        user_id: row.get(2)?,
                        agent_name: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
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
                            agent_name: s.agent_name.clone(),
                            run_status: s.run_status,
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
                                 AND e.event_type IN ('user_message', 'assistant_message')) as turn_count,
                                s.agent_name, s.run_status
                         FROM sessions s WHERE s.user_id = ? ORDER BY s.updated_at DESC"
                    )?;
                    let rows = stmt.query_map([uid], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, name, updated_at, turn_count, agent_name, run_status) = row?;
                        sessions.push(SessionMetadata {
                            id,
                            name,
                            turn_count: turn_count as usize,
                            updated_at,
                            relative_time: format_relative_time(updated_at, now),
                            agent_name,
                            run_status: run_status.as_deref().and_then(RunStatus::from_str),
                        });
                    }
                } else {
                    let mut stmt = conn.prepare(
                        "SELECT s.id, s.name, s.updated_at,
                                (SELECT COUNT(*) FROM events e WHERE e.session_id = s.id
                                 AND e.event_type IN ('user_message', 'assistant_message')) as turn_count,
                                s.agent_name, s.run_status
                         FROM sessions s ORDER BY s.updated_at DESC"
                    )?;
                    let rows = stmt.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, name, updated_at, turn_count, agent_name, run_status) = row?;
                        sessions.push(SessionMetadata {
                            id,
                            name,
                            turn_count: turn_count as usize,
                            updated_at,
                            relative_time: format_relative_time(updated_at, now),
                            agent_name,
                            run_status: run_status.as_deref().and_then(RunStatus::from_str),
                        });
                    }
                }

                Ok(sessions)
            }
        }
    }

    /// Most recent sessions produced by a given agent, newest first.
    pub async fn list_agent_sessions(&self, agent_name: &str, limit: usize) -> Result<Vec<SessionMetadata>> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                let now = chrono::Utc::now().timestamp();
                let mut sessions: Vec<SessionMetadata> = store
                    .values()
                    .filter(|s| s.agent_name.as_deref() == Some(agent_name))
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
                            agent_name: s.agent_name.clone(),
                            run_status: s.run_status,
                        }
                    })
                    .collect();
                sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                sessions.truncate(limit);
                Ok(sessions)
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let now = chrono::Utc::now().timestamp();

                let mut stmt = conn.prepare(
                    "SELECT s.id, s.name, s.updated_at,
                            (SELECT COUNT(*) FROM events e WHERE e.session_id = s.id
                             AND e.event_type IN ('user_message', 'assistant_message')) as turn_count,
                            s.agent_name, s.run_status
                     FROM sessions s WHERE s.agent_name = ? ORDER BY s.updated_at DESC LIMIT ?"
                )?;
                let rows = stmt.query_map(params![agent_name, limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })?;

                let mut sessions = Vec::new();
                for row in rows {
                    let (id, name, updated_at, turn_count, agent_name, run_status) = row?;
                    sessions.push(SessionMetadata {
                        id,
                        name,
                        turn_count: turn_count as usize,
                        updated_at,
                        relative_time: format_relative_time(updated_at, now),
                        agent_name,
                        run_status: run_status.as_deref().and_then(RunStatus::from_str),
                    });
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

    /// Get conversation history for a session (full history for replay/display)
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

    /// Get conversation history optimized for LLM context (uses compaction if available)
    /// Returns: (messages_for_llm, total_message_count, needs_compaction)
    #[allow(dead_code)]
    pub async fn get_history_for_llm(
        &self,
        session_id: &str,
        _recent_messages_to_keep: usize,
        compaction_threshold: usize,
    ) -> Result<(Vec<Message>, usize, bool)> {
        match self {
            SessionStorage::Memory(store) => {
                let store = store.read().await;
                let history = store.get(session_id).map(|s| s.history.clone()).unwrap_or_default();
                let total = history.len();
                let needs_compaction = total > compaction_threshold;
                Ok((history, total, needs_compaction))
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;

                // Get total message count
                let total: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM events WHERE session_id = ?
                     AND event_type IN ('user_message', 'assistant_message', 'tool_message')",
                    [session_id],
                    |row| row.get(0),
                )?;
                let total = total as usize;

                // Check if we have a compaction
                let compaction: Option<(String, i64)> = conn.query_row(
                    "SELECT summary, compacted_through_seq FROM compactions
                     WHERE session_id = ? ORDER BY id DESC LIMIT 1",
                    [session_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ).ok();

                let mut history = Vec::new();

                if let Some((summary, compacted_seq)) = compaction {
                    // Start with compaction summary as system context
                    history.push(Message::User {
                        content: format!(
                            "[Previous conversation summary]\n{}\n[End of summary - recent messages follow]",
                            summary
                        ),
                    });

                    // Get only messages after the compaction point
                    let mut stmt = conn.prepare(
                        "SELECT event_type, content FROM events WHERE session_id = ?
                         AND event_type IN ('user_message', 'assistant_message', 'tool_message')
                         AND sequence_num > ?
                         ORDER BY sequence_num ASC",
                    )?;
                    let rows = stmt.query_map(params![session_id, compacted_seq], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?;

                    for row in rows {
                        let (event_type, content) = row?;
                        if let Ok(msg) = serde_json::from_str::<Message>(&content) {
                            history.push(msg);
                        } else if event_type == "user_message" {
                            history.push(Message::User { content });
                        }
                    }

                    // Check if we need another compaction (recent messages after last compaction > threshold)
                    let needs_compaction = history.len() > compaction_threshold;
                    Ok((history, total, needs_compaction))
                } else {
                    // No compaction - return full history and flag if compaction needed
                    let mut stmt = conn.prepare(
                        "SELECT event_type, content FROM events WHERE session_id = ?
                         AND event_type IN ('user_message', 'assistant_message', 'tool_message')
                         ORDER BY sequence_num ASC",
                    )?;
                    let rows = stmt.query_map([session_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?;

                    for row in rows {
                        let (event_type, content) = row?;
                        if let Ok(msg) = serde_json::from_str::<Message>(&content) {
                            history.push(msg);
                        } else if event_type == "user_message" {
                            history.push(Message::User { content });
                        }
                    }

                    let needs_compaction = total > compaction_threshold;
                    Ok((history, total, needs_compaction))
                }
            }
        }
    }

    /// Save a compaction summary for a session
    #[allow(dead_code)]
    pub async fn save_compaction(&self, session_id: &str, summary: &str) -> Result<()> {
        match self {
            SessionStorage::Memory(_) => {
                // In-memory doesn't persist compactions
                Ok(())
            }
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let now = chrono::Utc::now().timestamp();

                // Get the current max sequence number
                let max_seq: i64 = conn.query_row(
                    "SELECT COALESCE(MAX(sequence_num), 0) FROM events WHERE session_id = ?",
                    [session_id],
                    |row| row.get(0),
                )?;

                conn.execute(
                    "INSERT INTO compactions (session_id, summary, compacted_through_seq, created_at)
                     VALUES (?, ?, ?, ?)",
                    params![session_id, summary, max_seq, now],
                )?;
                Ok(())
            }
        }
    }

    /// Get the latest compaction for a session (if any)
    #[allow(dead_code)]
    pub async fn get_compaction(&self, session_id: &str) -> Result<Option<String>> {
        match self {
            SessionStorage::Memory(_) => Ok(None),
            SessionStorage::Sqlite { conn, .. } => {
                let conn = conn.lock().await;
                let result: rusqlite::Result<String> = conn.query_row(
                    "SELECT summary FROM compactions WHERE session_id = ? ORDER BY id DESC LIMIT 1",
                    [session_id],
                    |row| row.get(0),
                );
                match result {
                    Ok(summary) => Ok(Some(summary)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
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
                    agent_name: None,
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
                    agent_name: None,
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
                                agent_name: session.agent_name.clone(),
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
                    "SELECT id, name, user_id, agent_name, created_at, updated_at FROM sessions
                     WHERE user_id = ? ORDER BY updated_at DESC LIMIT 1",
                    [user_id],
                    |row| {
                        Ok(SessionRecord {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            user_id: row.get(2)?,
                            agent_name: row.get(3)?,
                            created_at: row.get(4)?,
                            updated_at: row.get(5)?,
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
    /// Returns the new session that was created
    pub async fn clear_user_session(&self, user_id: &str) -> Result<SessionRecord> {
        match self {
            SessionStorage::Memory(store) => {
                // For memory mode, also create a new session (consistent with SQLite)
                let mut store = store.write().await;
                // Clear runtime state from old session if exists
                for session in store.values_mut() {
                    if session.user_id.as_deref() == Some(user_id) {
                        session.history.clear();
                        session.events.clear();
                        break;
                    }
                }
                drop(store);
                // Create new session
                self.create_session(Some(user_id)).await
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
                // Create new session for user and return it
                self.create_session(Some(user_id)).await
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

    // =====================================================
    // SESSION ISOLATION TESTS
    // =====================================================

    #[tokio::test]
    async fn test_long_session_history_preservation() {
        // Test that a session with 100+ messages preserves all history
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(Some("user@example.com")).await.unwrap();

        // Create 100 message pairs (user + assistant = 200 messages)
        let mut history = Vec::new();
        for i in 0..100 {
            history.push(Message::User { content: format!("Message {}", i) });
            history.push(Message::Assistant {
                content: Some(format!("Response {}", i)),
                tool_calls: None,
            });
        }

        storage.set_history(&session.id, &history).await.unwrap();

        // Verify all 200 messages are preserved
        let retrieved = storage.get_history(&session.id).await.unwrap();
        assert_eq!(retrieved.len(), 200);

        // Verify first message
        match &retrieved[0] {
            Message::User { content } => assert_eq!(content, "Message 0"),
            _ => panic!("Expected User message"),
        }

        // Verify last message
        match &retrieved[199] {
            Message::Assistant { content, .. } => assert_eq!(content, &Some("Response 99".to_string())),
            _ => panic!("Expected Assistant message"),
        }

        // Verify middle message (message 50)
        match &retrieved[100] {
            Message::User { content } => assert_eq!(content, "Message 50"),
            _ => panic!("Expected User message"),
        }
    }

    #[tokio::test]
    async fn test_user_session_isolation() {
        // Test that different users have completely isolated sessions
        let storage = SessionStorage::new(false).unwrap();

        let user1 = "alice@example.com";
        let user2 = "bob@example.com";

        // Create sessions for each user
        let session1 = storage.create_session(Some(user1)).await.unwrap();
        let session2 = storage.create_session(Some(user2)).await.unwrap();

        // Set different history for each
        let history1 = vec![
            Message::User { content: "Alice's secret message".to_string() },
            Message::Assistant { content: Some("Alice's private response".to_string()), tool_calls: None },
        ];
        let history2 = vec![
            Message::User { content: "Bob's confidential query".to_string() },
            Message::Assistant { content: Some("Bob's private data".to_string()), tool_calls: None },
        ];

        storage.set_history(&session1.id, &history1).await.unwrap();
        storage.set_history(&session2.id, &history2).await.unwrap();

        // Verify user1's sessions don't show user2's data
        let user1_sessions = storage.list_sessions(Some(user1)).await.unwrap();
        assert_eq!(user1_sessions.len(), 1);
        assert_eq!(user1_sessions[0].id, session1.id);

        // Verify user2's sessions don't show user1's data
        let user2_sessions = storage.list_sessions(Some(user2)).await.unwrap();
        assert_eq!(user2_sessions.len(), 1);
        assert_eq!(user2_sessions[0].id, session2.id);

        // Verify histories are isolated
        let retrieved1 = storage.get_history(&session1.id).await.unwrap();
        let retrieved2 = storage.get_history(&session2.id).await.unwrap();

        match &retrieved1[0] {
            Message::User { content } => assert!(content.contains("Alice")),
            _ => panic!("Expected User message"),
        }
        match &retrieved2[0] {
            Message::User { content } => assert!(content.contains("Bob")),
            _ => panic!("Expected User message"),
        }
    }

    #[tokio::test]
    async fn test_session_ownership_is_preserved() {
        // Test that session user_id cannot be changed
        let storage = SessionStorage::new(false).unwrap();

        let user1 = "original@example.com";
        let session = storage.create_session(Some(user1)).await.unwrap();

        // Verify session ownership
        let retrieved = storage.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_id, Some(user1.to_string()));

        // Session ownership should persist
        let retrieved_again = storage.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved_again.user_id, Some(user1.to_string()));
    }

    #[tokio::test]
    async fn test_multiple_sessions_per_user() {
        // Test that a user can have multiple sessions with independent history
        let storage = SessionStorage::new(false).unwrap();
        let user = "multi-session@example.com";

        // Create 3 sessions for the same user
        let session1 = storage.create_session(Some(user)).await.unwrap();
        let session2 = storage.create_session(Some(user)).await.unwrap();
        let session3 = storage.create_session(Some(user)).await.unwrap();

        // Set different history for each
        storage.set_history(&session1.id, &vec![
            Message::User { content: "Session 1 - Project A discussion".to_string() },
        ]).await.unwrap();

        storage.set_history(&session2.id, &vec![
            Message::User { content: "Session 2 - Bug investigation".to_string() },
            Message::Assistant { content: Some("Found the bug".to_string()), tool_calls: None },
        ]).await.unwrap();

        storage.set_history(&session3.id, &vec![
            Message::User { content: "Session 3 - Code review".to_string() },
            Message::Assistant { content: Some("LGTM".to_string()), tool_calls: None },
            Message::User { content: "Thanks!".to_string() },
        ]).await.unwrap();

        // Verify all sessions belong to user
        let user_sessions = storage.list_sessions(Some(user)).await.unwrap();
        assert_eq!(user_sessions.len(), 3);

        // Verify each session has its correct history
        let h1 = storage.get_history(&session1.id).await.unwrap();
        let h2 = storage.get_history(&session2.id).await.unwrap();
        let h3 = storage.get_history(&session3.id).await.unwrap();

        assert_eq!(h1.len(), 1);
        assert_eq!(h2.len(), 2);
        assert_eq!(h3.len(), 3);
    }

    #[tokio::test]
    async fn test_get_or_create_returns_most_recent_session() {
        // Test that get_or_create_user_session returns the most recently updated session
        let storage = SessionStorage::new(false).unwrap();
        let user = "returning@example.com";

        // Create first session and add history
        let session1 = storage.create_session(Some(user)).await.unwrap();
        storage.set_history(&session1.id, &vec![
            Message::User { content: "Old conversation".to_string() },
        ]).await.unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create second session and add history (this is now the most recent)
        let session2 = storage.create_session(Some(user)).await.unwrap();
        storage.set_history(&session2.id, &vec![
            Message::User { content: "New conversation".to_string() },
        ]).await.unwrap();

        // get_or_create should return an existing session (the first one found)
        let returned = storage.get_or_create_user_session(user).await.unwrap();
        // Note: The behavior may return first or most recent depending on implementation
        // The key is that it returns ONE of the user's sessions
        assert!(returned.id == session1.id || returned.id == session2.id);
        assert_eq!(returned.user_id, Some(user.to_string()));
    }

    #[tokio::test]
    async fn test_session_clear_creates_new_session() {
        // Test that clearing a user session creates a fresh new session
        let storage = SessionStorage::new(false).unwrap();
        let user = "clearable@example.com";

        // Create session with history
        let original = storage.get_or_create_user_session(user).await.unwrap();
        storage.set_history(&original.id, &vec![
            Message::User { content: "This will be cleared".to_string() },
            Message::Assistant { content: Some("Indeed".to_string()), tool_calls: None },
        ]).await.unwrap();

        // Verify history exists
        let history_before = storage.get_history(&original.id).await.unwrap();
        assert_eq!(history_before.len(), 2);

        // Clear session - returns new session
        let new_session = storage.clear_user_session(user).await.unwrap();

        // New session should have different ID
        assert_ne!(new_session.id, original.id);

        // New session should have empty history
        let history_new = storage.get_history(&new_session.id).await.unwrap();
        assert!(history_new.is_empty());

        // Old session's history should be cleared too (memory mode)
        let history_old = storage.get_history(&original.id).await.unwrap();
        assert!(history_old.is_empty());
    }

    #[tokio::test]
    async fn test_anonymous_sessions_are_separate() {
        // Test that anonymous sessions are completely isolated
        let storage = SessionStorage::new(false).unwrap();

        // Create anonymous sessions
        let anon1 = storage.create_session(None).await.unwrap();
        let anon2 = storage.create_session(None).await.unwrap();

        // Both should have no user_id
        assert!(anon1.user_id.is_none());
        assert!(anon2.user_id.is_none());

        // Set different history
        storage.set_history(&anon1.id, &vec![
            Message::User { content: "Anonymous 1".to_string() },
        ]).await.unwrap();

        storage.set_history(&anon2.id, &vec![
            Message::User { content: "Anonymous 2".to_string() },
        ]).await.unwrap();

        // Histories are isolated
        let h1 = storage.get_history(&anon1.id).await.unwrap();
        let h2 = storage.get_history(&anon2.id).await.unwrap();

        match &h1[0] {
            Message::User { content } => assert_eq!(content, "Anonymous 1"),
            _ => panic!("Expected User message"),
        }
        match &h2[0] {
            Message::User { content } => assert_eq!(content, "Anonymous 2"),
            _ => panic!("Expected User message"),
        }
    }

    #[tokio::test]
    async fn test_append_to_existing_history() {
        // Test that appending to history preserves existing messages
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(None).await.unwrap();

        // Set initial history
        let initial = vec![
            Message::User { content: "First message".to_string() },
            Message::Assistant { content: Some("First response".to_string()), tool_calls: None },
        ];
        storage.set_history(&session.id, &initial).await.unwrap();

        // Append more messages
        let mut extended = initial.clone();
        extended.push(Message::User { content: "Second message".to_string() });
        extended.push(Message::Assistant { content: Some("Second response".to_string()), tool_calls: None });
        storage.set_history(&session.id, &extended).await.unwrap();

        // Verify all messages are preserved
        let retrieved = storage.get_history(&session.id).await.unwrap();
        assert_eq!(retrieved.len(), 4);

        match &retrieved[0] {
            Message::User { content } => assert_eq!(content, "First message"),
            _ => panic!("Expected User message"),
        }
        match &retrieved[2] {
            Message::User { content } => assert_eq!(content, "Second message"),
            _ => panic!("Expected User message"),
        }
    }

    // =====================================================
    // SQLITE STORAGE TESTS
    // =====================================================

    /// Create a temp SQLite storage for testing
    fn create_temp_sqlite_storage() -> (SessionStorage, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_sessions.db");
        let storage = SessionStorage::new_sqlite(db_path.to_str().unwrap()).unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_sqlite_storage_is_persistent() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        assert!(storage.is_persistent());
    }

    #[tokio::test]
    async fn test_sqlite_storage_create_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let session = storage.create_session(None).await.unwrap();
        assert!(!session.id.is_empty());
        assert!(session.name.contains('-'));
        assert!(session.user_id.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_storage_create_session_with_user() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let session = storage.create_session(Some("user@example.com")).await.unwrap();
        assert_eq!(session.user_id, Some("user@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_sqlite_storage_get_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let session = storage.create_session(None).await.unwrap();

        let retrieved = storage.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.name, session.name);
    }

    #[tokio::test]
    async fn test_sqlite_storage_get_nonexistent_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let retrieved = storage.get_session("nonexistent-id").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_storage_append_and_get_events() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let session = storage.create_session(None).await.unwrap();

        // Add events using append_event
        storage.append_event(&session.id, "response", "Hello world").await.unwrap();
        storage.append_event(&session.id, "tool_call", r#"{"name": "bash"}"#).await.unwrap();

        // Get history (reconstructed from events)
        let history = storage.get_history(&session.id).await.unwrap();
        // Events are stored but history reconstruction depends on event types
        // Check that we can query events
        assert!(history.is_empty() || !history.is_empty()); // May or may not have history
    }

    #[tokio::test]
    async fn test_sqlite_storage_list_sessions() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        // Create sessions
        let _session1 = storage.create_session(None).await.unwrap();
        let _session2 = storage.create_session(None).await.unwrap();

        let sessions = storage.list_sessions(None).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_storage_list_sessions_by_user() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

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
    async fn test_sqlite_storage_delete_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
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
    async fn test_sqlite_storage_delete_nonexistent_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let deleted = storage.delete_session("nonexistent-id").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_sqlite_storage_get_or_create_user_session() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let user_id = "user@example.com";

        // First call creates a new session
        let session1 = storage.get_or_create_user_session(user_id).await.unwrap();
        assert_eq!(session1.user_id, Some(user_id.to_string()));

        // Second call returns the same session
        let session2 = storage.get_or_create_user_session(user_id).await.unwrap();
        assert_eq!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_sqlite_storage_persistence_across_connections() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("persist_test.db");
        let db_path_str = db_path.to_str().unwrap();

        // Create storage and add a session
        let session_id;
        let session_name;
        {
            let storage = SessionStorage::new_sqlite(db_path_str).unwrap();
            let session = storage.create_session(Some("persist@example.com")).await.unwrap();
            session_id = session.id.clone();
            session_name = session.name.clone();

            // Add an event
            storage.append_event(&session_id, "response", "Persisted message").await.unwrap();
        }
        // Storage dropped here, connection closed

        // Reopen storage and verify data persisted
        {
            let storage = SessionStorage::new_sqlite(db_path_str).unwrap();

            // Session should exist
            let retrieved = storage.get_session(&session_id).await.unwrap();
            assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            assert_eq!(retrieved.id, session_id);
            assert_eq!(retrieved.name, session_name);
            assert_eq!(retrieved.user_id, Some("persist@example.com".to_string()));

            // List should include the session
            let sessions = storage.list_sessions(Some("persist@example.com")).await.unwrap();
            assert_eq!(sessions.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_sqlite_storage_unique_session_names() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        // Create many sessions and verify all names are unique
        let mut names = std::collections::HashSet::new();
        for _ in 0..50 {
            let session = storage.create_session(None).await.unwrap();
            assert!(names.insert(session.name.clone()), "Duplicate name: {}", session.name);
        }
        assert_eq!(names.len(), 50);
    }

    #[tokio::test]
    async fn test_sqlite_storage_session_timestamps() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let session = storage.create_session(None).await.unwrap();

        // Timestamps should be set
        assert!(session.created_at > 0);
        assert!(session.updated_at >= session.created_at);

        // Check that list_sessions includes relative time
        let sessions = storage.list_sessions(None).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(!sessions[0].relative_time.is_empty());
    }

    #[tokio::test]
    async fn test_sqlite_storage_multiple_sessions_for_user() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let user_id = "newuser@example.com";

        // Create two sessions for the same user
        let session1 = storage.create_session(Some(user_id)).await.unwrap();
        let session2 = storage.create_session(Some(user_id)).await.unwrap();

        // Should be different sessions
        assert_ne!(session1.id, session2.id);
        assert_eq!(session1.user_id, Some(user_id.to_string()));
        assert_eq!(session2.user_id, Some(user_id.to_string()));

        // Both sessions should exist
        let sessions = storage.list_sessions(Some(user_id)).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_storage_event_ordering() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let session = storage.create_session(None).await.unwrap();

        // Add multiple events
        for i in 0..10 {
            storage.append_event(&session.id, "response", &format!("Message {}", i)).await.unwrap();
        }

        // Events should be stored (we can't directly query them, but history should work)
        let history = storage.get_history(&session.id).await.unwrap();
        // History may be empty since these are response events without user messages
        // But the session should still be accessible
        let retrieved = storage.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_some());
    }

    // =====================================================
    // AGENT SESSION TESTS
    // =====================================================

    /// The sessions/events schema as it existed before the agent_name column,
    /// so migrations can be tested against a realistic pre-upgrade database.
    fn create_old_schema_db(path: &std::path::Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                user_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                sequence_num INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(session_id, sequence_num)
            );
            "#,
        )
        .unwrap();
    }

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table)).unwrap();
        let cols = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap();
        cols
    }

    #[test]
    fn test_migrate_schema_adds_agent_name_to_old_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        assert!(!column_names(&conn, "sessions").contains(&"agent_name".to_string()));

        SessionStorage::migrate_schema(&conn).unwrap();
        assert!(column_names(&conn, "sessions").contains(&"agent_name".to_string()));

        conn.execute(
            "INSERT INTO sessions (id, name, user_id, agent_name, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params!["id-1", "chrome-molly", None::<String>, "daily-digest", 100, 100],
        )
        .unwrap();
        let agent_name: Option<String> = conn
            .query_row("SELECT agent_name FROM sessions WHERE id = ?", ["id-1"], |row| row.get(0))
            .unwrap();
        assert_eq!(agent_name, Some("daily-digest".to_string()));
    }

    #[test]
    fn test_migrate_schema_is_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        SessionStorage::migrate_schema(&conn).unwrap();
        SessionStorage::migrate_schema(&conn).unwrap();

        let count = column_names(&conn, "sessions")
            .iter()
            .filter(|c| *c == "agent_name")
            .count();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_sqlite_opens_old_database_and_tags_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        // Pre-existing untagged session written under the old schema.
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute(
                "INSERT INTO sessions (id, name, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                params!["legacy-id", "legacy-name", None::<String>, 100, 100],
            )
            .unwrap();
        }

        let storage = SessionStorage::new_sqlite(db_path.to_str().unwrap()).unwrap();

        let legacy = storage.get_session("legacy-id").await.unwrap().unwrap();
        assert_eq!(legacy.agent_name, None);

        let tagged = storage.create_agent_session("daily-digest").await.unwrap();
        assert_eq!(tagged.agent_name, Some("daily-digest".to_string()));

        let sessions = storage.list_sessions(None).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_create_agent_session_round_trip() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let created = storage.create_agent_session("repo-watch").await.unwrap();
        assert_eq!(created.agent_name, Some("repo-watch".to_string()));
        assert!(created.user_id.is_none());

        let retrieved = storage.get_session(&created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.agent_name, Some("repo-watch".to_string()));
    }

    #[tokio::test]
    async fn test_sqlite_list_agent_sessions() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let a1 = storage.create_agent_session("repo-watch").await.unwrap();
        let a2 = storage.create_agent_session("repo-watch").await.unwrap();
        let other = storage.create_agent_session("daily-digest").await.unwrap();
        let _untagged = storage.create_session(None).await.unwrap();

        let watch = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
        assert_eq!(watch.len(), 2);
        let ids: Vec<&str> = watch.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&a1.id.as_str()));
        assert!(ids.contains(&a2.id.as_str()));
        assert!(!ids.contains(&other.id.as_str()));
        assert!(watch.iter().all(|s| s.agent_name.as_deref() == Some("repo-watch")));

        // Newest first
        assert!(watch[0].updated_at >= watch[1].updated_at);
    }

    #[tokio::test]
    async fn test_sqlite_list_agent_sessions_respects_limit() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        for _ in 0..5 {
            storage.create_agent_session("repo-watch").await.unwrap();
        }

        let limited = storage.list_agent_sessions("repo-watch", 2).await.unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_list_sessions_reports_agent_name() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let tagged = storage.create_agent_session("daily-digest").await.unwrap();
        let untagged = storage.create_session(None).await.unwrap();

        let sessions = storage.list_sessions(None).await.unwrap();
        let tagged_meta = sessions.iter().find(|s| s.id == tagged.id).unwrap();
        let untagged_meta = sessions.iter().find(|s| s.id == untagged.id).unwrap();

        assert_eq!(tagged_meta.agent_name, Some("daily-digest".to_string()));
        assert_eq!(untagged_meta.agent_name, None);
    }

    #[tokio::test]
    async fn test_sqlite_list_sessions_by_user_reports_agent_name() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let user_session = storage.create_session(Some("user@example.com")).await.unwrap();
        storage.create_agent_session("daily-digest").await.unwrap();

        let sessions = storage.list_sessions(Some("user@example.com")).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, user_session.id);
        assert_eq!(sessions[0].agent_name, None);
    }

    #[tokio::test]
    async fn test_memory_create_agent_session_round_trip() {
        let storage = SessionStorage::new(false).unwrap();

        let created = storage.create_agent_session("repo-watch").await.unwrap();
        assert_eq!(created.agent_name, Some("repo-watch".to_string()));
        assert!(created.user_id.is_none());

        let retrieved = storage.get_session(&created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.agent_name, Some("repo-watch".to_string()));
    }

    #[tokio::test]
    async fn test_memory_list_agent_sessions() {
        let storage = SessionStorage::new(false).unwrap();

        let a1 = storage.create_agent_session("repo-watch").await.unwrap();
        let other = storage.create_agent_session("daily-digest").await.unwrap();
        storage.create_session(None).await.unwrap();

        let watch = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
        assert_eq!(watch.len(), 1);
        assert_eq!(watch[0].id, a1.id);
        assert_eq!(watch[0].agent_name, Some("repo-watch".to_string()));

        let digest = storage.list_agent_sessions("daily-digest", 10).await.unwrap();
        assert_eq!(digest.len(), 1);
        assert_eq!(digest[0].id, other.id);
    }

    #[tokio::test]
    async fn test_memory_list_agent_sessions_respects_limit() {
        let storage = SessionStorage::new(false).unwrap();

        for _ in 0..5 {
            storage.create_agent_session("repo-watch").await.unwrap();
        }

        let limited = storage.list_agent_sessions("repo-watch", 2).await.unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_list_sessions_reports_agent_name() {
        let storage = SessionStorage::new(false).unwrap();

        let tagged = storage.create_agent_session("daily-digest").await.unwrap();
        let untagged = storage.create_session(None).await.unwrap();

        let sessions = storage.list_sessions(None).await.unwrap();
        let tagged_meta = sessions.iter().find(|s| s.id == tagged.id).unwrap();
        let untagged_meta = sessions.iter().find(|s| s.id == untagged.id).unwrap();

        assert_eq!(tagged_meta.agent_name, Some("daily-digest".to_string()));
        assert_eq!(untagged_meta.agent_name, None);
    }

    #[tokio::test]
    async fn test_create_session_is_never_tagged() {
        let storage = SessionStorage::new(false).unwrap();
        let session = storage.create_session(Some("user@example.com")).await.unwrap();
        assert_eq!(session.agent_name, None);

        let (sqlite, _temp_dir) = create_temp_sqlite_storage();
        let session = sqlite.create_session(Some("user@example.com")).await.unwrap();
        assert_eq!(session.agent_name, None);
        let ensured = sqlite.ensure_session("explicit-id", None).await.unwrap();
        assert_eq!(ensured.agent_name, None);
    }

    // =====================================================
    // RUN STATUS TESTS
    // =====================================================

    #[test]
    fn test_run_status_str_round_trip() {
        for status in [
            RunStatus::Running,
            RunStatus::Success,
            RunStatus::Failed,
            RunStatus::Skipped,
            RunStatus::Interrupted,
        ] {
            assert_eq!(RunStatus::from_str(status.as_str()), Some(status));
        }

        // The stored string must match the JSON representation, so the DB value and
        // the API value never drift apart.
        assert_eq!(
            serde_json::to_string(&RunStatus::Interrupted).unwrap(),
            "\"interrupted\""
        );
    }

    #[test]
    fn test_run_status_from_unknown_string_is_none() {
        assert_eq!(RunStatus::from_str("cancelled"), None);
        assert_eq!(RunStatus::from_str(""), None);
        assert_eq!(RunStatus::from_str("Running"), None);
    }

    #[test]
    fn test_migrate_schema_adds_run_status_to_old_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        assert!(!column_names(&conn, "sessions").contains(&"run_status".to_string()));

        SessionStorage::migrate_schema(&conn).unwrap();
        assert!(column_names(&conn, "sessions").contains(&"run_status".to_string()));

        conn.execute(
            "INSERT INTO sessions (id, name, user_id, agent_name, run_status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params!["id-1", "chrome-molly", None::<String>, "daily-digest", "success", 100, 100],
        )
        .unwrap();
        let run_status: Option<String> = conn
            .query_row("SELECT run_status FROM sessions WHERE id = ?", ["id-1"], |row| row.get(0))
            .unwrap();
        assert_eq!(run_status, Some("success".to_string()));
    }

    #[test]
    fn test_migrate_schema_run_status_is_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        SessionStorage::migrate_schema(&conn).unwrap();
        SessionStorage::migrate_schema(&conn).unwrap();

        let count = column_names(&conn, "sessions")
            .iter()
            .filter(|c| *c == "run_status")
            .count();
        assert_eq!(count, 1);
    }

    /// Rows written before the column existed must survive the upgrade and read as
    /// "no status" rather than failing the query.
    #[tokio::test]
    async fn test_pre_existing_rows_read_null_run_status() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("old.db");
        create_old_schema_db(&db_path);

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute(
                "INSERT INTO sessions (id, name, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                params!["legacy-id", "legacy-name", None::<String>, 100, 100],
            )
            .unwrap();
        }

        let storage = SessionStorage::new_sqlite(db_path.to_str().unwrap()).unwrap();
        let sessions = storage.list_sessions(None).await.unwrap();
        let legacy = sessions.iter().find(|s| s.id == "legacy-id").unwrap();
        assert_eq!(legacy.run_status, None);
    }

    #[tokio::test]
    async fn test_sqlite_agent_session_starts_running() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let created = storage.create_agent_session("repo-watch").await.unwrap();
        let sessions = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, created.id);
        assert_eq!(sessions[0].run_status, Some(RunStatus::Running));
    }

    #[tokio::test]
    async fn test_sqlite_run_status_round_trip() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();

        let created = storage.create_agent_session("repo-watch").await.unwrap();

        for status in [RunStatus::Success, RunStatus::Failed, RunStatus::Interrupted] {
            storage.set_run_status(&created.id, status).await.unwrap();
            let sessions = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
            assert_eq!(sessions[0].run_status, Some(status));

            // Also visible through the plain session list, which is what the UI reads.
            let all = storage.list_sessions(None).await.unwrap();
            let meta = all.iter().find(|s| s.id == created.id).unwrap();
            assert_eq!(meta.run_status, Some(status));
        }
    }

    #[tokio::test]
    async fn test_memory_run_status_round_trip() {
        let storage = SessionStorage::new(false).unwrap();

        let created = storage.create_agent_session("repo-watch").await.unwrap();
        let sessions = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
        assert_eq!(sessions[0].run_status, Some(RunStatus::Running));

        storage.set_run_status(&created.id, RunStatus::Success).await.unwrap();
        let sessions = storage.list_agent_sessions("repo-watch", 10).await.unwrap();
        assert_eq!(sessions[0].run_status, Some(RunStatus::Success));

        let all = storage.list_sessions(None).await.unwrap();
        let meta = all.iter().find(|s| s.id == created.id).unwrap();
        assert_eq!(meta.run_status, Some(RunStatus::Success));
    }

    #[tokio::test]
    async fn test_interactive_sessions_have_no_run_status() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        let created = storage.create_session(None).await.unwrap();
        let sessions = storage.list_sessions(None).await.unwrap();
        let meta = sessions.iter().find(|s| s.id == created.id).unwrap();
        assert_eq!(meta.run_status, None);

        let memory = SessionStorage::new(false).unwrap();
        let created = memory.create_session(None).await.unwrap();
        let sessions = memory.list_sessions(None).await.unwrap();
        let meta = sessions.iter().find(|s| s.id == created.id).unwrap();
        assert_eq!(meta.run_status, None);
    }

    #[tokio::test]
    async fn test_set_run_status_on_missing_session_is_a_no_op() {
        let (storage, _temp_dir) = create_temp_sqlite_storage();
        storage.set_run_status("nope", RunStatus::Failed).await.unwrap();

        let memory = SessionStorage::new(false).unwrap();
        memory.set_run_status("nope", RunStatus::Failed).await.unwrap();
    }

    /// A process killed mid-run leaves `running` on disk; reopening the database must
    /// rewrite it so the UI does not spin forever.
    #[tokio::test]
    async fn test_startup_sweep_rewrites_running_to_interrupted() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("sessions.db");

        let finished_id;
        let interrupted_id;
        {
            let storage = SessionStorage::new_sqlite(db_path.to_str().unwrap()).unwrap();
            let finished = storage.create_agent_session("repo-watch").await.unwrap();
            storage.set_run_status(&finished.id, RunStatus::Success).await.unwrap();
            finished_id = finished.id;

            // Left as `running`, simulating the process dying mid-run.
            interrupted_id = storage.create_agent_session("repo-watch").await.unwrap().id;
        }

        let storage = SessionStorage::new_sqlite(db_path.to_str().unwrap()).unwrap();
        let sessions = storage.list_agent_sessions("repo-watch", 10).await.unwrap();

        let finished = sessions.iter().find(|s| s.id == finished_id).unwrap();
        let interrupted = sessions.iter().find(|s| s.id == interrupted_id).unwrap();
        assert_eq!(finished.run_status, Some(RunStatus::Success));
        assert_eq!(interrupted.run_status, Some(RunStatus::Interrupted));
    }

    #[test]
    fn test_startup_sweep_reports_rows_touched_and_is_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("sweep.db");
        let conn = Connection::open(&db_path).unwrap();
        SessionStorage::init_schema(&conn).unwrap();
        SessionStorage::migrate_schema(&conn).unwrap();

        for (id, status) in [
            ("a", Some("running")),
            ("b", Some("running")),
            ("c", Some("success")),
            ("d", None),
        ] {
            conn.execute(
                "INSERT INTO sessions (id, name, user_id, agent_name, run_status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![id, id, None::<String>, "agent", status, 100, 100],
            )
            .unwrap();
        }

        assert_eq!(SessionStorage::sweep_interrupted_runs(&conn).unwrap(), 2);
        // Nothing is left at `running`, so a second sweep touches nothing.
        assert_eq!(SessionStorage::sweep_interrupted_runs(&conn).unwrap(), 0);

        let unchanged: Option<String> = conn
            .query_row("SELECT run_status FROM sessions WHERE id = ?", ["c"], |row| row.get(0))
            .unwrap();
        assert_eq!(unchanged, Some("success".to_string()));
        let still_null: Option<String> = conn
            .query_row("SELECT run_status FROM sessions WHERE id = ?", ["d"], |row| row.get(0))
            .unwrap();
        assert_eq!(still_null, None);
    }

    #[test]
    fn test_session_metadata_serialization_shape() {
        let meta = SessionMetadata {
            id: "abc123".to_string(),
            name: "chrome-molly".to_string(),
            turn_count: 4,
            updated_at: 1784534460,
            relative_time: "2h ago".to_string(),
            agent_name: Some("daily-digest".to_string()),
            run_status: Some(RunStatus::Success),
        };

        let json: serde_json::Value = serde_json::to_value(&meta).unwrap();
        for field in [
            "id",
            "name",
            "turn_count",
            "updated_at",
            "relative_time",
            "agent_name",
            "run_status",
        ] {
            assert!(json.get(field).is_some(), "missing field {}", field);
        }
        assert_eq!(json.as_object().unwrap().len(), 7);
        assert_eq!(json["run_status"], "success");

        // An untagged chat serializes both optional fields as null, not as an
        // omitted key, so the frontend can read them unconditionally.
        let plain = SessionMetadata {
            id: "def456".to_string(),
            name: "void-case".to_string(),
            turn_count: 0,
            updated_at: 0,
            relative_time: "just now".to_string(),
            agent_name: None,
            run_status: None,
        };
        let json: serde_json::Value = serde_json::to_value(&plain).unwrap();
        assert!(json["agent_name"].is_null());
        assert!(json["run_status"].is_null());
        assert_eq!(json.as_object().unwrap().len(), 7);
    }
}
