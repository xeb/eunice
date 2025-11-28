use crate::client::Client;
use crate::display;
use crate::display::{Spinner, ThinkingSpinner};
use crate::mcp::McpManager;
use crate::models::{AgentConfig, McpConfig, Message, Tool, FunctionSpec};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::future::Future;

/// Manages multi-agent orchestration
pub struct AgentOrchestrator {
    agents: HashMap<String, AgentConfig>,
    resolved_prompts: HashMap<String, String>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator from config
    pub fn new(config: &McpConfig, config_dir: Option<&Path>) -> Result<Self> {
        let mut resolved_prompts = HashMap::new();

        // Resolve all agent prompts (inline or from files)
        for (name, agent) in &config.agents {
            let prompt = resolve_prompt(&agent.prompt, config_dir)?;
            resolved_prompts.insert(name.clone(), prompt);
        }

        Ok(Self {
            agents: config.agents.clone(),
            resolved_prompts,
        })
    }

    /// Check if multi-agent mode is enabled
    pub fn has_agents(&self) -> bool {
        !self.agents.is_empty()
    }

    /// Get list of agent names
    pub fn agent_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.agents.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get agent config by name
    pub fn get_agent(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.get(name)
    }

    /// Get resolved prompt for an agent
    #[allow(dead_code)]
    pub fn get_prompt(&self, name: &str) -> Option<&String> {
        self.resolved_prompts.get(name)
    }

