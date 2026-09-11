//! In-session model changes. Replacing the client never replaces conversation,
//! instructions, tool outputs, or usage history owned by the REPL.
use crate::{
    client::Client,
    models::{Provider, ProviderInfo},
    provider::{detect_provider, get_available_models},
};
use anyhow::{anyhow, Result};
use serde::Serialize;

/// Offer only effort controls implemented by this provider's request adapter.
pub fn efforts(provider: &Provider, model: &str) -> Vec<String> {
    let mut levels = vec!["default"];
    match provider {
        Provider::Gemini if model.starts_with("gemini-3") => {
            levels.push("low");
            if model.contains("flash") {
                levels.push("medium");
            }
            levels.push("high");
        }
        Provider::OpenAI if crate::openai::is_astra(model) => { levels.extend(["low", "medium", "high", "xhigh", "max"]); }
        Provider::OpenAI | Provider::AzureOpenAI
            if model.starts_with("gpt-5.6")
                || model.starts_with("gpt-5.3-codex")
                || model.starts_with("gpt-6-astra") =>
        {
            levels.extend(["low", "medium", "high", "xhigh"])
        }
        _ => {}
    }
    levels.into_iter().map(str::to_string).collect()
}

#[derive(Clone, Serialize)]
struct Choice {
    id: String,
    label: String,
    description: String,
    efforts: Vec<String>,
}

pub struct SessionModel {
    pub client: Client,
    pub info: ProviderInfo,
    id: String,
    effort: String,
    catalog: Option<Vec<Choice>>,
}

impl SessionModel {
    pub fn new(client: &Client, info: &ProviderInfo) -> Self {
        let id = match info.provider {
            Provider::Cerebras => format!("cerebras:{}", info.resolved_model),
            Provider::AzureOpenAI => format!("azure:{}", info.resolved_model),
            _ => info.resolved_model.clone(),
        };
        Self {
            client: client.clone(),
            info: info.clone(),
            id,
            effort: "default".into(),
            catalog: None,
        }
    }

    fn catalog(&mut self) -> &Vec<Choice> {
        self.catalog.get_or_insert_with(|| {
            let mut choices = Vec::new();
            for (provider, rows, available) in get_available_models() {
                if !available || provider == Provider::Local {
                    continue;
                }
                for row in rows {
                    let id = row
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .next()
                        .unwrap_or("");
                    if id.is_empty() || id.contains(['<', '>']) {
                        continue;
                    }
                    choices.push(Choice {
                        id: id.into(),
                        label: id.into(),
                        description: provider.to_string(),
                        efforts: efforts(&provider, id),
                    });
                }
            }
            choices
        })
    }

    fn settings(&mut self) -> String {
        let current = Choice {
            id: self.id.clone(),
            label: self.id.clone(),
            description: self.info.provider.to_string(),
            efforts: efforts(&self.info.provider, &self.info.resolved_model),
        };
        if !self.catalog().iter().any(|c| c.id == current.id) {
            self.catalog.as_mut().unwrap().insert(0, current);
        }
        let cursor = self
            .catalog
            .as_ref()
            .unwrap()
            .iter()
            .position(|c| c.id == self.id)
            .unwrap_or(0);
        format!(
            "EUNICE_MODEL_SETTINGS {}",
            serde_json::json!({
                "agent": "eunice", "stage": "model", "title": "Model & effort", "options": self.catalog,
                "cursor": cursor, "effort": self.effort, "adjustable": false, "session_only": true, "fingerprint": ""
            })
        )
    }

