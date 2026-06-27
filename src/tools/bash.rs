use crate::models::Tool;
use crate::tools::make_tool;
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Kills a child's entire process group with SIGKILL when dropped, unless
/// disarmed. Combined with spawning the shell into its own process group, this
/// reaps the whole subprocess tree when the command finishes early — i.e. when
/// the user cancels (the execute future is dropped) or the command times out.
#[cfg(unix)]
struct ProcessGroupKiller {
    pgid: i32,
    armed: bool,
}

#[cfg(unix)]
impl Drop for ProcessGroupKiller {
    fn drop(&mut self) {
        if self.armed {
            // pgid == leader pid (set via process_group(0)); kill the whole group.
            unsafe {
                libc::killpg(self.pgid, libc::SIGKILL);
            }
        }
    }
}

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

        // Build the command. kill_on_drop reaps the shell leader if this future
        // is dropped (e.g. the user cancels); on unix we also put it in its own
        // process group so the whole subtree can be killed (see the guard below).
        let mut cmd = Command::new(&shell);
        cmd.arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn shell: {}", shell))?;

        // Arm a process-group killer for the whole subtree; disarmed on success.
        #[cfg(unix)]
        let mut group_killer = child.id().map(|pid| ProcessGroupKiller {
            pgid: pid as i32,
            armed: true,
        });

        // Wait with timeout
        let output = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
            .await
            .with_context(|| format!("Command timed out after {} seconds", timeout_secs))?
            .with_context(|| "Failed to execute command")?;

        // Completed normally — don't signal a (possibly recycled) process group.
        #[cfg(unix)]
        if let Some(g) = group_killer.as_mut() {
            g.armed = false;
        }

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

    // When the execute future is dropped (e.g. the user cancels with Escape),
    // the spawned command must be killed — its side effects must not happen.
    #[tokio::test]
    async fn test_bash_killed_when_future_dropped() {
        let tool = BashTool::new();
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        // Sleep, then create the marker. If the process is properly killed when
        // we drop the future, the marker is never created.
        let cmd = format!("sleep 1; touch '{}'", marker.display());

        {
            let fut = tool.execute(serde_json::json!({ "command": cmd }));
            tokio::pin!(fut);
            tokio::select! {
                _ = &mut fut => panic!("command should not have completed"),
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
            // `fut` (and its child process) is dropped at end of this scope.
        }

        // Wait well past the command's own sleep; the marker must NOT appear.
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert!(
            !marker.exists(),
            "subprocess kept running after the future was dropped"
        );
    }
}
