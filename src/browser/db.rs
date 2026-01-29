use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

/// Database for browser state, credentials, and sessions
pub struct BrowserDb {
    conn: Connection,
}

impl BrowserDb {
    /// Open or create the database
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists with restricted permissions
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS credentials (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                url_pattern TEXT NOT NULL,
                username TEXT,
                password TEXT,
                token TEXT,
                headers TEXT,
                cookies TEXT,
                notes TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                domain TEXT NOT NULL,
                cookies TEXT NOT NULL,
                local_storage TEXT,
                user_agent TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }

    // === State ===

    #[allow(dead_code)]
    pub fn get_state(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM state WHERE key = ?1", params![key], |row| {
                row.get(0)
            })
            .ok()
    }

    pub fn set_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO state (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_state(&self, key: &str) -> Result<()> {
        self.conn.execute("DELETE FROM state WHERE key = ?1", params![key])?;
        Ok(())
    }

    // === Credentials ===

    pub fn add_credential(
        &self,
        name: &str,
        url_pattern: &str,
        username: Option<&str>,
        password: Option<&str>,
        token: Option<&str>,
        headers: Option<&str>,
        cookies: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO credentials (name, url_pattern, username, password, token, headers, cookies, notes, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
            params![name, url_pattern, username, password, token, headers, cookies, notes],
        )?;
        Ok(())
    }

    pub fn list_credentials(&self) -> Result<Vec<CredentialRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, url_pattern, username, token IS NOT NULL, created_at FROM credentials ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CredentialRow {
                name: row.get(0)?,
                url_pattern: row.get(1)?,
                username: row.get(2)?,
                has_token: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_credential(&self, name: &str) -> Result<Option<Credential>> {
        let result = self.conn.query_row(
            "SELECT name, url_pattern, username, password, token, headers, cookies, notes FROM credentials WHERE name = ?1",
            params![name],
            |row| {
                Ok(Credential {
                    name: row.get(0)?,
                    url_pattern: row.get(1)?,
                    username: row.get(2)?,
                    password: row.get(3)?,
                    token: row.get(4)?,
                    headers: row.get(5)?,
                    cookies: row.get(6)?,
                    notes: row.get(7)?,
                })
            },
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_credential(&self, name: &str) -> Result<bool> {
        let count = self.conn.execute("DELETE FROM credentials WHERE name = ?1", params![name])?;
        Ok(count > 0)
    }

    // === Sessions ===

    pub fn save_session(
        &self,
        name: &str,
        domain: &str,
        cookies: &str,
        local_storage: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (name, domain, cookies, local_storage, user_agent, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![name, domain, cookies, local_storage, user_agent],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, domain, created_at FROM sessions ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionRow {
                name: row.get(0)?,
                domain: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_session(&self, name: &str) -> Result<Option<Session>> {
        let result = self.conn.query_row(
            "SELECT name, domain, cookies, local_storage, user_agent FROM sessions WHERE name = ?1",
            params![name],
            |row| {
                Ok(Session {
                    name: row.get(0)?,
                    domain: row.get(1)?,
                    cookies: row.get(2)?,
                    local_storage: row.get(3)?,
                    user_agent: row.get(4)?,
                })
            },
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_session(&self, name: &str) -> Result<bool> {
        let count = self.conn.execute("DELETE FROM sessions WHERE name = ?1", params![name])?;
        Ok(count > 0)
    }
}

#[derive(Debug)]
pub struct CredentialRow {
    pub name: String,
    pub url_pattern: String,
    pub username: Option<String>,
    pub has_token: bool,
    #[allow(dead_code)]
    pub created_at: Option<String>,
}

#[derive(Debug)]
pub struct Credential {
    pub name: String,
    pub url_pattern: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub headers: Option<String>,
    pub cookies: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct SessionRow {
    pub name: String,
    pub domain: String,
    pub created_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Session {
    pub name: String,
    pub domain: String,
    pub cookies: String,
    pub local_storage: Option<String>,
    pub user_agent: Option<String>,
}