    fn switch(&mut self, id: &str, effort: &str) -> Result<()> {
        if self.client.runtime() == crate::runtime::Runtime::OpenaiAgents && (id != self.id || effort != self.effort) {
            return Err(anyhow!("Managed sessions pin their model and effort; start a new Eunice process to change them."));
        }
        let info = if id == self.id {
            self.info.clone()
        } else {
            detect_provider(id)?
        };
        if !efforts(&info.provider, &info.resolved_model)
            .iter()
            .any(|e| e == effort)
        {
            return Err(anyhow!("Effort '{effort}' is not supported for {id}"));
        }
        // Local models may require downloads or server startup. Those belong to
        // launch, not an in-session switch, where history must remain intact.
        if info.provider == Provider::Local && id != self.id {
            return Err(anyhow!("Start this local model in a new window first; live switching requires an already running provider."));
        }
        let mut next = if id == self.id {
            self.client.clone()
        } else {
            Client::new(&info)?
        };
        next.set_runtime(self.client.runtime())?;
        next.set_effort(if effort == "default" {
            None
        } else {
            Some(effort.to_string())
        });
        self.client = next;
        self.info = info;
        self.id = id.into();
        self.effort = effort.into();
        Ok(())
    }

    /// Commands are intercepted before the agent loop, including malformed
    /// variants, so a settings command can never become a user prompt.
    pub fn handle(&mut self, input: &str) -> Option<String> {
        let args: Vec<_> = input.split_whitespace().collect();
        if !matches!(args.first(), Some(&"/model") | Some(&"/effort")) {
            return None;
        }
        if args == ["/model"] {
            return Some(format!("Model: {} · effort: {}\nUse /model <id> [effort] or /effort <level>. Conversation is preserved.\nUse /model --json for the available choices.", self.id, self.effort));
        }
        if args == ["/model", "--json"] {
            return Some(self.settings());
        }
        if args == ["/model", "--tmux"] {
            let settings = self.settings();
            return Some(
                match publish_tmux(
                    "@eunice_model_settings",
                    settings.trim_start_matches("EUNICE_MODEL_SETTINGS "),
                ) {
                    Ok(()) => format!("Model: {} · effort: {}", self.id, self.effort),
                    Err(e) => format!("Could not open model settings: {e}"),
                },
            );
        }
        if args == ["/effort"] {
            return Some(format!(
                "Effort: {} · available: {}",
                self.effort,
                efforts(&self.info.provider, &self.info.resolved_model).join(", ")
            ));
        }
        let (id, effort, nonce) = if args[0] == "/model" && (2..=4).contains(&args.len()) {
            (
                args[1].to_string(),
                args.get(2).copied().unwrap_or("default"),
                args.get(3).copied().unwrap_or("manual"),
            )
        } else if args[0] == "/effort" && args.len() == 2 {
            (self.id.clone(), args[1], "manual")
        } else {
            return Some(
                "EUNICE_MODEL_ERROR Usage: /model <id> [effort] or /effort <level>".into(),
            );
        };
        let result = self.switch(&id, effort);
        if nonce != "manual" {
            let _ = publish_tmux("@eunice_model_result", &serde_json::json!({"nonce": nonce, "success": result.is_ok(), "error": result.as_ref().err().map(ToString::to_string)}).to_string());
        }
        Some(match result {
            Ok(()) => format!(
                "Model: {} · effort: {} · conversation preserved",
                self.id, self.effort
            ),
            Err(e) => format!("Could not change model: {e}"),
        })
    }

    pub fn handle_with_history(
        &mut self,
        input: &str,
        history: &mut [crate::models::Message],
    ) -> Option<String> {
        // Any new input invalidates a previously opened web picker.
        let _ = publish_tmux("@eunice_model_settings", "");
        let previous = (self.info.provider.clone(), self.info.resolved_model.clone());
        let response = self.handle(input);
        if previous != (self.info.provider.clone(), self.info.resolved_model.clone()) {
            portable_history(history, &self.info.provider);
        }
        response
    }
}

fn publish_tmux(option: &str, value: &str) -> Result<()> {
    let pane = std::env::var("TMUX_PANE").map_err(|_| anyhow!("This command requires tmux"))?;
    let output = std::process::Command::new("tmux")
        .args(["set-option", "-p", "-t", &pane, option, value])
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("tmux could not publish model settings"));
    }
    Ok(())
}

