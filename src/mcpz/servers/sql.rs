use anyhow::{anyhow, Result};
use serde::Serialize;
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::postgres::{PgPool, PgRow};
use sqlx::sqlite::{SqlitePool, SqliteRow};
use sqlx::{Column, Row, TypeInfo};
use std::time::Duration;

use super::common::{error_content, text_content, McpServer, McpTool};

/// Access mode for the SQL server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Only SELECT queries allowed
    ReadOnly,
    /// All SQL statements allowed (SELECT, INSERT, UPDATE, DELETE, etc.)
    FullAccess,
}

/// Database type detected from connection string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl DatabaseType {
    /// Detect database type from connection string
    pub fn from_connection_string(conn: &str) -> Result<Self> {
        if conn.starts_with("postgres://") || conn.starts_with("postgresql://") {
            Ok(DatabaseType::PostgreSQL)
        } else if conn.starts_with("mysql://") || conn.starts_with("mariadb://") {
            Ok(DatabaseType::MySQL)
        } else if conn.starts_with("sqlite://") || conn.starts_with("sqlite:") {
            Ok(DatabaseType::SQLite)
        } else {
            Err(anyhow!(
                "Unsupported database type. Connection string must start with postgres://, mysql://, or sqlite://"
            ))
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
            DatabaseType::SQLite => "SQLite",
        }
    }
}

/// Native database pool - holds the specific driver's pool
pub enum DatabasePool {
    PostgreSQL(PgPool),
    MySQL(MySqlPool),
    SQLite(SqlitePool),
}

/// Configuration for the SQL server
pub struct SqlServerConfig {
    pub connection_string: String,
    pub access_mode: AccessMode,
    pub timeout: Duration,
    pub verbose: bool,
    pub db_type: DatabaseType,
}

impl SqlServerConfig {
    pub fn new(connection_string: String, access_mode: AccessMode, timeout: u64, verbose: bool) -> Result<Self> {
        let db_type = DatabaseType::from_connection_string(&connection_string)?;
        Ok(Self {
            connection_string,
            access_mode,
            timeout: Duration::from_secs(timeout),
            verbose,
            db_type,
        })
    }

    /// Check if a SQL statement is allowed based on access mode
    pub fn is_statement_allowed(&self, sql: &str) -> bool {
        if self.access_mode == AccessMode::FullAccess {
            return true;
        }

        // In readonly mode, only allow SELECT statements
        let trimmed = sql.trim().to_uppercase();

        // Allow SELECT, WITH (for CTEs that result in SELECT), EXPLAIN, SHOW, DESCRIBE
        trimmed.starts_with("SELECT")
            || trimmed.starts_with("WITH")
            || trimmed.starts_with("EXPLAIN")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("DESCRIBE")
            || trimmed.starts_with("DESC")
            || trimmed.starts_with("PRAGMA") // SQLite introspection
    }
}

/// Query result for serialization
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
}

/// Execute result for non-SELECT statements
#[derive(Debug, Serialize)]
pub struct ExecuteResult {
    pub rows_affected: u64,
    pub message: String,
}

/// Table info for list_tables
#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub table_type: String,
}

/// Column info for describe_table
#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

/// SQL MCP server with native driver support
pub struct SqlServer {
    config: SqlServerConfig,
    pool: DatabasePool,
    runtime: tokio::runtime::Runtime,
}

impl SqlServer {
    pub fn new(config: SqlServerConfig, pool: DatabasePool, runtime: tokio::runtime::Runtime) -> Self {
        Self {
            config,
            pool,
            runtime,
        }
    }

