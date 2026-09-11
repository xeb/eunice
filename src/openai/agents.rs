//! Managed sessions, observed through durable snapshots. Polling also provides
//! recovery after a lost connection without relying on replayable stream events.
use super::responses::{functions, required_str, usage};
use crate::{
    agent::{AgentResult, AgentStatus},
    client::Client,
    display_sink::{DisplayEvent, DisplaySink},
    models::{FunctionCall, Message, ToolCall},
    runtime::Conversation,
    tools::ToolRegistry,
    usage::SessionUsage,
};
use anyhow::{anyhow, bail, Context, Result};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedSession {
    pub id: Option<String>,
    pub model: String,
    pub endpoint: String,
    pub effort: Option<String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub active: bool,
    /// Persisted before a stateful POST. Never repeat an ambiguous submission.
    pub uncertain_input: bool,
    pub finished_turns: BTreeSet<String>,
    pub displayed_items: BTreeSet<String>,
    pub journal: BTreeMap<String, ToolResult>,
    pub cancel_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub action: Value,
    /// None means execution was claimed but its outcome was not committed.
    pub result: Option<Value>,
    pub full_output: Option<String>,
}

fn resource(id: &str) -> Result<String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Invalid remote resource ID");
    }
    Ok(format!("/agents/sessions/{id}"))
}

async fn get(client: &Client, path: &str) -> Result<Value> {
    let mut attempts = 0;
    loop {
        match client.agents_request(Method::GET, path, None).await {
            Ok(value) => return Ok(value),
            Err(error) => {
                let msg = format!("{error:#}");
                // Do not turn authentication or schema errors into silent polling.
                let retry = msg.contains("request failed")
                    || ["429", "500", "502", "503", "504"]
                        .iter()
                        .any(|s| msg.contains(&format!("HTTP {s}")));
                if !retry || attempts == 3 {
                    return Err(error);
                }
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(250 * (1 << attempts))).await;
            }
        }
    }
}

async fn list(client: &Client, path: &str) -> Result<Vec<Value>> {
    let mut items = Vec::new();
    let mut after = None;
    let mut cursors = BTreeSet::new();
    loop {
        let query = format!(
            "{path}?order=asc&limit=100{}",
            after
                .as_ref()
                .map(|id| format!("&after={id}"))
                .unwrap_or_default()
        );
        let page = get(client, &query).await?;
        items.extend(
            page["data"]
                .as_array()
                .ok_or_else(|| anyhow!("Missing paginated data"))?
                .iter()
                .cloned(),
        );
        if page["has_more"] == false {
            return Ok(items);
        }
        if page["has_more"] != true {
            bail!("Missing pagination has_more");
        }
        let cursor = required_str(&page, "last_id")?.to_string();
        resource(&cursor)?;
        if !cursors.insert(cursor.clone()) {
            bail!("Repeated pagination cursor");
        }
        after = Some(cursor);
    }
}

async fn send(client: &Client, path: &str, event: Value) -> Result<()> {
    client
        .agents_request(
            Method::POST,
            &format!("{path}/events"),
            Some(&json!({"events":[event]})),
        )
        .await?;
    Ok(())
}

async fn cancelled(rx: &mut Option<watch::Receiver<bool>>) {
    match rx {
        Some(rx) => loop {
            if *rx.borrow_and_update() {
                return;
            }
            if rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        },
        None => std::future::pending::<()>().await,
    }
}

/// Only the active root turn can determine the result; idle and subagent state
/// are deliberately insufficient.
fn root_turn<'a>(turns: &'a [Value], state: &ManagedSession) -> Option<&'a Value> {
    turns.iter().find(|turn| {
        turn.get("subagent_id").is_some_and(Value::is_null)
            && turn["id"]
                .as_str()
                .is_some_and(|id| !state.finished_turns.contains(id))
    })
}

