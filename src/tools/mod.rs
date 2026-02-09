mod bash;
mod read;
mod skill;
mod write;

pub use bash::BashTool;
pub use read::ReadTool;
pub use skill::SkillTool;
pub use write::WriteTool;

use crate::models::{FunctionSpec, Tool};
use anyhow::Result;

/// Registry of built-in tools
pub struct ToolRegistry {
    bash: BashTool,
    read: ReadTool,
    write: WriteTool,
    skill: SkillTool,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            bash: BashTool::new(),
            read: ReadTool::new(),
            write: WriteTool::new(),
            skill: SkillTool::new(),
        }
    }

    /// Get all tool specifications for the API
    pub fn get_tools(&self) -> Vec<Tool> {
        vec![
            self.bash.get_spec(),
            self.read.get_spec(),
            self.write.get_spec(),
            self.skill.get_spec(),
        ]
    }

    /// Check if a tool name is handled by this registry
    pub fn has_tool(&self, name: &str) -> bool {
        matches!(name, "Bash" | "Read" | "Write" | "Skill")
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<String> {
        match name {
            "Bash" => self.bash.execute(args).await,
            "Read" => self.read.execute(args),
            "Write" => self.write.execute(args),
            "Skill" => self.skill.execute(args).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a Tool spec
pub fn make_tool(name: &str, description: &str, parameters: serde_json::Value) -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.get_tools();
        assert_eq!(tools.len(), 4);

        let names: Vec<_> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(names.contains(&"Bash"));
        assert!(names.contains(&"Read"));
        assert!(names.contains(&"Write"));
        assert!(names.contains(&"Skill"));
    }

    #[test]
    fn test_has_tool() {
        let registry = ToolRegistry::new();
        assert!(registry.has_tool("Bash"));
        assert!(registry.has_tool("Read"));
        assert!(registry.has_tool("Write"));
        assert!(registry.has_tool("Skill"));
        assert!(!registry.has_tool("unknown"));
    }
}
