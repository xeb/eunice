use crate::client::Client;
use crate::config::has_mcpz;
use crate::models::Message;
use crate::provider::detect_provider;
use anyhow::{anyhow, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

/// Available MCP servers from mcpz
const MCPZ_SERVERS: &[(&str, &str)] = &[
    ("filesystem", "File operations (read, write, list, search)"),
    ("shell", "Execute shell commands"),
    ("browser", "Browser automation via Chrome DevTools Protocol"),
    ("sql", "SQL database operations"),
];

/// Built-in tools
const BUILTIN_TOOLS: &[(&str, &str)] = &[
    ("interpret_image", "Analyze images and PDFs"),
    ("search_query", "Web search using Gemini with Google Search"),
];

/// Agent definition collected from user
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
}

/// Read a line from stdin with a prompt
fn read_line(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Read a line with a default value shown in parentheses
fn read_line_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{} {}: ", prompt, format!("({})", default).dimmed());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

/// Display a numbered list and get selection(s)
fn select_from_list(items: &[(&str, &str)], multi: bool) -> Result<Vec<String>> {
    println!();
    println!("  {}  {}", "0.".dimmed(), "ALL".bold());
    for (i, (name, desc)) in items.iter().enumerate() {
        println!("  {}  {} - {}", format!("{}.", i + 1).dimmed(), name.cyan(), desc);
    }
    println!();

    let prompt = if multi {
        "Select tools (comma-separated numbers, 0 for ALL, Enter for none): "
    } else {
        "Select (number): "
    };

    loop {
        let input = read_line(prompt)?;

        // Empty input = no tools selected
        if input.is_empty() {
            return Ok(Vec::new());
        }

        // Parse selection
        let mut selected = Vec::new();
        let mut has_error = false;
        for part in input.split(',') {
            let part = part.trim();
            if let Ok(num) = part.parse::<usize>() {
                if num == 0 {
                    // ALL selected
                    return Ok(items.iter().map(|(name, _)| name.to_string()).collect());
                } else if num > 0 && num <= items.len() {
                    selected.push(items[num - 1].0.to_string());
                } else {
                    println!("{}", format!("Invalid selection: {}", num).red());
                    has_error = true;
                    break;
                }
            } else {
                println!("{}", format!("Invalid input: {}", part).red());
                has_error = true;
                break;
            }
        }

        if !has_error {
            return Ok(selected);
        }
    }
}

/// Collect agent definitions interactively
fn collect_agents() -> Result<(Vec<AgentDefinition>, Option<String>)> {
    let mut agents = Vec::new();
    let mcpz_available = has_mcpz();

    println!();
    println!("{}", "=== Create Agent Configuration ===".bold().cyan());
    println!();

    loop {
        // 1. Agent name
        println!("{}", "Step 1: Agent Name".bold());
        println!("Keep it short like 'researcher', 'chef', 'coder'");
        let name = loop {
            let input = read_line("Agent name: ")?;
            if input.is_empty() {
                println!("{}", "Name cannot be empty".red());
                continue;
            }
            if input.contains(' ') || input.contains('.') {
                println!("{}", "Name should be a single word without spaces or dots".red());
                continue;
            }
            // Check for duplicate names
            let lower_name = input.to_lowercase();
            if agents.iter().any(|a: &AgentDefinition| a.name == lower_name) {
                println!("{}", format!("Agent '{}' already defined", lower_name).red());
                continue;
            }
            break lower_name;
        };
        println!();

        // 2. Agent description/purpose
        println!("{}", "Step 2: Agent Purpose".bold());
        println!("Describe what you want the agent to do");
        let description = loop {
            let input = read_line("What should this agent do? ")?;
            if input.is_empty() {
                println!("{}", "Description cannot be empty".red());
                continue;
            }
            break input;
        };
        println!();

        // 3. Tool selection
        println!("{}", "Step 3: Tool Selection".bold());

        let mut all_tools: Vec<(&str, &str)> = Vec::new();

        // Add built-in tools
        println!("{}", "Built-in tools:".dimmed());
        all_tools.extend_from_slice(BUILTIN_TOOLS);

        // Add mcpz servers if available
        if mcpz_available {
            println!("{}", "MCP servers (via mcpz):".dimmed());
            all_tools.extend_from_slice(MCPZ_SERVERS);
        } else {
            println!("{}", "Note: mcpz not installed, only built-in tools available".yellow());
        }

        let selected_tools = select_from_list(&all_tools, true)?;
        println!();

        agents.push(AgentDefinition {
            name,
            description,
            tools: selected_tools,
        });

        // Ask if user wants to add another agent
        let add_more = read_line("Add another agent? (y/N): ")?;
        if !add_more.to_lowercase().starts_with('y') {
            break;
        }
        println!();
    }

    // If multiple agents, ask about communication
    let communication_description = if agents.len() > 1 {
        println!();
        println!("{}", "Step 4: Agent Communication".bold());
        println!("You have {} agents: {}", agents.len(),
            agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "));
        println!();
        println!("How should these agents communicate with each other?");
        println!("Examples:");
        println!("  - \"coordinator delegates to all others\"");
        println!("  - \"researcher reports to coordinator, coder works independently\"");
        println!("  - \"no communication, they work independently\"");
        println!();
        let comm = read_line("Communication pattern: ")?;
        if comm.is_empty() {
            None
        } else {
            Some(comm)
        }
    } else {
        None
    };

    Ok((agents, communication_description))
}

/// Generate the few-shot prompt for the model
fn generate_prompt(agents: &[AgentDefinition], communication: Option<&str>) -> String {
    let mut prompt = String::new();

    prompt.push_str(r#"You are an expert at creating eunice.toml configuration files. Generate a valid TOML configuration based on the user's agent definitions.

## TOML Format

The eunice.toml file has two main sections:
1. `[mcpServers.*]` - MCP server configurations
2. `[agents.*]` - Agent configurations

## Examples

### Example 1: Single researcher agent
```toml
[mcpServers.filesystem]
command = "mcpz"
args = ["server", "filesystem"]

[mcpServers.shell]
command = "mcpz"
args = ["server", "shell"]

[agents.researcher]
prompt = """You are a research assistant.
Your job is to find information and organize it clearly.
Use shell commands to search and filesystem to save notes."""
tools = ["filesystem_*", "shell_*", "search_query"]
can_invoke = []
```

### Example 2: Multi-agent system with coordinator
```toml
[mcpServers.filesystem]
command = "mcpz"
args = ["server", "filesystem"]

[mcpServers.shell]
command = "mcpz"
args = ["server", "shell"]

[mcpServers.browser]
command = "mcpz"
args = ["server", "browser"]

[agents.coordinator]
prompt = """You are a project coordinator.
Break down tasks and delegate to specialized agents.
Do not do work yourself - delegate to workers."""
tools = []
can_invoke = ["coder", "researcher"]

[agents.coder]
prompt = """You are a skilled programmer.
Write clean, well-documented code.
Test your changes before considering them complete."""
tools = ["filesystem_*", "shell_*"]
can_invoke = []

[agents.researcher]
prompt = """You are a research specialist.
Search the web and browse pages to gather information.
Save findings to files for other agents to use."""
tools = ["filesystem_write_*", "browser_*", "search_query"]
can_invoke = []
```

### Example 3: Chef agent with limited tools
```toml
[mcpServers.filesystem]
command = "mcpz"
args = ["server", "filesystem"]

[agents.chef]
prompt = """You are a culinary expert and recipe creator.
Help users with cooking questions, create recipes, and provide meal planning.
Save recipes to markdown files in a recipes/ directory."""
tools = ["filesystem_*", "search_query"]
can_invoke = []
```

## Rules

1. Only include MCP servers that are actually needed by at least one agent
2. Use wildcard patterns like `filesystem_*` instead of listing individual tools
3. Built-in tools (interpret_image, search_query) don't need MCP server config
4. Write clear, helpful prompts that explain the agent's role and capabilities
5. The prompt should be multi-line and detailed (use triple quotes)
6. Always use `command = "mcpz"` and `args = ["server", "<name>"]` format
7. The `can_invoke` array lists OTHER agent names this agent can call
8. If there's a coordinator/orchestrator, it typically has `tools = []` and invokes others

## User's Agent Definitions

"#);

    for (i, agent) in agents.iter().enumerate() {
        prompt.push_str(&format!("\n### Agent {} ###\n", i + 1));
        prompt.push_str(&format!("Name: {}\n", agent.name));
        prompt.push_str(&format!("Purpose: {}\n", agent.description));
        prompt.push_str(&format!("Tools: {}\n", agent.tools.join(", ")));
    }

    if let Some(comm) = communication {
        prompt.push_str(&format!("\n### Agent Communication Pattern ###\n"));
        prompt.push_str(&format!("{}\n", comm));
    }

    prompt.push_str(r#"

## Task

Generate a complete eunice.toml file based on the above agent definitions. Include:
1. All necessary MCP server configurations
2. Agent configurations with detailed prompts
3. Appropriate tool patterns for each agent
4. Set up `can_invoke` arrays based on the communication pattern described above

Output ONLY the TOML content, no explanations or markdown code fences.
"#);

    prompt
}

/// Extract TOML from model response (handles markdown code blocks)
fn extract_toml(response: &str) -> String {
    let response = response.trim();

    // Check if wrapped in markdown code block
    if response.starts_with("```") {
        let lines: Vec<&str> = response.lines().collect();
        let start = if lines.first().map(|l| l.starts_with("```")).unwrap_or(false) { 1 } else { 0 };
        let end = if lines.last().map(|l| *l == "```").unwrap_or(false) { lines.len() - 1 } else { lines.len() };
        return lines[start..end].join("\n");
    }

    response.to_string()
}

/// Prompt for output filename with proper handling
fn get_output_filename() -> Result<String> {
    let default_path = Path::new("eunice.toml");

    if !default_path.exists() {
        // Default doesn't exist, offer it as default
        let filename = read_line_with_default("File name", "eunice.toml")?;
        let filename = if !filename.ends_with(".toml") {
            format!("{}.toml", filename)
        } else {
            filename
        };

        // Check if the entered filename exists
        if Path::new(&filename).exists() && filename != "eunice.toml" {
            let confirm = read_line(&format!("{} already exists. Overwrite? (y/N): ", filename))?;
            if !confirm.to_lowercase().starts_with('y') {
                // Recursively ask again
                return get_output_filename();
            }
        }

        Ok(filename)
    } else {
        // eunice.toml exists, force user to enter something
        println!("{}", "eunice.toml already exists.".yellow());
        loop {
            let filename = read_line("File name (required): ")?;
            if filename.is_empty() {
                println!("{}", "Please enter a filename".red());
                continue;
            }

            let filename = if !filename.ends_with(".toml") {
                format!("{}.toml", filename)
            } else {
                filename
            };

            // Check if this file exists
            if Path::new(&filename).exists() {
                let confirm = read_line(&format!("{} already exists. Overwrite? (y/N): ", filename))?;
                if !confirm.to_lowercase().starts_with('y') {
                    continue;
                }
            }

            return Ok(filename);
        }
    }
}

/// Create an animated spinner
fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

/// Run the create-agent interactive flow
pub async fn run_create_agent(model: &str, verbose: bool) -> Result<()> {
    // Collect agent definitions (including communication pattern)
    let (agents, communication) = collect_agents()?;
    if agents.is_empty() {
        println!("{}", "No agents defined. Exiting.".yellow());
        return Ok(());
    }

    // Generate prompt
    let prompt = generate_prompt(&agents, communication.as_deref());

    if verbose {
        println!("\n{}", "=== Prompt ===".dimmed());
        println!("{}", prompt.dimmed());
        println!("{}", "==============".dimmed());
    }

    // Call the model with spinner
    println!();
    let spinner = create_spinner("Generating configuration...");

    let provider_info = detect_provider(model)?;
    let client = Client::new(&provider_info, verbose)?;

    let messages = vec![Message::User {
        content: prompt,
    }];
    let messages_json = serde_json::to_value(&messages)?;

    let response = client.chat_completion(model, messages_json, None, false).await?;

    spinner.finish_and_clear();

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .ok_or_else(|| anyhow!("No response from model"))?;

    // Extract TOML content
    let toml_content = extract_toml(content);

    // Validate TOML
    if let Err(e) = toml::from_str::<toml::Value>(&toml_content) {
        println!("{}", format!("Warning: Generated TOML may have errors: {}", e).yellow());
    }

    // Show the generated config
    println!();
    println!("{}", "=== Generated Configuration ===".bold().green());
    println!();
    println!("{}", toml_content);
    println!();
    println!("{}", "================================".bold().green());
    println!();

    // Get output filename
    let output_path = get_output_filename()?;

    // Save the file
    std::fs::write(&output_path, &toml_content)?;
    println!();
    println!("{}", format!("✓ Configuration saved to {}", output_path).green().bold());
    println!();
    println!("To use your new agent configuration:");
    println!("  {}", format!("eunice --config {} \"your prompt\"", output_path).cyan());
    println!();

    Ok(())
}