    /// Convert a PostgreSQL row to JSON values
    fn pg_row_to_json(row: &PgRow) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for i in 0..row.columns().len() {
            let col = &row.columns()[i];
            let type_name = col.type_info().name();

            let value: serde_json::Value = match type_name {
                "INT2" | "INT4" | "INT8" | "SERIAL" | "BIGSERIAL" => {
                    row.try_get::<i64, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "FLOAT4" | "FLOAT8" | "NUMERIC" | "DECIMAL" => {
                    row.try_get::<f64, _>(i)
                        .map(serde_json::Value::from)
                        .or_else(|_| {
                            // Try as string for high-precision decimals
                            row.try_get::<String, _>(i).map(serde_json::Value::from)
                        })
                        .unwrap_or(serde_json::Value::Null)
                }
                "BOOL" => {
                    row.try_get::<bool, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "DATE" | "TIME" | "TIMESTAMP" | "TIMESTAMPTZ" => {
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::from)
                        .or_else(|_| {
                            row.try_get::<chrono::NaiveDate, _>(i)
                                .map(|d| serde_json::Value::from(d.to_string()))
                        })
                        .or_else(|_| {
                            row.try_get::<chrono::NaiveDateTime, _>(i)
                                .map(|d| serde_json::Value::from(d.to_string()))
                        })
                        .unwrap_or(serde_json::Value::Null)
                }
                "JSON" | "JSONB" => {
                    row.try_get::<serde_json::Value, _>(i)
                        .unwrap_or(serde_json::Value::Null)
                }
                _ => {
                    // Default to string
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
            };
            values.push(value);
        }
        values
    }

    /// Convert a MySQL row to JSON values
    fn mysql_row_to_json(row: &MySqlRow) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for i in 0..row.columns().len() {
            let col = &row.columns()[i];
            let type_name = col.type_info().name();

            let value: serde_json::Value = match type_name {
                "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" | "BIGINT" => {
                    row.try_get::<i64, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "FLOAT" | "DOUBLE" => {
                    row.try_get::<f64, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "DECIMAL" | "NUMERIC" => {
                    // MySQL DECIMAL - try as string first for precision
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::from)
                        .or_else(|_| {
                            row.try_get::<f64, _>(i).map(serde_json::Value::from)
                        })
                        .unwrap_or(serde_json::Value::Null)
                }
                "DATE" => {
                    row.try_get::<chrono::NaiveDate, _>(i)
                        .map(|d| serde_json::Value::from(d.to_string()))
                        .unwrap_or(serde_json::Value::Null)
                }
                "TIME" => {
                    row.try_get::<chrono::NaiveTime, _>(i)
                        .map(|t| serde_json::Value::from(t.to_string()))
                        .unwrap_or(serde_json::Value::Null)
                }
                "DATETIME" | "TIMESTAMP" => {
                    row.try_get::<chrono::NaiveDateTime, _>(i)
                        .map(|d| serde_json::Value::from(d.to_string()))
                        .unwrap_or(serde_json::Value::Null)
                }
                "BOOLEAN" | "BOOL" => {
                    row.try_get::<bool, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "JSON" => {
                    row.try_get::<serde_json::Value, _>(i)
                        .unwrap_or(serde_json::Value::Null)
                }
                _ => {
                    // VARCHAR, TEXT, CHAR, BLOB, etc. - try as string
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::from)
                        .or_else(|_| {
                            row.try_get::<Vec<u8>, _>(i)
                                .map(|b| serde_json::Value::from(String::from_utf8_lossy(&b).to_string()))
                        })
                        .unwrap_or(serde_json::Value::Null)
                }
            };
            values.push(value);
        }
        values
    }

    /// Convert a SQLite row to JSON values
    fn sqlite_row_to_json(row: &SqliteRow) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for i in 0..row.columns().len() {
            let col = &row.columns()[i];
            let type_name = col.type_info().name().to_uppercase();

            let value: serde_json::Value = match type_name.as_str() {
                "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                    row.try_get::<i64, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" => {
                    row.try_get::<f64, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "BOOLEAN" | "BOOL" => {
                    row.try_get::<bool, _>(i)
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                }
                "NULL" => serde_json::Value::Null,
                _ => {
                    // TEXT, BLOB, etc.
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::from)
                        .or_else(|_| {
                            row.try_get::<Vec<u8>, _>(i)
                                .map(|b| serde_json::Value::from(String::from_utf8_lossy(&b).to_string()))
                        })
                        .unwrap_or(serde_json::Value::Null)
                }
            };
            values.push(value);
        }
        values
    }