fn preview(id: &str, text: &str) -> String {
    let lines: Vec<_> = text.lines().collect();
    if lines.len() <= 100 && text.len() <= 32000 {
        return text.to_string();
    }
    // Also cap single enormous lines, preserving character boundaries.
    let head: String = lines
        .iter()
        .take(10)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(8000)
        .collect();
    let tail: String = lines
        .iter()
        .skip(lines.len().saturating_sub(10))
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(8000)
        .collect();
    format!(
        "[Output {id}: {} lines. Use get_output for more.]\n{head}\n...\n{tail}",
        lines.len()
    )
}

fn read_output(state: &ManagedSession, args: &Value) -> Result<String> {
    let id = required_str(args, "id")?;
    let output = state
        .journal
        .get(id)
        .and_then(|entry| entry.full_output.as_deref())
        .ok_or_else(|| anyhow!("Unknown output ID: {id}"))?;
    let start = args["start"].as_u64().unwrap_or(0) as usize;
    let end = args["end"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(start.saturating_add(100));
    if end < start || end.saturating_sub(start) > 1000 {
        bail!("Request a range of at most 1000 lines");
    }
    Ok(output
        .lines()
        .skip(start)
        .take(end - start)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(32000)
        .collect())
}

async fn handle_action(
    client: &Client,
    path: &str,
    action: &Value,
    conversation: &mut Conversation,
    tools: &ToolRegistry,
    display: &dyn DisplaySink,
    limit: usize,
) -> Result<()> {
    if action["type"] != "function_call" {
        bail!("Unsupported required action: {}", action["type"]);
    }
    let call_id = required_str(action, "call_id")?;
    let turn_id = required_str(action, "turn_id")?;
    let name = required_str(action, "name")?;
    let key = format!("{turn_id}:{call_id}");
    if let Some(entry) = conversation.managed.journal.get(&key) {
        if entry.action != *action {
            bail!("Function action changed for existing call {key}");
        }
        let result = entry.result.clone().ok_or_else(|| anyhow!("Tool {key} has an uncertain outcome after interruption; inspect its effects before retrying. It will not be run again automatically."))?;
        send(client, path, result).await?;
        return Ok(());
    }
    conversation.managed.journal.insert(
        key.clone(),
        ToolResult {
            action: action.clone(),
            result: None,
            full_output: None,
        },
    );
    conversation.messages.push(Message::Assistant {
        native_output: None,
        content: None,
        tool_calls: Some(vec![ToolCall {
            id: call_id.into(),
            call_type: "function".into(),
            function: FunctionCall {
                name: name.into(),
                arguments: action["arguments"].to_string(),
            },
        }]),
    });
    // Claim is durable before execution. The web layer holds an exclusive session
    // lease while this future runs; reconnecting observers never execute tools.
    conversation.save().await?;
    display.write_event(DisplayEvent::ThinkingStop);
    display.write_event(DisplayEvent::ToolCall {
        name: name.into(),
        arguments: action["arguments"].to_string(),
    });
    let result = if !action["arguments"].is_object() {
        Err(anyhow!("Function arguments must be an object"))
    } else if name == "get_output" {
        read_output(&conversation.managed, &action["arguments"])
    } else {
        tools.execute(name, action["arguments"].clone()).await
    };
    let (success, output) = match result {
        Ok(text) => (true, text),
        Err(e) => (false, format!("{e:#}")),
    };
    let mut event = json!({"type":"agent.session.input.tool_result","call_id":call_id,"turn_id":turn_id,"success":success});
    event[if success { "output" } else { "error" }] = json!(preview(&key, &output));
    let entry = conversation.managed.journal.get_mut(&key).unwrap();
    entry.result = Some(event.clone());
    entry.full_output = Some(output.clone());
    conversation.messages.push(Message::Tool {
        tool_call_id: call_id.into(),
        content: preview(&key, &output),
    });
    conversation.save().await?;
    display.write_event(DisplayEvent::ToolResult {
        result: output,
        limit,
    });
    send(client, path, event).await?;
    display.write_event(DisplayEvent::ThinkingStart);
    Ok(())
}

async fn collect_items(
    client: &Client,
    path: &str,
    conversation: &mut Conversation,
    display: &dyn DisplaySink,
) -> Result<()> {
    for item in list(client, &format!("{path}/items")).await? {
        if item["type"] != "message" || item["role"] != "assistant" || item["status"] != "completed"
        {
            continue;
        }
        let id = required_str(&item, "id")?.to_string();
        if conversation.managed.displayed_items.contains(&id) {
            continue;
        }
        let content = item["content"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing managed message content"))?;
        let text = content
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        conversation.managed.displayed_items.insert(id);
        conversation.messages.push(Message::Assistant {
            content: Some(text.clone()),
            tool_calls: None,
            native_output: None,
        });
        conversation.save().await?;
        if !text.is_empty() {
            display.write_event(DisplayEvent::ThinkingStop);
            display.write_event(DisplayEvent::Response { content: text });
        }
    }
    Ok(())
}

async fn drive(
    client: &Client,
    path: &str,
    conversation: &mut Conversation,
    tools: &ToolRegistry,
    display: &dyn DisplaySink,
    limit: usize,
) -> Result<AgentResult> {
    let started = tokio::time::Instant::now();
    loop {
        let session = get(client, path).await?;
        if session["status"] == "failed" {
            conversation.managed.active = false;
            conversation.managed.uncertain_input = false;
            conversation.managed.cancel_pending = false;
            conversation.save().await?;
            bail!("Managed session failed: {}", session["error"]);
        }
        let turns = list(client, &format!("{path}/turns")).await?;
        collect_items(client, path, conversation, display).await?;
        if let Some(turn) = root_turn(&turns, &conversation.managed) {
            let id = required_str(turn, "id")?.to_string();
            conversation.managed.uncertain_input = false;
            match required_str(turn, "status")? {
                "completed" | "failed" | "cancelled" => {
                    conversation.managed.finished_turns.insert(id);
                    conversation.managed.active = false;
                    conversation.managed.cancel_pending = false;
                    conversation.save().await?;
                    let mut total = SessionUsage::new();
                    if let Some(usage) = usage(&turn["usage"]) {
                        total.add(&usage);
                        total.aggregate_only = true;
                    }
                    if turn["status"] == "failed" {
                        bail!("Managed turn failed: {}", turn["error"]);
                    }
                    return Ok(AgentResult {
                        status: if turn["status"] == "cancelled" {
                            AgentStatus::Cancelled
                        } else {
                            AgentStatus::Completed
                        },
                        usage: total,
                    });
                }
                "queued" | "in_progress" | "waiting" => {}
                other => bail!("Unknown managed turn status: {other}"),
            }
        }
        if root_turn(&turns, &conversation.managed).is_none()
            && started.elapsed() > Duration::from_secs(30)
        {
            bail!("No root turn was found for the submitted input. The outcome is uncertain; reconnect to reconcile it before sending another message.");
        }
        let actions = session["required_actions"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing required_actions"))?;
        for action in actions {
            if conversation.managed.cancel_pending {
                break;
            }
            if !root_turn(&turns, &conversation.managed)
                .is_some_and(|turn| turn["id"] == action["turn_id"])
            {
                bail!("Required action does not belong to the active root turn");
            }
            handle_action(client, path, action, conversation, tools, display, limit).await?;
        }
        // No terminal root turn means keep observing, including an idle snapshot.
        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}

async fn drive_cancellable(
    client: &Client,
    path: &str,
    conversation: &mut Conversation,
    tools: &ToolRegistry,
    display: &dyn DisplaySink,
    rx: &mut Option<watch::Receiver<bool>>,
    limit: usize,
) -> Result<AgentResult> {
    if conversation.managed.cancel_pending {
        return cancel_and_observe(client, path, conversation, tools, display, limit).await;
    }
    let outcome = tokio::select! {
        biased;
        _ = cancelled(rx) => None,
        result = drive(client,path,conversation,tools,display,limit) => Some(result),
    };
    if let Some(result) = outcome {
        return result;
    }
    conversation.managed.cancel_pending = true;
    conversation.save().await?;
    cancel_and_observe(client, path, conversation, tools, display, limit).await
}

async fn cancel_and_observe(
    client: &Client,
    path: &str,
    conversation: &mut Conversation,
    tools: &ToolRegistry,
    display: &dyn DisplaySink,
    limit: usize,
) -> Result<AgentResult> {
    send(client, path, json!({"type":"agent.session.input.cancel"}))
        .await
        .context("Remote cancellation was not confirmed; session remains recoverable")?;
    tokio::time::timeout(
        Duration::from_secs(15),
        drive(client, path, conversation, tools, display, limit),
    )
    .await
    .context("Remote cancellation is pending; reconnect to reconcile the session")?
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    client: &Client,
    model: &str,
    prompt: &str,
    tools: &ToolRegistry,
    display: Arc<dyn DisplaySink>,
    conversation: &mut Conversation,
    mut cancel: Option<watch::Receiver<bool>>,
    limit: usize,
) -> Result<AgentResult> {
    let result = run_inner(
        client,
        model,
        prompt,
        tools,
        display.as_ref(),
        conversation,
        &mut cancel,
        limit,
    )
    .await;
    display.write_event(DisplayEvent::ThinkingStop);
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_inner(
    client: &Client,
    model: &str,
    prompt: &str,
    tools: &ToolRegistry,
    display: &dyn DisplaySink,
    conversation: &mut Conversation,
    cancel: &mut Option<watch::Receiver<bool>>,
    limit: usize,
) -> Result<AgentResult> {
    let endpoint = client.session_info(model).base_url;
    let cwd = tools
        .cwd()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    if conversation.managed.id.is_some()
        && (conversation.managed.model != model
            || conversation.managed.endpoint != endpoint
            || conversation.managed.effort.as_deref() != client.effort()
            || conversation.managed.working_dir.as_deref() != Some(cwd.as_path()))
    {
        bail!("Managed session configuration differs from this client; start a new session");
    }
    if conversation.managed.id.is_none() && conversation.managed.uncertain_input {
        bail!("Session creation had an uncertain outcome. Inspect the OpenAI Agents dashboard before starting a new session.");
    }
    display.write_event(DisplayEvent::ThinkingStart);
    if let Some(id) = conversation.managed.id.clone() {
        let path = resource(&id)?;
        if conversation.managed.active {
            display.write_event(DisplayEvent::Info {
                message: format!("Recovering managed session {id}"),
            });
            let recovered =
                drive_cancellable(client, &path, conversation, tools, display, cancel, limit)
                    .await?;
            if prompt.is_empty() || recovered.status == AgentStatus::Cancelled {
                return Ok(recovered);
            }
        }
        if prompt.is_empty() {
            return Ok(AgentResult {
                status: AgentStatus::Completed,
                usage: SessionUsage::new(),
            });
        }
        // Mark submission before sending. After an ambiguous failure, the next run
        // observes this turn instead of replaying its input.
        conversation.managed.active = true;
        conversation.managed.uncertain_input = true;
        conversation.messages.push(Message::User {
            content: prompt.into(),
        });
        conversation.save().await?;
        if let Err(error) = send(client,&path,json!({"type":"agent.session.input.message","input":[{"role":"user","content":[{"type":"input_text","text":prompt}]}]})).await {
            reject_input(conversation, &error).await?;
            return Err(error);
        }
    } else {
        if prompt.is_empty() {
            bail!("A new managed session requires a prompt");
        }
        conversation.managed.working_dir = Some(cwd);
        conversation.managed.model = model.into();
        conversation.managed.endpoint = endpoint;
        conversation.managed.effort = client.effort().map(str::to_string);
        conversation.managed.uncertain_input = true;
        conversation.messages.push(Message::User {
            content: prompt.into(),
        });
        conversation.save().await?;
        let mut specs = tools.get_tools();
        specs.push(crate::agent::get_get_output_tool_spec());
        let mut agent = json!({"model":model,"instructions":"You are Eunice. Use the provided local tools to complete the user's task. Tool paths refer to the user's working directory. Use get_output to retrieve truncated results.","tools":functions(Some(&specs),false),"multi_agent":{"enabled":false}});
        if let Some(effort) = client.effort() {
            agent["reasoning"] = json!({"effort":effort});
        }
        let session = match client
            .agents_request(
                Method::POST,
                "/agents/sessions",
                Some(&json!({"agent":agent,"environment":{"type":"none"},"input":prompt})),
            )
            .await
        {
            Ok(session) => session,
            Err(error) => {
                reject_input(conversation, &error).await?;
                return Err(error);
            }
        };
        let id = required_str(&session, "id")?.to_string();
        resource(&id)?;
        conversation.managed.id = Some(id.clone());
        conversation.managed.active = true;
        conversation.managed.uncertain_input = false;
        conversation.save().await?;
        display.write_event(DisplayEvent::Info {
            message: format!("OpenAI managed session: {id}"),
        });
    }
    let path = resource(conversation.managed.id.as_deref().unwrap())?;
    drive_cancellable(client, &path, conversation, tools, display, cancel, limit).await
}

async fn reject_input(conversation: &mut Conversation, error: &anyhow::Error) -> Result<()> {
    let message = format!("{error:#}");
    if [400, 401, 403, 404, 422, 429]
        .iter()
        .any(|code| message.contains(&format!("HTTP {code}:")))
    {
        conversation.managed.active = false;
        conversation.managed.uncertain_input = false;
        conversation.messages.pop();
        conversation.save().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_unfinished_root_turns_count() {
        let mut state = ManagedSession::default();
        state.finished_turns.insert("old".into());
        let turns = vec![
            json!({"id":"old","subagent_id":null,"status":"completed"}),
            json!({"id":"child","subagent_id":"sub_1","status":"completed"}),
            json!({"id":"root","subagent_id":null,"status":"waiting"}),
        ];
        assert_eq!(root_turn(&turns, &state).unwrap()["id"], "root");
        assert!(root_turn(&turns[..2], &state).is_none());
    }
    #[test]
    fn resource_ids_cannot_change_request_paths() {
        for id in ["", "../other", "a?x=y", "a/b", "a#b"] {
            assert!(resource(id).is_err());
        }
        assert_eq!(resource("sess_a-1").unwrap(), "/agents/sessions/sess_a-1");
    }
    #[test]
    fn journal_round_trips_claimed_and_completed_actions() {
        let mut state = ManagedSession::default();
        state.journal.insert(
            "t:c".into(),
            ToolResult {
                action: json!({"call_id":"c"}),
                result: None,
                full_output: None,
            },
        );
        let loaded: ManagedSession =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert!(loaded.journal["t:c"].result.is_none());
        state.journal.get_mut("t:c").unwrap().result = Some(json!({"success":true,"output":"ok"}));
        let loaded: ManagedSession =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(
            loaded.journal["t:c"].result.as_ref().unwrap()["output"],
            "ok"
        );
    }
    #[test]
    fn long_unicode_output_stays_retrievable_after_restart() {
        let text = (0..150)
            .map(|i| format!("line {i} 🦀"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(preview("t:c", &text).contains("get_output"));
        let mut state = ManagedSession::default();
        state.journal.insert(
            "t:c".into(),
            ToolResult {
                action: Value::Null,
                result: None,
                full_output: Some(text),
            },
        );
        let state: ManagedSession =
            serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
        assert_eq!(
            read_output(&state, &json!({"id":"t:c","start":70,"end":72})).unwrap(),
            "line 70 🦀\nline 71 🦀"
        );
        assert!(read_output(&state, &json!({"id":"t:c","start":9,"end":1})).is_err());
    }
}
