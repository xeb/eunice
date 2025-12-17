use crate::models::{AgentConfig, McpConfig, McpServerConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Check if mcpz is installed
pub fn has_mcpz() -> bool {
    std::process::Command::new("mcpz").arg("--version").output().is_ok()
}

/// Check if GEMINI_API_KEY is set
pub fn has_gemini_api_key() -> bool {
    std::env::var("GEMINI_API_KEY").is_ok()
}

/// Embedded DMN (Default Mode Network) MCP configuration
/// Minimal set: shell + filesystem (interpret_image is built-in)
pub fn get_dmn_mcp_config() -> McpConfig {
    let mut servers = HashMap::new();
    let use_mcpz = has_mcpz();

    servers.insert(
        "shell".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "shell".into()], url: None, timeout: None }
        } else {
            McpServerConfig { command: "uvx".into(), args: vec!["git+https://github.com/emsi/mcp-server-shell".into()], url: None, timeout: None }
        },
    );

    servers.insert(
        "filesystem".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "filesystem".into()], url: None, timeout: None }
        } else {
            McpServerConfig { command: "npx".into(), args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), ".".into()], url: None, timeout: None }
        },
    );

    // Browser automation (optional - only if mcpz is available)
    if use_mcpz {
        servers.insert(
            "browser".to_string(),
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "browser".into()], url: None, timeout: None },
        );
    }

    McpConfig {
        mcp_servers: servers,
        agents: HashMap::new(),
        allowed_tools: Vec::new(),
        denied_tools: Vec::new(),
        webapp: None,
    }
}

/// Embedded Research Agent configuration
/// Multi-agent system: root -> researcher, report_writer, evaluator
pub fn get_research_mcp_config() -> McpConfig {
    let mut servers = HashMap::new();
    let use_mcpz = has_mcpz();

    // Research mode only needs filesystem (search_query is built-in)
    servers.insert(
        "filesystem".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "filesystem".into()], url: None, timeout: None }
        } else {
            McpServerConfig { command: "npx".into(), args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), ".".into()], url: None, timeout: None }
        },
    );

    // Browser automation (optional - only if mcpz is available)
    if use_mcpz {
        servers.insert(
            "browser".to_string(),
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "browser".into()], url: None, timeout: None },
        );
    }

    let mut agents = HashMap::new();

    // Root agent (lead coordinator)
    agents.insert("root".to_string(), AgentConfig {
        prompt: RESEARCH_LEAD_PROMPT.to_string(),
        mcp_servers: vec![],
        tools: vec![],
        can_invoke: vec!["researcher".to_string(), "report_writer".to_string(), "evaluator".to_string()],
    });

    // Researcher agent
    agents.insert("researcher".to_string(), AgentConfig {
        prompt: RESEARCH_RESEARCHER_PROMPT.to_string(),
        mcp_servers: vec![],
        tools: vec!["filesystem_write_*".to_string(), "search_query".to_string(), "browser_*".to_string()],
        can_invoke: vec![],
    });

    // Report writer agent
    agents.insert("report_writer".to_string(), AgentConfig {
        prompt: RESEARCH_REPORT_WRITER_PROMPT.to_string(),
        mcp_servers: vec![],
        tools: vec!["filesystem_*".to_string()],
        can_invoke: vec![],
    });

    // Evaluator agent
    agents.insert("evaluator".to_string(), AgentConfig {
        prompt: RESEARCH_EVALUATOR_PROMPT.to_string(),
        mcp_servers: vec![],
        tools: vec!["filesystem_read_*".to_string(), "filesystem_list_*".to_string()],
        can_invoke: vec![],
    });

    McpConfig {
        mcp_servers: servers,
        agents,
        allowed_tools: Vec::new(),
        denied_tools: Vec::new(),
        webapp: None,
    }
}

/// Load MCP configuration from a JSON or TOML file
pub fn load_mcp_config(path: &Path) -> Result<McpConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Determine format based on file extension
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "toml" => toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config file: {}", path.display())),
        _ => serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON config file: {}", path.display())),
    }
}