    /// Get invoke tools for an agent (tools to call other agents)
    pub fn get_invoke_tools(&self, agent_name: &str) -> Vec<Tool> {
        let Some(agent) = self.agents.get(agent_name) else {
            return Vec::new();
        };

        agent.can_invoke.iter().filter_map(|target_name| {
            let target = self.agents.get(target_name)?;
            let description = format!(
                "Invoke the '{}' agent. {}",
                target_name,
                truncate_prompt(&self.resolved_prompts.get(target_name).unwrap_or(&target.prompt), 100)
            );

            Some(Tool {
                tool_type: "function".to_string(),
                function: FunctionSpec {
                    name: format!("invoke_{}", target_name),
                    description,
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "task": {
                                "type": "string",
                                "description": "The task to assign to this agent"
                            },
                            "context": {
                                "type": "string",
                                "description": "Optional additional context or information"
                            }
                        },
                        "required": ["task"]
                    }),
                },
            })
        }).collect()
    }

    /// Check if a tool name is an invoke tool
    pub fn is_invoke_tool(&self, tool_name: &str) -> bool {
        tool_name.starts_with("invoke_")
    }

    /// Get target agent name from invoke tool name
    pub fn get_invoke_target<'a>(&self, tool_name: &'a str) -> Option<&'a str> {
        tool_name.strip_prefix("invoke_")
    }

    /// Run an agent with a task (returns boxed future to enable recursion)
    pub fn run_agent<'a>(
        &'a self,
        client: &'a Client,
        model: &'a str,
        agent_name: &'a str,
        task: &'a str,
        context: Option<&'a str>,
        mcp_manager: &'a mut McpManager,
        tool_output_limit: usize,
        silent: bool,
        verbose: bool,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let agent = self.agents.get(agent_name)
                .ok_or_else(|| anyhow!("Unknown agent: {}", agent_name))?;

            let system_prompt = self.resolved_prompts.get(agent_name)
                .ok_or_else(|| anyhow!("No prompt for agent: {}", agent_name))?;

            // Build the full prompt
            let full_prompt = if let Some(ctx) = context {
                format!(
                    "{}\n\n# TASK\n{}\n\n# CONTEXT\n{}",
                    system_prompt, task, ctx
                )
            } else {
                format!("{}\n\n# TASK\n{}", system_prompt, task)
            };

            // Display agent invocation
            if !silent {
                let indent = "  ".repeat(depth);
                eprintln!("{}🤖 Agent '{}' starting task: {}", indent, agent_name, truncate_prompt(task, 60));
            }

            // Get tools for this agent
            let mut tools = self.get_filtered_tools(agent, mcp_manager);
            tools.extend(self.get_invoke_tools(agent_name));

            // Run the agent loop
            let mut conversation_history: Vec<Message> = Vec::new();
            let result = self.agent_loop(
                client,
                model,
                &full_prompt,
                &tools,
                agent_name,
                mcp_manager,
                tool_output_limit,
                silent,
                verbose,
                &mut conversation_history,
                depth,
            ).await?;

            if !silent {
                let indent = "  ".repeat(depth);
                eprintln!("{}✅ Agent '{}' completed", indent, agent_name);
            }

            Ok(result)
        })
    }

    /// Get MCP tools filtered by agent's allowed tools
    fn get_filtered_tools(&self, agent: &AgentConfig, mcp_manager: &McpManager) -> Vec<Tool> {
        if agent.tools.is_empty() {
            return Vec::new();
        }

        let all_tools = mcp_manager.get_tools();
        all_tools.into_iter().filter(|tool| {
            // Check if tool belongs to any allowed server
            agent.tools.iter().any(|allowed| {
                let prefix = format!("{}_", allowed);
                tool.function.name.starts_with(&prefix)
            })
        }).collect()
    }

    /// The agent loop - processes messages and tool calls
    async fn agent_loop(
        &self,
        client: &Client,
        model: &str,
        prompt: &str,
        tools: &[Tool],
        agent_name: &str,
        mcp_manager: &mut McpManager,
        tool_output_limit: usize,
        silent: bool,
        verbose: bool,
        conversation_history: &mut Vec<Message>,
        depth: usize,
    ) -> Result<String> {
        let indent = "  ".repeat(depth);

        // Add user message
        conversation_history.push(Message::User {
            content: prompt.to_string(),
        });

        let tools_option = if tools.is_empty() { None } else { Some(tools) };
        let mut final_response = String::new();

        loop {
            display::debug(&format!("{}Agent '{}' calling LLM...", indent, agent_name), verbose);

            // Start thinking spinner
            let thinking_spinner = if !silent {
                Some(ThinkingSpinner::start())
            } else {
                None
            };

            // Call the LLM
            let response = client
                .chat_completion(model, conversation_history, tools_option, false)
                .await;

            // Stop thinking spinner
            if let Some(spinner) = thinking_spinner {
                spinner.stop();
            }

            let response = response?;
            let choice = &response.choices[0];

            // Add assistant response to history
            conversation_history.push(Message::Assistant {
                content: choice.message.content.clone(),
                tool_calls: choice.message.tool_calls.clone(),
            });

            // Capture content
            if let Some(content) = &choice.message.content {
                if !content.is_empty() {
                    final_response = content.clone();
                    if !silent {
                        println!("{}{}", indent, content);
                    }
                }
            }

            // Check for tool calls
            let Some(tool_calls) = &choice.message.tool_calls else {
                break;
            };

            if tool_calls.is_empty() {
                break;
            }

            // Execute tool calls
            for tool_call in tool_calls {
                let tool_name = &tool_call.function.name;
                let arguments = &tool_call.function.arguments;

                if !silent {
                    eprintln!("{}🔧 {}", indent, tool_name);
                }

                let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

                let result = if self.is_invoke_tool(tool_name) {
                    // Handle agent invocation
                    self.handle_invoke(
                        client, model, tool_name, &args, mcp_manager,
                        tool_output_limit, silent, verbose, depth,
                    ).await
                } else {
                    // Regular MCP tool
                    let spinner = if !silent {
                        Some(Spinner::start(&format!("Running {}", tool_name)))
                    } else {
                        None
                    };

                    let result = mcp_manager.execute_tool(tool_name, args).await
                        .unwrap_or_else(|e| format!("Error: {}", e));

                    if let Some(spinner) = spinner {
                        spinner.stop().await;
                    }

                    result
                };

                if !silent && !self.is_invoke_tool(tool_name) {
                    display::print_tool_result(&result, tool_output_limit);
                }

                conversation_history.push(Message::Tool {
                    tool_call_id: tool_call.id.clone(),
                    content: result,
                });
            }
        }

        Ok(final_response)
    }

    /// Handle an invoke_* tool call
    async fn handle_invoke(
        &self,
        client: &Client,
        model: &str,
        tool_name: &str,
        args: &serde_json::Value,
        mcp_manager: &mut McpManager,
        tool_output_limit: usize,
        silent: bool,
        verbose: bool,
        depth: usize,
    ) -> String {
        let Some(target_agent) = self.get_invoke_target(tool_name) else {
            return format!("Error: Invalid invoke tool name: {}", tool_name);
        };

        let task = args.get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("No task specified");

        let context = args.get("context")
            .and_then(|v| v.as_str());

        match self.run_agent(
            client, model, target_agent, task, context,
            mcp_manager, tool_output_limit, silent, verbose, depth + 1,
        ).await {
            Ok(result) => result,
            Err(e) => format!("Agent '{}' failed: {}", target_agent, e),
        }
    }
}

