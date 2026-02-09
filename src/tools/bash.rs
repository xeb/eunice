use crate::models::Tool;
use crate::tools::make_tool;
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Bash tool for executing shell commands
pub struct BashTool {
    default_timeout: u64,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            default_timeout: 600, // 10 minutes
        }
    }

    pub fn get_spec(&self) -> Tool {
        make_tool(
            "Bash",
            "Execute a shell command and return the output. Commands run in the user's default shell with full system access.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 600)"
                    }
                },
                "required": ["command"]
            }),
        )
    }

    pub async fn execute(&self, args: serde_json::Value) -> Result<String> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        let timeout_secs = args["timeout"]
            .as_u64()
            .unwrap_or(self.default_timeout);

        // Detect shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        // Execute command
        let child = Command::new(&shell)
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn shell: {}", shell))?;

        // Wait with timeout
        let output = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
            .await
            .with_context(|| format!("Command timed out after {} seconds", timeout_secs))?
            .with_context(|| "Failed to execute command")?;

        // Combine stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str("[stderr]\n");
            result.push_str(&stderr);
        }

        // Add exit code if non-zero
        if !output.status.success() {
            let code = output.status.code().unwrap_or(-1);
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str(&format!("[exit code: {}]", code));
        }

        if result.is_empty() {
            result = "(no output)".to_string();
        }

        Ok(result)
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_spec() {
        let tool = BashTool::new();
        let spec = tool.get_spec();
        assert_eq!(spec.function.name, "Bash");
        assert!(spec.function.description.contains("shell"));
    }

    #[tokio::test]
    async fn test_bash_execute_echo() {
        let tool = BashTool::new();
        let args = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_execute_missing_command() {
        let tool = BashTool::new();
        let args = serde_json::json!({});
        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_execute_with_exit_code() {
        let tool = BashTool::new();
        let args = serde_json::json!({"command": "exit 42"});
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("exit code: 42"));
    }
}