    /// Execute a query and return results
    fn execute_query(&self, sql: &str) -> Result<QueryResult> {
        if !self.config.is_statement_allowed(sql) {
            return Err(anyhow!(
                "Statement not allowed in readonly mode. Only SELECT, SHOW, DESCRIBE, and EXPLAIN are permitted."
            ));
        }

        self.log(&format!("Executing query: {}", sql));

        match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<PgRow> = sqlx::query(sql).fetch_all(pool).await?;
                    if rows.is_empty() {
                        return Ok(QueryResult { columns: vec![], rows: vec![], row_count: 0 });
                    }
                    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
                    let json_rows: Vec<Vec<serde_json::Value>> = rows.iter().map(Self::pg_row_to_json).collect();
                    let row_count = json_rows.len();
                    Ok(QueryResult { columns, rows: json_rows, row_count })
                })
            }
            DatabasePool::MySQL(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<MySqlRow> = sqlx::query(sql).fetch_all(pool).await?;
                    if rows.is_empty() {
                        return Ok(QueryResult { columns: vec![], rows: vec![], row_count: 0 });
                    }
                    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
                    let json_rows: Vec<Vec<serde_json::Value>> = rows.iter().map(Self::mysql_row_to_json).collect();
                    let row_count = json_rows.len();
                    Ok(QueryResult { columns, rows: json_rows, row_count })
                })
            }
            DatabasePool::SQLite(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<SqliteRow> = sqlx::query(sql).fetch_all(pool).await?;
                    if rows.is_empty() {
                        return Ok(QueryResult { columns: vec![], rows: vec![], row_count: 0 });
                    }
                    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
                    let json_rows: Vec<Vec<serde_json::Value>> = rows.iter().map(Self::sqlite_row_to_json).collect();
                    let row_count = json_rows.len();
                    Ok(QueryResult { columns, rows: json_rows, row_count })
                })
            }
        }
    }

    /// Execute a statement (INSERT, UPDATE, DELETE, etc.)
    fn execute_statement(&self, sql: &str) -> Result<ExecuteResult> {
        if self.config.access_mode == AccessMode::ReadOnly {
            return Err(anyhow!(
                "Write operations not allowed in readonly mode. Use --fullaccess to enable."
            ));
        }

        self.log(&format!("Executing statement: {}", sql));

        let rows_affected = match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                self.runtime.block_on(async {
                    let result = sqlx::query(sql).execute(pool).await?;
                    Ok::<u64, anyhow::Error>(result.rows_affected())
                })?
            }
            DatabasePool::MySQL(pool) => {
                self.runtime.block_on(async {
                    let result = sqlx::query(sql).execute(pool).await?;
                    Ok::<u64, anyhow::Error>(result.rows_affected())
                })?
            }
            DatabasePool::SQLite(pool) => {
                self.runtime.block_on(async {
                    let result = sqlx::query(sql).execute(pool).await?;
                    Ok::<u64, anyhow::Error>(result.rows_affected())
                })?
            }
        };

        Ok(ExecuteResult {
            rows_affected,
            message: format!("Statement executed successfully. {} row(s) affected.", rows_affected),
        })
    }

    /// List all tables in the database
    fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let sql = match self.config.db_type {
            DatabaseType::PostgreSQL => {
                "SELECT table_name as name, table_type FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
            DatabaseType::MySQL => {
                "SELECT table_name as name, table_type FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name"
            }
            DatabaseType::SQLite => {
                "SELECT name, type as table_type FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name"
            }
        };

        self.log(&format!("Listing tables with: {}", sql));

        match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<PgRow> = sqlx::query(sql).fetch_all(pool).await?;
                    let tables: Vec<TableInfo> = rows.iter().map(|row| {
                        let name: String = row.try_get("name").unwrap_or_default();
                        let table_type: String = row.try_get("table_type").unwrap_or_else(|_| "TABLE".to_string());
                        TableInfo { name, table_type }
                    }).collect();
                    Ok(tables)
                })
            }
            DatabasePool::MySQL(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<MySqlRow> = sqlx::query(sql).fetch_all(pool).await?;
                    let tables: Vec<TableInfo> = rows.iter().map(|row| {
                        let name: String = row.try_get("name").unwrap_or_default();
                        let table_type: String = row.try_get("table_type").unwrap_or_else(|_| "TABLE".to_string());
                        TableInfo { name, table_type }
                    }).collect();
                    Ok(tables)
                })
            }
            DatabasePool::SQLite(pool) => {
                self.runtime.block_on(async {
                    let rows: Vec<SqliteRow> = sqlx::query(sql).fetch_all(pool).await?;
                    let tables: Vec<TableInfo> = rows.iter().map(|row| {
                        let name: String = row.try_get("name").unwrap_or_default();
                        let table_type: String = row.try_get("table_type").unwrap_or_else(|_| "TABLE".to_string());
                        TableInfo { name, table_type }
                    }).collect();
                    Ok(tables)
                })
            }
        }
    }

    /// Describe a table's schema
    fn describe_table(&self, table_name: &str) -> Result<Vec<ColumnInfo>> {
        // Sanitize table name to prevent SQL injection
        if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(anyhow!("Invalid table name"));
        }

        match self.config.db_type {
            DatabaseType::PostgreSQL => {
                let sql = format!(
                    "SELECT column_name as name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
                    table_name
                );
                self.log(&format!("Describing table with: {}", sql));

                if let DatabasePool::PostgreSQL(pool) = &self.pool {
                    self.runtime.block_on(async {
                        let rows: Vec<PgRow> = sqlx::query(&sql).fetch_all(pool).await?;
                        let columns: Vec<ColumnInfo> = rows.iter().map(|row| {
                            let name: String = row.try_get("name").unwrap_or_default();
                            let data_type: String = row.try_get("data_type").unwrap_or_default();
                            let is_nullable: String = row.try_get("is_nullable").unwrap_or_else(|_| "YES".to_string());
                            ColumnInfo { name, data_type, is_nullable: is_nullable.to_uppercase() == "YES" }
                        }).collect();
                        Ok(columns)
                    })
                } else {
                    Err(anyhow!("Pool type mismatch"))
                }
            }
            DatabaseType::MySQL => {
                let sql = format!(
                    "SELECT column_name as name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE() ORDER BY ordinal_position",
                    table_name
                );
                self.log(&format!("Describing table with: {}", sql));

                if let DatabasePool::MySQL(pool) = &self.pool {
                    self.runtime.block_on(async {
                        let rows: Vec<MySqlRow> = sqlx::query(&sql).fetch_all(pool).await?;
                        let columns: Vec<ColumnInfo> = rows.iter().map(|row| {
                            let name: String = row.try_get("name").unwrap_or_default();
                            let data_type: String = row.try_get("data_type").unwrap_or_default();
                            let is_nullable: String = row.try_get("is_nullable").unwrap_or_else(|_| "YES".to_string());
                            ColumnInfo { name, data_type, is_nullable: is_nullable.to_uppercase() == "YES" }
                        }).collect();
                        Ok(columns)
                    })
                } else {
                    Err(anyhow!("Pool type mismatch"))
                }
            }
            DatabaseType::SQLite => {
                let sql = format!("PRAGMA table_info({})", table_name);
                self.log(&format!("Describing table with: {}", sql));

                if let DatabasePool::SQLite(pool) = &self.pool {
                    self.runtime.block_on(async {
                        let rows: Vec<SqliteRow> = sqlx::query(&sql).fetch_all(pool).await?;
                        let columns: Vec<ColumnInfo> = rows.iter().map(|row| {
                            let name: String = row.try_get("name").unwrap_or_default();
                            let data_type: String = row.try_get("type").unwrap_or_default();
                            let notnull: i32 = row.try_get("notnull").unwrap_or(0);
                            ColumnInfo { name, data_type, is_nullable: notnull == 0 }
                        }).collect();
                        Ok(columns)
                    })
                } else {
                    Err(anyhow!("Pool type mismatch"))
                }
            }
        }
    }
}

