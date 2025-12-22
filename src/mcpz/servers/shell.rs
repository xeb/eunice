use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use super::common::{error_content, text_content, McpServer, McpTool};

/// Configuration for the shell server
pub struct ShellServerConfig {
    pub working_dir: Option<PathBuf>,
    pub timeout: Duration,
    pub shell: String,
    pub allow_patterns: Vec<String>,
    pub deny_patterns: Vec<String>,
    pub include_stderr: bool,
    pub verbose: bool,
}

impl ShellServerConfig {
    pub fn new(
        working_dir: Option<PathBuf>,
        timeout: u64,
        shell: String,
        allow: Option<String>,
        deny: Option<String>,
        no_stderr: bool,
        verbose: bool,
    ) -> Self {
        Self {
            working_dir,
            timeout: Duration::from_secs(timeout),
            shell,
            allow_patterns: allow
                .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                .unwrap_or_default(),
            deny_patterns: deny
                .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                .unwrap_or_default(),
            include_stderr: !no_stderr,
            verbose,
        }
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Check deny list first
        for pattern in &self.deny_patterns {
            if Self::matches_pattern(command, pattern) {
                return false;
            }
        }

        // If allow list is empty, allow all (that aren't denied)
        if self.allow_patterns.is_empty() {
            return true;
        }

        // Check allow list
        for pattern in &self.allow_patterns {
            if Self::matches_pattern(command, pattern) {
                return true;
            }
        }

        false
    }

    pub fn matches_pattern(command: &str, pattern: &str) -> bool {
        // Simple wildcard matching: "ls*" matches "ls -la"
        let cmd_first_word = command.split_whitespace().next().unwrap_or("");
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            cmd_first_word.starts_with(prefix)
        } else {
            cmd_first_word == pattern
        }
    }
}

/// Command execution result
#[derive(Serialize)]
pub struct ShellCommandResult {
    pub command: String,
    pub output: String,
    pub return_code: i32,
}

/// Shell MCP server
pub struct ShellServer {
    config: ShellServerConfig,
}

impl ShellServer {
    pub fn new(config: ShellServerConfig) -> Self {
        Self { config }
    }

    fn execute_command(&self, command: &str) -> ShellCommandResult {
        // Check sandboxing rules
        if !self.config.is_command_allowed(command) {
            self.log(&format!("Command denied by security policy: {}", command));
            return ShellCommandResult {
                command: command.to_string(),
                output: "Command denied by security policy".to_string(),
                return_code: -1,
            };
        }

        self.log(&format!("Executing: {}", command));

        let mut cmd = Command::new(&self.config.shell);
        cmd.arg("-c").arg(command);

        // Set working directory if specified
        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let combined = if self.config.include_stderr {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    format!("{}{}", stdout, stderr)
                } else {
                    stdout.to_string()
                };

                let return_code = output.status.code().unwrap_or(-1);
                self.log(&format!("Exit code: {}", return_code));

                ShellCommandResult {
                    command: command.to_string(),
                    output: combined,
                    return_code,
                }
            }
            Err(e) => {
                self.log(&format!("Error: {}", e));
                ShellCommandResult {
                    command: command.to_string(),
                    output: format!("Failed to execute: {}", e),
                    return_code: -1,
                }
            }
        }
    }
}

impl McpServer for ShellServer {
    fn name(&self) -> &str {
        "mcpz-shell"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn verbose(&self) -> bool {
        self.config.verbose
    }

    fn tools(&self) -> Vec<McpTool> {
        vec![McpTool {
            name: "execute_command".to_string(),
            description: "Execute a shell command and return its output".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        }]
    }

    fn call_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        if name != "execute_command" {
            return Ok(error_content(&format!("Unknown tool: {}", name)));
        }

        let command = arguments
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing command argument"))?;

        let result = self.execute_command(command);
        let result_json = serde_json::to_string_pretty(&result)?;

        Ok(text_content(&result_json))
    }
}