/// DMN (Default Mode Network) instructions loaded from external file
pub const DMN_INSTRUCTIONS: &str = include_str!("../dmn_instructions.md");

/// LLM context files embedded for --llms-txt and --llms-full-txt flags
pub const LLMS_TXT: &str = include_str!("../llms.txt");
pub const LLMS_FULL_TXT: &str = include_str!("../llms-full.txt");

// === Research Agent Prompts (embedded) ===

/// Research lead agent prompt
pub const RESEARCH_LEAD_PROMPT: &str = r#"You are a lead research coordinator who orchestrates multi-agent research projects.

## Rules

1. Delegate ALL research and report writing to subagents. Never research or write reports yourself.
2. Keep responses SHORT - 2-3 sentences max. No greetings, no emojis.
3. Get to work immediately.

## Role

- Break research requests into 2-4 distinct subtopics
- Spawn researcher subagents to investigate each subtopic
- After research, spawn report-writer to synthesize findings
- Have evaluator review the report
- If evaluator says NEEDS_REVISION, have report-writer revise ONCE
- Your tools are invoke_researcher, invoke_report_writer, and invoke_evaluator

## Workflow

**STEP 1: ANALYZE** - Understand the research topic, identify 2-4 distinct subtopics

**STEP 2: SPAWN RESEARCHERS** - Use invoke_researcher for each subtopic

**STEP 3: SPAWN REPORT-WRITER** - After all research completes, invoke report-writer

**STEP 4: EVALUATE** - Invoke evaluator to review the report

**STEP 5: REVISE IF NEEDED (ONE TIME ONLY)** - If evaluator says NEEDS_REVISION, invoke report-writer again

**STEP 6: CONFIRM** - Tell user where to find the report"#;

/// Research researcher agent prompt
pub const RESEARCH_RESEARCHER_PROMPT: &str = r#"You are a research specialist focused on information gathering.

## Critical Rules

1. Use search_query for ALL research - never rely on your own knowledge
2. ALWAYS use model="pro_preview" for search_query (best quality)
3. Save CONCISE research summaries (3-4 paragraphs max) to research_notes/
4. You do NOT write formal reports - save brief notes for the report-writer

## Tools

- search_query: Web search using Gemini with Google Search grounding. ALWAYS use model="pro_preview"
- filesystem_write_file: Save findings to research_notes/{topic}.md
- browser_* (optional): Browser automation for JavaScript-heavy pages

## Browser Tools (Optional)

Browser tools are optional and may not be available. Only use if:
- A page requires JavaScript to render content
- You need a screenshot of a web page

**Usage:**
1. Call `browser_is_available` first to check if Chrome is installed
2. If `is_available` returns false, **do not use any browser tools** - stick to search_query
3. Call `browser_start_browser` to launch Chrome
4. Use `browser_open_url` to navigate, `browser_get_page_as_markdown` for content
5. Always call `browser_stop_browser` when done

## Strategy

1. Use search_query 2-4 times with different angles (always model="pro_preview")
2. Extract key findings including specific names, prices, features
3. Save to research_notes/{descriptive_topic_name}.md
4. Return brief confirmation

## Output Format (save to research_notes/)

```markdown
# {Topic}

{2-3 sentences summarizing key findings}

## Key Details

- [Item]: [Price/Details] - [Features]

## Sources

- [Source name with URL]
```"#;

/// Research report writer agent prompt
pub const RESEARCH_REPORT_WRITER_PROMPT: &str = r#"You are a professional report writer who creates clear, concise research summaries.

## Critical Rules

1. Read research notes from research_notes/ folder
2. Synthesize into a one-page summary
3. Do NOT conduct research - only read and write

## Tools

- filesystem_list_directory: Find files in research_notes/
- filesystem_read_file: Read research notes
- filesystem_write_file: Create report in reports/

## Workflow

1. List files in research_notes/
2. Read each research note
3. Synthesize into cohesive report
4. Save to reports/{topic}_summary.md

## Report Structure

```markdown
# {Topic} Research Summary

## Overview
{2-3 paragraph executive summary}

## Key Findings

### {Subtopic}
{Key points with citations}

## Conclusion
{Summary and implications}

## Sources
{List all sources cited}
```"#;

