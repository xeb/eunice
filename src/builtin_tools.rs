//! Built-in tools for direct execution without MCP overhead.
//!
//! This module provides shell, filesystem, and browser tools that can be executed
//! directly in Rust, bypassing the MCP subprocess overhead.

use crate::mcpz::servers::common::{McpServer, McpTool};
use crate::mcpz::servers::filesystem::{FilesystemServer, FilesystemServerConfig};
use crate::mcpz::servers::shell::{ShellServer, ShellServerConfig};
use crate::models::{FunctionSpec, Tool};
use anyhow::Result;
use std::path::PathBuf;

/// Registry of built-in tools for direct execution
pub struct BuiltinToolRegistry {
    shell: Option<ShellServer>,
    filesystem: Option<FilesystemServer>,
    // Browser is more complex (requires async Chrome connection), handle separately
}

impl BuiltinToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            shell: None,
            filesystem: None,
        }
    }

    /// Enable shell tools with the given working directory
    pub fn with_shell(mut self, working_dir: Option<PathBuf>) -> Self {
        let config = ShellServerConfig::new(
            working_dir,
            600,             // 10 minute timeout
            detect_shell(), // Use system shell
            None,           // No allow patterns (allow all)
            None,           // No deny patterns
            false,          // Include stderr
            false,          // Not verbose
        );
        self.shell = Some(ShellServer::new(config));
        self
    }

    /// Enable filesystem tools with the given allowed directories
    /// Returns None if the directories are invalid
    pub fn with_filesystem(mut self, allowed_dirs: Vec<PathBuf>) -> Self {
        let dirs = if allowed_dirs.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            allowed_dirs
        };
        match FilesystemServerConfig::new(dirs, false) {
            Ok(config) => {
                self.filesystem = Some(FilesystemServer::new(config));
            }
            Err(e) => {
                eprintln!("Warning: Could not enable filesystem tools: {}", e);
            }
        }
        self
    }

    /// Check if any tools are registered
    pub fn is_empty(&self) -> bool {
        self.shell.is_none() && self.filesystem.is_none()
    }

    /// Get all available tools in OpenAI format
    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();

        if let Some(ref shell) = self.shell {
            for mcp_tool in shell.tools() {
                tools.push(convert_mcp_tool_to_openai("shell", &mcp_tool));
            }
        }

        if let Some(ref filesystem) = self.filesystem {
            for mcp_tool in filesystem.tools() {
                tools.push(convert_mcp_tool_to_openai("filesystem", &mcp_tool));
            }
        }

        tools
    }

    /// Execute a tool by name
    pub fn execute_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<String> {
        // Check if this is a shell tool
        if let Some(actual_name) = tool_name.strip_prefix("shell_") {
            if let Some(ref shell) = self.shell {
                let result = shell.call_tool(actual_name, &arguments)?;
                return extract_text_from_mcp_result(&result);
            }
            return Err(anyhow::anyhow!("Shell tools not enabled"));
        }

        // Check if this is a filesystem tool
        if let Some(actual_name) = tool_name.strip_prefix("filesystem_") {
            if let Some(ref filesystem) = self.filesystem {
                let result = filesystem.call_tool(actual_name, &arguments)?;
                return extract_text_from_mcp_result(&result);
            }
            return Err(anyhow::anyhow!("Filesystem tools not enabled"));
        }

        Err(anyhow::anyhow!("Unknown built-in tool: {}", tool_name))
    }

    /// Check if a tool name belongs to this registry
    pub fn has_tool(&self, tool_name: &str) -> bool {
        if tool_name.starts_with("shell_") {
            self.shell.is_some()
        } else if tool_name.starts_with("filesystem_") {
            self.filesystem.is_some()
        } else {
            false
        }
    }
}

impl Default for BuiltinToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an MCP tool to OpenAI tool format
fn convert_mcp_tool_to_openai(prefix: &str, mcp_tool: &McpTool) -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: format!("{}_{}", prefix, mcp_tool.name),
            description: mcp_tool.description.clone(),
            parameters: mcp_tool.input_schema.clone(),
        },
    }
}

/// Extract text content from MCP result format
fn extract_text_from_mcp_result(result: &serde_json::Value) -> Result<String> {
    // MCP returns { "content": [{ "type": "text", "text": "..." }] }
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        let texts: Vec<String> = content
            .iter()
            .filter_map(|c| {
                if c.get("type").and_then(|t| t.as_str()) == Some("text") {
                    c.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        return Ok(texts.join("\n"));
    }

    // Fallback: just stringify the result
    Ok(serde_json::to_string_pretty(result)?)
}

/// Detect the system shell
fn detect_shell() -> String {
    if cfg!(target_os = "windows") {
        "cmd.exe".to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = BuiltinToolRegistry::new();
        assert!(registry.is_empty());
        assert!(registry.get_tools().is_empty());
    }

    #[test]
    fn test_shell_registry() {
        let registry = BuiltinToolRegistry::new().with_shell(None);
        assert!(!registry.is_empty());

        let tools = registry.get_tools();
        assert!(!tools.is_empty());

        // Should have execute_command tool
        let tool_names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(tool_names.contains(&"shell_execute_command"));
    }

    #[test]
    fn test_filesystem_registry() {
        let registry = BuiltinToolRegistry::new().with_filesystem(vec![]);
        assert!(!registry.is_empty());

        let tools = registry.get_tools();
        assert!(!tools.is_empty());

        // Should have read_file tool
        let tool_names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(tool_names.contains(&"filesystem_read_file"));
    }

    #[test]
    fn test_execute_shell_command() {
        let registry = BuiltinToolRegistry::new().with_shell(None);
        let result = registry.execute_tool(
            "shell_execute_command",
            serde_json::json!({"command": "echo hello"}),
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
        assert!(output.contains("Exit code: 0"));
    }

    #[test]
    fn test_has_tool() {
        let registry = BuiltinToolRegistry::new().with_shell(None);
        assert!(registry.has_tool("shell_execute_command"));
        assert!(!registry.has_tool("filesystem_read_file"));
    }

    #[test]
    fn test_extract_text_from_mcp_result() {
        let result = serde_json::json!({
            "content": [{
                "type": "text",
                "text": "Hello, World!"
            }]
        });
        assert_eq!(
            extract_text_from_mcp_result(&result).unwrap(),
            "Hello, World!"
        );
    }
}
