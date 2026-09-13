//! Give local Qwen an agent role in addition to the function schemas.
use crate::models::Tool;
use serde_json::{json, Value};

const AGENT_INSTRUCTIONS: &str = "You are Eunice, a terminal agent running on the user's computer. \
The supplied tools are connected to this computer and their calls execute through Eunice. \
Use them to inspect the environment and complete the user's task. \
For questions about the current directory, files, or command results, obtain the facts using tools \
instead of asking the user to run commands. When Bash is available, use Bash with pwd to check the working directory. \
Do not claim you lack filesystem access without trying an available tool. \
Report only results you actually observed, and follow the user's instructions and scope.";

/// Add transport-only instructions; never put them in visible or persisted history.
/// Existing caller instructions are retained after the general agent guidance.
pub fn prepare_messages(model: &str, messages: &mut Value, tools: Option<&[Tool]>) {
    if !model.to_ascii_lowercase().contains("qwen3.5") || !tools.is_some_and(|tools| !tools.is_empty()) {
        return;
    }
    let Some(rows) = messages.as_array_mut() else { return; };
    if let Some(first) = rows.first_mut().filter(|row| row["role"] == "system") {
        if let Some(content) = first["content"].as_str() {
            first["content"] = format!("{AGENT_INSTRUCTIONS}\n\n{content}").into();
            return;
        }
    }
    rows.insert(0, json!({"role":"system", "content":AGENT_INSTRUCTIONS}));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolRegistry;

    #[test]
    fn supplies_agent_role_without_altering_tool_results_or_user_scope() {
        let history = json!([
            {"role":"user","content":"Read only. What directory is this?"},
            {"role":"assistant","tool_calls":[{"id":"one","type":"function","function":{"name":"Bash","arguments":"{\"command\":\"pwd\"}"}}]},
            {"role":"tool","tool_call_id":"one","content":"/test/location"}
        ]);
        let mut request = history.clone();
        prepare_messages("Qwen3.5-2B-Q4_K_M", &mut request, Some(&ToolRegistry::new().get_tools()));
        assert_eq!(request[0]["role"], "system");
        assert!(request[0]["content"].as_str().unwrap().contains("follow the user's instructions and scope"));
        assert_eq!(&request.as_array().unwrap()[1..], history.as_array().unwrap());
    }

    #[test]
    fn preserves_existing_system_instructions_in_a_single_system_message() {
        let mut request = json!([{"role":"system","content":"Never modify files."},{"role":"user","content":"Where am I?"}]);
        prepare_messages("hf:qwen3.5:2b", &mut request, Some(&ToolRegistry::new().get_tools()));
        assert_eq!(request.as_array().unwrap().len(), 2);
        assert!(request[0]["content"].as_str().unwrap().ends_with("Never modify files."));
    }

    #[test]
    fn no_agent_claims_without_tools_and_no_changes_to_other_models() {
        let original = json!([{"role":"user","content":"Hello"}]);
        for (model, tools) in [("Qwen3.5-2B", None), ("Qwen3.5-2B", Some(vec![])), ("gemma4", Some(ToolRegistry::new().get_tools()))] {
            let mut request = original.clone();
            prepare_messages(model, &mut request, tools.as_deref());
            assert_eq!(request, original);
        }
    }
}