/// Keep message content and tool-result links, but don't replay another model's
/// private thought signatures. Gemini requires function names for tool results;
/// OpenAI requires short, unique call ids without signature punctuation.
fn portable_history(history: &mut [crate::models::Message], provider: &Provider) {
    use crate::models::Message;
    let mut ids = std::collections::HashMap::new();
    let mut counter = 0;
    for message in history {
        if let Message::Assistant { native_output, .. } = message { *native_output = None; }
        match message {
            Message::Assistant {
                tool_calls: Some(calls),
                ..
            } => {
                for call in calls {
                    let id = if *provider == Provider::Gemini {
                        call.function.name.clone()
                    } else {
                        format!("call_migrated_{counter}")
                    };
                    counter += 1;
                    ids.insert(call.id.clone(), id.clone());
                    call.id = id;
                }
            }
            Message::Tool { tool_call_id, .. } => {
                if let Some(id) = ids.get(tool_call_id) {
                    *tool_call_id = id.clone();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn info() -> ProviderInfo {
        ProviderInfo {
            provider: Provider::Gemini,
            base_url: "http://localhost/".into(),
            api_key: "test".into(),
            resolved_model: "gemini-3.8-flash".into(),
            use_native_gemini_api: true,
            azure_api_version: None,
        }
    }
    #[test]
    fn supported_efforts_are_model_specific() {
        assert!(efforts(&Provider::Gemini, "gemini-3.8-flash").contains(&"medium".into()));
        assert!(!efforts(&Provider::Gemini, "gemini-3.1-pro-preview").contains(&"medium".into()));
        assert_eq!(efforts(&Provider::Ollama, "llama3"), ["default"]);
        assert_eq!(efforts(&Provider::OpenAI, "gpt-5-mini"), ["default"]);
    }
    #[test]
    fn invalid_change_keeps_previous_session_settings() {
        let info = info();
        let client = Client::new(&info).unwrap();
        let mut session = SessionModel::new(&client, &info);
        assert!(session
            .handle("/effort low")
            .unwrap()
            .contains("conversation preserved"));
        assert!(session
            .handle("/effort ultra")
            .unwrap()
            .starts_with("Could not change model"));
        assert_eq!(session.effort, "low");
        assert_eq!(session.info.resolved_model, info.resolved_model);
        assert!(session.handle("hello").is_none());
        assert!(session
            .handle("/model too many extra arguments")
            .unwrap()
            .starts_with("EUNICE_MODEL_ERROR"));
    }
    #[test]
    fn migrating_tool_history_keeps_content_and_matching_results() {
        use crate::models::{FunctionCall, Message, ToolCall};
        let mut history = vec![
            Message::User {
                content: "Remember amber-seven".into(),
            },
            Message::Assistant {
                native_output: None,
                content: Some("Reading".into()),
                tool_calls: Some(vec![ToolCall {
                    id: "Read::private-signature".into(),
                    call_type: "function".into(),
                    function: FunctionCall {
                        name: "Read".into(),
                        arguments: "{}".into(),
                    },
                }]),
            },
            Message::Tool {
                tool_call_id: "Read::private-signature".into(),
                content: "file contents".into(),
            },
        ];
        portable_history(&mut history, &Provider::OpenAI);
        let json = serde_json::to_value(&history).unwrap();
        assert_eq!(json[0]["content"], "Remember amber-seven");
        assert_eq!(json[1]["tool_calls"][0]["id"], json[2]["tool_call_id"]);
        assert_eq!(json[2]["content"], "file contents");
        assert!(!json.to_string().contains("private-signature"));
        portable_history(&mut history, &Provider::Gemini);
        assert_eq!(
            serde_json::to_value(&history).unwrap()[2]["tool_call_id"],
            "Read"
        );
    }
}
