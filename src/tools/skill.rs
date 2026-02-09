use crate::models::Tool;
use crate::skills;
use crate::tools::make_tool;
use anyhow::Result;

/// Skill tool for discovering skills from ~/.eunice/skills/
pub struct SkillTool;

impl SkillTool {
    pub fn new() -> Self {
        Self
    }

    pub fn get_spec(&self) -> Tool {
        make_tool(
            "Skill",
            "Search for skills in ~/.eunice/skills/ that can help with a task. Returns matching skill directories with their descriptions from SKILL.md. After finding a skill, use the Read tool to read the full SKILL.md and the Bash tool to execute any helper scripts. Call with no query to list all available skills.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Description of the capability or task you need help with. If omitted, lists all available skills."
                    }
                },
                "required": []
            }),
        )
    }

    pub async fn execute(&self, args: serde_json::Value) -> Result<String> {
        match args["query"].as_str() {
            Some(query) => skills::discover_skills(query).await,
            None => skills::list_all_skills().await,
        }
    }
}

impl Default for SkillTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_tool_spec() {
        let tool = SkillTool::new();
        let spec = tool.get_spec();
        assert_eq!(spec.function.name, "Skill");
        assert!(spec.function.description.contains("skills"));
        assert!(spec.function.description.contains("~/.eunice/skills/"));
    }

    #[tokio::test]
    async fn test_skill_no_query_lists_all() {
        let tool = SkillTool::new();
        let args = serde_json::json!({});
        let result = tool.execute(args).await;
        // Should succeed and list skills (or say none found)
        assert!(result.is_ok());
    }
}
