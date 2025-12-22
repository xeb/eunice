//! mcpz - Runtime MCP router tool for running MCP servers via npx, uvx, or cargo
//!
//! This module provides:
//! - Package routing (search across crates.io, PyPI, npm)
//! - Built-in MCP servers (shell, filesystem, sql, browser)
//! - HTTP transport support for all servers

pub mod cli;
pub mod http;
pub mod servers;

pub use cli::run;