/// Resolve a prompt - either return as-is if not a file, or read from file
fn resolve_prompt(prompt: &str, config_dir: Option<&Path>) -> Result<String> {
    // Check if it looks like a file path
    if prompt.ends_with(".md") || prompt.ends_with(".txt") || prompt.contains('/') {
        let path = if let Some(dir) = config_dir {
            dir.join(prompt)
        } else {
            Path::new(prompt).to_path_buf()
        };

        if path.exists() {
            return std::fs::read_to_string(&path)
                .map_err(|e| anyhow!("Failed to read prompt file '{}': {}", path.display(), e));
        }
    }

    // Return as inline prompt
    Ok(prompt.to_string())
}

/// Truncate a string for display
fn truncate_prompt(s: &str, max_len: usize) -> String {
    let s = s.lines().next().unwrap_or(s); // First line only
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_orchestrator() {
        let config = McpConfig::default();
        let orch = AgentOrchestrator::new(&config, None).unwrap();
        assert!(!orch.has_agents());
        assert!(orch.agent_names().is_empty());
    }

    #[test]
    fn test_orchestrator_with_agents() {
        let mut config = McpConfig::default();
        config.agents.insert("root".to_string(), AgentConfig {
            prompt: "You are the root agent".to_string(),
            tools: vec![],
            can_invoke: vec!["worker".to_string()],
        });
        config.agents.insert("worker".to_string(), AgentConfig {
            prompt: "You are a worker agent".to_string(),
            tools: vec!["shell".to_string()],
            can_invoke: vec![],
        });

        let orch = AgentOrchestrator::new(&config, None).unwrap();
        assert!(orch.has_agents());
        assert_eq!(orch.agent_names(), vec!["root", "worker"]);
    }

    #[test]
    fn test_get_invoke_tools() {
        let mut config = McpConfig::default();
        config.agents.insert("root".to_string(), AgentConfig {
            prompt: "Root agent".to_string(),
            tools: vec![],
            can_invoke: vec!["dev".to_string()],
        });
        config.agents.insert("dev".to_string(), AgentConfig {
            prompt: "Developer agent".to_string(),
            tools: vec![],
            can_invoke: vec![],
        });

        let orch = AgentOrchestrator::new(&config, None).unwrap();
        let tools = orch.get_invoke_tools("root");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "invoke_dev");
    }

    #[test]
    fn test_is_invoke_tool() {
        let config = McpConfig::default();
        let orch = AgentOrchestrator::new(&config, None).unwrap();
        assert!(orch.is_invoke_tool("invoke_worker"));
        assert!(!orch.is_invoke_tool("shell_execute"));
    }

    #[test]
    fn test_get_invoke_target() {
        let config = McpConfig::default();
        let orch = AgentOrchestrator::new(&config, None).unwrap();
        assert_eq!(orch.get_invoke_target("invoke_worker"), Some("worker"));
        assert_eq!(orch.get_invoke_target("shell_exec"), None);
    }

    #[test]
    fn test_truncate_prompt() {
        assert_eq!(truncate_prompt("short", 10), "short");
        assert_eq!(truncate_prompt("this is a long string", 10), "this is...");
        assert_eq!(truncate_prompt("line1\nline2", 100), "line1");
    }

    #[test]
    fn test_resolve_prompt_inline() {
        let result = resolve_prompt("You are an agent", None).unwrap();
        assert_eq!(result, "You are an agent");
    }
}