/// Run the shell MCP server
pub fn run_shell_server(config: ShellServerConfig) -> Result<()> {
    if config.verbose {
        eprintln!("[mcpz] Shell server configuration:");
        eprintln!("[mcpz]   Working dir: {:?}", config.working_dir);
        eprintln!("[mcpz]   Shell: {}", config.shell);
        eprintln!("[mcpz]   Timeout: {:?}", config.timeout);
        if !config.allow_patterns.is_empty() {
            eprintln!("[mcpz]   Allow patterns: {:?}", config.allow_patterns);
        }
        if !config.deny_patterns.is_empty() {
            eprintln!("[mcpz]   Deny patterns: {:?}", config.deny_patterns);
        }
    }

    let server = ShellServer::new(config);
    server.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_config_pattern_matching() {
        // Test wildcard matching
        assert!(ShellServerConfig::matches_pattern("ls -la", "ls*"));
        assert!(ShellServerConfig::matches_pattern("ls", "ls*"));
        assert!(ShellServerConfig::matches_pattern("lsblk", "ls*"));
        assert!(!ShellServerConfig::matches_pattern("cat file", "ls*"));

        // Test exact matching
        assert!(ShellServerConfig::matches_pattern("ls -la", "ls"));
        assert!(!ShellServerConfig::matches_pattern("lsblk", "ls"));
    }

    #[test]
    fn test_shell_config_is_command_allowed() {
        // No restrictions - allow all
        let config = ShellServerConfig::new(None, 30, "/bin/sh".to_string(), None, None, false, false);
        assert!(config.is_command_allowed("ls -la"));
        assert!(config.is_command_allowed("rm -rf /"));

        // Only allow list
        let config = ShellServerConfig::new(
            None,
            30,
            "/bin/sh".to_string(),
            Some("ls*,cat*".to_string()),
            None,
            false,
            false,
        );
        assert!(config.is_command_allowed("ls -la"));
        assert!(config.is_command_allowed("cat file"));
        assert!(!config.is_command_allowed("rm file"));

        // Only deny list
        let config = ShellServerConfig::new(
            None,
            30,
            "/bin/sh".to_string(),
            None,
            Some("rm*,sudo*".to_string()),
            false,
            false,
        );
        assert!(config.is_command_allowed("ls -la"));
        assert!(!config.is_command_allowed("rm file"));
        assert!(!config.is_command_allowed("sudo ls"));

        // Both allow and deny - deny takes precedence
        let config = ShellServerConfig::new(
            None,
            30,
            "/bin/sh".to_string(),
            Some("*".to_string()),
            Some("rm*".to_string()),
            false,
            false,
        );
        assert!(!config.is_command_allowed("rm file"));
    }

    #[test]
    fn test_execute_shell_command() {
        let config = ShellServerConfig::new(None, 30, "/bin/sh".to_string(), None, None, false, false);
        let server = ShellServer::new(config);
        let result = server.execute_command("echo hello");
        assert_eq!(result.command, "echo hello");
        assert!(result.output.contains("hello"));
        assert_eq!(result.return_code, 0);
    }

    #[test]
    fn test_execute_shell_command_denied() {
        let config = ShellServerConfig::new(
            None,
            30,
            "/bin/sh".to_string(),
            Some("ls*".to_string()),
            None,
            false,
            false,
        );
        let server = ShellServer::new(config);
        let result = server.execute_command("rm file");
        assert_eq!(result.return_code, -1);
        assert!(result.output.contains("denied"));
    }

    #[test]
    fn test_shell_server_tools() {
        let config = ShellServerConfig::new(None, 30, "/bin/sh".to_string(), None, None, false, false);
        let server = ShellServer::new(config);
        let tools = server.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "execute_command");
    }

    #[test]
    fn test_shell_server_initialize() {
        let config = ShellServerConfig::new(None, 30, "/bin/sh".to_string(), None, None, false, false);
        let server = ShellServer::new(config);
        let result = server.handle_initialize();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "mcpz-shell");
    }

    #[test]
    fn test_shell_server_call_tool() {
        let config = ShellServerConfig::new(None, 30, "/bin/sh".to_string(), None, None, false, false);
        let server = ShellServer::new(config);
        let result = server
            .call_tool("execute_command", &serde_json::json!({"command": "echo test"}))
            .unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("test"));
    }
}