/// Research evaluator agent prompt
pub const RESEARCH_EVALUATOR_PROMPT: &str = r#"You are a research quality evaluator. Review reports and provide actionable feedback.

## Role

- Read the report from reports/
- Evaluate completeness, accuracy, and usefulness
- Return a verdict: APPROVED or NEEDS_REVISION

## Tools

- filesystem_read_file: Read the report
- filesystem_list_directory: Find report files

## Evaluation Criteria

1. **Completeness**: Does it cover all key aspects?
2. **Specificity**: Concrete names, prices, features?
3. **Sources**: Are claims backed by citations?
4. **Usefulness**: Is it actionable?

## Output Format

```
VERDICT: [APPROVED or NEEDS_REVISION]

STRENGTHS:
- [What the report does well]

ISSUES (if any):
- [Specific problem]

REVISION INSTRUCTIONS (if NEEDS_REVISION):
[Specific instructions]
```

## Rules

- Be concise - max 10 lines
- Only NEEDS_REVISION for significant gaps
- One revision cycle is enough"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dmn_mcp_config() {
        let config = get_dmn_mcp_config();
        assert!(config.mcp_servers.contains_key("shell"));
        assert!(config.mcp_servers.contains_key("filesystem"));
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_get_research_mcp_config() {
        let config = get_research_mcp_config();
        // Should have filesystem server
        assert!(config.mcp_servers.contains_key("filesystem"));
        // Should NOT have shell (research doesn't need it)
        assert!(!config.mcp_servers.contains_key("shell"));

        // Should have 4 agents
        assert_eq!(config.agents.len(), 4);
        assert!(config.agents.contains_key("root"));
        assert!(config.agents.contains_key("researcher"));
        assert!(config.agents.contains_key("report_writer"));
        assert!(config.agents.contains_key("evaluator"));
    }

    #[test]
    fn test_research_root_agent_config() {
        let config = get_research_mcp_config();
        let root = config.agents.get("root").unwrap();

        // Root has no tools, only invokes subagents
        assert!(root.tools.is_empty());
        assert_eq!(root.can_invoke.len(), 3);
        assert!(root.can_invoke.contains(&"researcher".to_string()));
        assert!(root.can_invoke.contains(&"report_writer".to_string()));
        assert!(root.can_invoke.contains(&"evaluator".to_string()));
    }

    #[test]
    fn test_research_researcher_agent_config() {
        let config = get_research_mcp_config();
        let researcher = config.agents.get("researcher").unwrap();

        // Researcher has filesystem_write_* and search_query
        assert!(researcher.tools.contains(&"filesystem_write_*".to_string()));
        assert!(researcher.tools.contains(&"search_query".to_string()));
        assert!(researcher.can_invoke.is_empty());
    }

    #[test]
    fn test_research_evaluator_agent_config() {
        let config = get_research_mcp_config();
        let evaluator = config.agents.get("evaluator").unwrap();

        // Evaluator has read-only filesystem access
        assert!(evaluator.tools.contains(&"filesystem_read_*".to_string()));
        assert!(evaluator.tools.contains(&"filesystem_list_*".to_string()));
        assert!(evaluator.can_invoke.is_empty());
    }

    #[test]
    fn test_research_prompts_not_empty() {
        assert!(!RESEARCH_LEAD_PROMPT.is_empty());
        assert!(!RESEARCH_RESEARCHER_PROMPT.is_empty());
        assert!(!RESEARCH_REPORT_WRITER_PROMPT.is_empty());
        assert!(!RESEARCH_EVALUATOR_PROMPT.is_empty());

        // Check for key content
        assert!(RESEARCH_LEAD_PROMPT.contains("invoke_researcher"));
        assert!(RESEARCH_RESEARCHER_PROMPT.contains("pro_preview"));
        assert!(RESEARCH_REPORT_WRITER_PROMPT.contains("research_notes"));
        assert!(RESEARCH_EVALUATOR_PROMPT.contains("APPROVED"));
    }
}