impl McpServer for SqlServer {
    fn name(&self) -> &str {
        "mcpz-sql"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn verbose(&self) -> bool {
        self.config.verbose
    }

    fn tools(&self) -> Vec<McpTool> {
        let mut tools = vec![
            McpTool {
                name: "query".to_string(),
                description: "Execute a SQL query and return results. Use for SELECT statements and data retrieval.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": {
                            "type": "string",
                            "description": "SQL query to execute (SELECT, SHOW, DESCRIBE, EXPLAIN)"
                        }
                    },
                    "required": ["sql"]
                }),
            },
            McpTool {
                name: "list_tables".to_string(),
                description: "List all tables and views in the database".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "describe_table".to_string(),
                description: "Get the schema/structure of a specific table".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Name of the table to describe"
                        }
                    },
                    "required": ["table_name"]
                }),
            },
        ];

        // Only add execute tool in fullaccess mode
        if self.config.access_mode == AccessMode::FullAccess {
            tools.push(McpTool {
                name: "execute".to_string(),
                description: "Execute a SQL statement that modifies data (INSERT, UPDATE, DELETE, CREATE, DROP, etc.)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": {
                            "type": "string",
                            "description": "SQL statement to execute"
                        }
                    },
                    "required": ["sql"]
                }),
            });
        }

        tools
    }

    fn call_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        match name {
            "query" => {
                let sql = arguments
                    .get("sql")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow!("Missing sql argument"))?;

                match self.execute_query(sql) {
                    Ok(result) => {
                        let result_json = serde_json::to_string_pretty(&result)?;
                        Ok(text_content(&result_json))
                    }
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "execute" => {
                let sql = arguments
                    .get("sql")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow!("Missing sql argument"))?;

                match self.execute_statement(sql) {
                    Ok(result) => {
                        let result_json = serde_json::to_string_pretty(&result)?;
                        Ok(text_content(&result_json))
                    }
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "list_tables" => match self.list_tables() {
                Ok(tables) => {
                    let result_json = serde_json::to_string_pretty(&tables)?;
                    Ok(text_content(&result_json))
                }
                Err(e) => Ok(error_content(&e.to_string())),
            },
            "describe_table" => {
                let table_name = arguments
                    .get("table_name")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow!("Missing table_name argument"))?;

                match self.describe_table(table_name) {
                    Ok(columns) => {
                        let result_json = serde_json::to_string_pretty(&columns)?;
                        Ok(text_content(&result_json))
                    }
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            _ => Ok(error_content(&format!("Unknown tool: {}", name))),
        }
    }
}

/// Connect to database and return native pool
pub async fn connect_database(connection_string: &str, db_type: DatabaseType, timeout: Duration) -> Result<DatabasePool> {
    match db_type {
        DatabaseType::PostgreSQL => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(timeout)
                .connect(connection_string)
                .await?;
            Ok(DatabasePool::PostgreSQL(pool))
        }
        DatabaseType::MySQL => {
            let pool = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(timeout)
                .connect(connection_string)
                .await?;
            Ok(DatabasePool::MySQL(pool))
        }
        DatabaseType::SQLite => {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5)
                .acquire_timeout(timeout)
                .connect(connection_string)
                .await?;
            Ok(DatabasePool::SQLite(pool))
        }
    }
}

/// Create and run the SQL MCP server
pub fn run_sql_server(config: SqlServerConfig) -> Result<()> {
    if config.verbose {
        eprintln!("[mcpz] SQL server configuration:");
        eprintln!("[mcpz]   Database: {}", config.db_type.name());
        eprintln!("[mcpz]   Access mode: {:?}", config.access_mode);
        eprintln!("[mcpz]   Timeout: {:?}", config.timeout);
    }

    // Create tokio runtime for async SQL operations
    let runtime = tokio::runtime::Runtime::new()?;

    // Connect to database using native driver
    let pool = runtime.block_on(connect_database(
        &config.connection_string,
        config.db_type,
        config.timeout,
    ))?;

    if config.verbose {
        eprintln!("[mcpz] Connected to {} database successfully", config.db_type.name());
    }

    let server = SqlServer::new(config, pool, runtime);
    server.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_type_detection() {
        assert_eq!(
            DatabaseType::from_connection_string("postgres://localhost/test").unwrap(),
            DatabaseType::PostgreSQL
        );
        assert_eq!(
            DatabaseType::from_connection_string("postgresql://localhost/test").unwrap(),
            DatabaseType::PostgreSQL
        );
        assert_eq!(
            DatabaseType::from_connection_string("mysql://localhost/test").unwrap(),
            DatabaseType::MySQL
        );
        assert_eq!(
            DatabaseType::from_connection_string("mariadb://localhost/test").unwrap(),
            DatabaseType::MySQL
        );
        assert_eq!(
            DatabaseType::from_connection_string("sqlite:///tmp/test.db").unwrap(),
            DatabaseType::SQLite
        );
        assert_eq!(
            DatabaseType::from_connection_string("sqlite::memory:").unwrap(),
            DatabaseType::SQLite
        );
        assert!(DatabaseType::from_connection_string("unknown://localhost").is_err());
    }

    #[test]
    fn test_sql_config_is_statement_allowed_readonly() {
        let config = SqlServerConfig::new(
            "postgres://localhost/test".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        // Allowed in readonly
        assert!(config.is_statement_allowed("SELECT * FROM users"));
        assert!(config.is_statement_allowed("select * from users"));
        assert!(config.is_statement_allowed("  SELECT * FROM users"));
        assert!(config.is_statement_allowed("WITH cte AS (SELECT 1) SELECT * FROM cte"));
        assert!(config.is_statement_allowed("EXPLAIN SELECT * FROM users"));
        assert!(config.is_statement_allowed("SHOW TABLES"));
        assert!(config.is_statement_allowed("DESCRIBE users"));
        assert!(config.is_statement_allowed("DESC users"));
        assert!(config.is_statement_allowed("PRAGMA table_info(users)"));

        // Not allowed in readonly
        assert!(!config.is_statement_allowed("INSERT INTO users VALUES (1)"));
        assert!(!config.is_statement_allowed("UPDATE users SET name = 'test'"));
        assert!(!config.is_statement_allowed("DELETE FROM users"));
        assert!(!config.is_statement_allowed("DROP TABLE users"));
        assert!(!config.is_statement_allowed("CREATE TABLE test (id INT)"));
        assert!(!config.is_statement_allowed("ALTER TABLE users ADD COLUMN test INT"));
        assert!(!config.is_statement_allowed("TRUNCATE users"));
    }

    #[test]
    fn test_sql_config_is_statement_allowed_fullaccess() {
        let config = SqlServerConfig::new(
            "postgres://localhost/test".to_string(),
            AccessMode::FullAccess,
            30,
            false,
        ).unwrap();

        // All allowed in fullaccess
        assert!(config.is_statement_allowed("SELECT * FROM users"));
        assert!(config.is_statement_allowed("INSERT INTO users VALUES (1)"));
        assert!(config.is_statement_allowed("UPDATE users SET name = 'test'"));
        assert!(config.is_statement_allowed("DELETE FROM users"));
        assert!(config.is_statement_allowed("DROP TABLE users"));
        assert!(config.is_statement_allowed("CREATE TABLE test (id INT)"));
    }

    #[test]
    fn test_sql_server_tools_readonly() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap()
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);
        let tools = server.tools();

        // Should have query, list_tables, describe_table but NOT execute
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.name == "query"));
        assert!(tools.iter().any(|t| t.name == "list_tables"));
        assert!(tools.iter().any(|t| t.name == "describe_table"));
        assert!(!tools.iter().any(|t| t.name == "execute"));
    }

    #[test]
    fn test_sql_server_tools_fullaccess() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap()
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::FullAccess,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);
        let tools = server.tools();

        // Should have all 4 tools including execute
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.name == "query"));
        assert!(tools.iter().any(|t| t.name == "list_tables"));
        assert!(tools.iter().any(|t| t.name == "describe_table"));
        assert!(tools.iter().any(|t| t.name == "execute"));
    }

    #[test]
    fn test_sql_server_query_sqlite() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            // Create a test table
            sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
                .execute(&pool)
                .await
                .unwrap();

            sqlx::query("INSERT INTO test (id, name) VALUES (1, 'Alice'), (2, 'Bob')")
                .execute(&pool)
                .await
                .unwrap();

            pool
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);

        // Test query
        let result = server.execute_query("SELECT * FROM test ORDER BY id").unwrap();
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns, vec!["id", "name"]);
    }

    #[test]
    fn test_sql_server_readonly_blocks_write() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();

            pool
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);

        // Try to execute write statement
        let result = server.execute_statement("INSERT INTO test (id) VALUES (1)");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("readonly"));
    }

    #[test]
    fn test_sql_server_list_tables_sqlite() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();

            sqlx::query("CREATE TABLE posts (id INTEGER PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();

            pool
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);

        let tables = server.list_tables().unwrap();
        assert_eq!(tables.len(), 2);
        assert!(tables.iter().any(|t| t.name == "users"));
        assert!(tables.iter().any(|t| t.name == "posts"));
    }

    #[test]
    fn test_sql_server_describe_table_sqlite() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)")
                .execute(&pool)
                .await
                .unwrap();

            pool
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);

        let columns = server.describe_table("users").unwrap();
        assert_eq!(columns.len(), 3);

        let id_col = columns.iter().find(|c| c.name == "id").unwrap();
        assert_eq!(id_col.data_type, "INTEGER");

        let name_col = columns.iter().find(|c| c.name == "name").unwrap();
        assert!(!name_col.is_nullable);

        let email_col = columns.iter().find(|c| c.name == "email").unwrap();
        assert!(email_col.is_nullable);
    }

    #[test]
    fn test_sql_server_call_tool_query() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();

            sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
                .execute(&pool)
                .await
                .unwrap();

            sqlx::query("INSERT INTO test VALUES (1, 'hello')")
                .execute(&pool)
                .await
                .unwrap();

            pool
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);

        let result = server.call_tool("query", &serde_json::json!({"sql": "SELECT * FROM test"})).unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("hello"));
        assert!(text.contains("row_count"));
    }

    #[test]
    fn test_sql_server_initialize() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let pool = runtime.block_on(async {
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap()
        });

        let config = SqlServerConfig::new(
            "sqlite::memory:".to_string(),
            AccessMode::ReadOnly,
            30,
            false,
        ).unwrap();

        let server = SqlServer::new(config, DatabasePool::SQLite(pool), runtime);
        let result = server.handle_initialize();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "mcpz-sql");
    }
}
