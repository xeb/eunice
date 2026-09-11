//! Lossless Responses history with a portable display/tool-call projection.
use crate::models::{
    AssistantMessage, ChatCompletionResponse, Choice, FunctionCall, Tool, ToolCall, UsageStats,
};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

pub fn functions(tools: Option<&[Tool]>, responses: bool) -> Vec<Value> {
    tools
        .unwrap_or_default()
        .iter()
        .map(|tool| {
            let mut value = json!({"type":"function", "name":tool.function.name,
            "description":tool.function.description,"parameters":tool.function.parameters});
            if responses {
                value["strict"] = json!(false);
            }
            value
        })
        .collect()
}

pub fn request(
    model: &str,
    messages: Value,
    tools: Option<&[Tool]>,
    effort: Option<&str>,
    stream: bool,
) -> Result<Value> {
    if let Some(effort) = effort {
        if !["low", "medium", "high", "xhigh", "max"].contains(&effort) {
            bail!("Unsupported Astra reasoning effort: {effort}");
        }
    }
    let mut input = Vec::new();
    for message in messages
        .as_array()
        .ok_or_else(|| anyhow!("Messages must be an array"))?
    {
        match message["role"].as_str() {
            Some("assistant") => {
                if let Some(items) = message["native_output"].as_array() {
                    input.extend(items.iter().cloned());
                    continue;
                }
                if let Some(text) = message["content"].as_str().filter(|s| !s.is_empty()) {
                    input.push(json!({"role":"assistant","content":text}));
                }
                if let Some(calls) = message["tool_calls"].as_array() {
                    for call in calls {
                        input.push(json!({"type":"function_call","call_id":call["id"],
                            "name":call["function"]["name"],"arguments":call["function"]["arguments"]}));
                    }
                }
            }
            Some("tool") => input.push(json!({"type":"function_call_output","call_id":message["tool_call_id"],"output":message["content"]})),
            Some("user" | "system" | "developer") => input.push(message.clone()),
            _ => bail!("Unsupported message role in Responses history"),
        }
    }
    // Native compaction carries everything before it. Keep the display history intact.
    if let Some(index) = input.iter().rposition(|item| item["type"] == "compaction") {
        input.drain(..index);
    }
    let mut body = json!({"model":model,"input":input,"store":false,"stream":stream,
        "include":["reasoning.encrypted_content"],"context_management":[{"type":"compaction","compact_threshold":800000}]});
    if let Some(effort) = effort {
        body["reasoning"] = json!({"effort":effort});
    }
    if tools.is_some_and(|tools| !tools.is_empty()) {
        body["tools"] = json!(functions(tools, true));
    }
    Ok(body)
}

pub fn parse(value: Value) -> Result<ChatCompletionResponse> {
    if value["status"] != "completed" {
        bail!(
            "Responses request did not complete: {}",
            value
                .get("error")
                .filter(|v| !v.is_null())
                .or_else(|| value.get("incomplete_details").filter(|v| !v.is_null()))
                .unwrap_or(&value["status"])
        );
    }
    let output = value["output"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing Responses output"))?;
    let mut texts = Vec::new();
    let mut calls = Vec::new();
    for item in output {
        match item["type"].as_str() {
            Some("message") => {
                if item["status"].as_str().is_some_and(|s| s != "completed") {
                    bail!("Incomplete assistant message");
                }
                for part in item["content"]
                    .as_array()
                    .ok_or_else(|| anyhow!("Missing assistant content"))?
                {
                    if let Some(text) = part["text"].as_str().or_else(|| part["refusal"].as_str()) {
                        texts.push(text.to_string());
                    }
                }
            }
            Some("function_call") => {
                let call_id = required_str(item, "call_id")?;
                if calls.iter().any(|c: &ToolCall| c.id == call_id) {
                    bail!("Duplicate function call ID: {call_id}");
                }
                let arguments = required_str(item, "arguments")?;
                let parsed: Value = serde_json::from_str(arguments)?;
                if !parsed.is_object() {
                    bail!("Function arguments must be an object");
                }
                calls.push(ToolCall {
                    id: call_id.to_string(),
                    call_type: "function".into(),
                    function: FunctionCall {
                        name: required_str(item, "name")?.into(),
                        arguments: arguments.into(),
                    },
                });
            }
            Some("reasoning" | "compaction") => {}
            Some(kind) => bail!("Unsupported Responses output item: {kind}"),
            None => bail!("Output item is missing type"),
        }
    }
    if texts.is_empty() && calls.is_empty() {
        bail!("Responses completed without text or function calls");
    }
    Ok(ChatCompletionResponse {
        choices: vec![Choice {
            message: AssistantMessage {
                content: (!texts.is_empty()).then(|| texts.join("\n")),
                tool_calls: (!calls.is_empty()).then_some(calls),
                native_output: Some(output.clone()),
            },
        }],
        usage: usage(&value["usage"]),
    })
}

pub fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Missing or invalid {key}"))
}

pub fn usage(value: &Value) -> Option<UsageStats> {
    value.as_object()?;
    Some(UsageStats {
        prompt_tokens: value["input_tokens"].as_u64().unwrap_or(0),
        completion_tokens: value["output_tokens"].as_u64().unwrap_or(0),
        total_tokens: value["total_tokens"].as_u64().unwrap_or(0),
        cached_tokens: value["input_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0),
        cache_write_tokens: value["input_tokens_details"]["cache_write_tokens"]
            .as_u64()
            .unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message() -> Value {
        json!({"type":"message","status":"completed","role":"assistant","phase":"final_answer","content":[{"type":"output_text","text":"done"}]})
    }
    fn completed(items: Value) -> Value {
        json!({"status":"completed","output":items})
    }
    #[test]
    fn native_items_round_trip_without_duplicate_projection() {
        let items = json!([
            {"type":"reasoning","encrypted_content":"secret","summary":[]},
            {"type":"message","phase":"commentary","status":"completed","content":[{"type":"output_text","text":"checking"}]},
            {"type":"function_call","id":"item_1","call_id":"call_1","name":"Read","arguments":"{\"path\":\"x\"}"},
            {"type":"function_call","id":"item_2","call_id":"call_2","name":"Read","arguments":"{\"path\":\"y\"}"}
        ]);
        let parsed = parse(completed(items.clone())).unwrap();
        let assistant = &parsed.choices[0].message;
        assert_eq!(assistant.tool_calls.as_ref().unwrap()[1].id, "call_2");
        let history = json!([{"role":"assistant","content":assistant.content,"tool_calls":assistant.tool_calls,"native_output":assistant.native_output},
            {"role":"tool","tool_call_id":"call_1","content":"x"},{"role":"tool","tool_call_id":"call_2","content":"y"}]);
        let body = request("gpt-6-astra", history, None, None, false).unwrap();
        assert_eq!(body["input"].as_array().unwrap().len(), 6);
        assert_eq!(body["input"][0], items[0]);
        assert_eq!(body["input"][1]["phase"], "commentary");
        assert_eq!(body["input"][4]["call_id"], "call_1");
    }
    #[test]
    fn effort_parameters_are_validated_and_default_is_omitted() {
        for effort in ["low", "medium", "high", "xhigh", "max"] {
            let body = request("gpt-6-astra", json!([]), None, Some(effort), false).unwrap();
            assert_eq!(body["reasoning"]["effort"], effort);
            for key in ["temperature", "top_p", "max_completion_tokens", "messages"] {
                assert!(body.get(key).is_none());
            }
        }
        for effort in ["none", "minimal", "ultra", "bogus"] {
            assert!(request("gpt-6-astra", json!([]), None, Some(effort), false).is_err());
        }
        assert!(request("gpt-6-astra", json!([]), None, None, false)
            .unwrap()
            .get("reasoning")
            .is_none());
    }
    #[test]
    fn rejects_failed_incomplete_missing_and_unsupported_outputs() {
        for value in [
            json!({"status":"failed","error":{"message":"bad"}}),
            json!({"status":"incomplete","output":[message()]}),
            completed(json!([])),
            completed(json!([{"type":"new_tool"}])),
            completed(json!([{"type":"function_call","name":"Read","arguments":"{}"}])),
            completed(
                json!([{"type":"function_call","call_id":"c","name":"Read","arguments":"{"}]),
            ),
            completed(
                json!([{"type":"function_call","call_id":"c","name":"Read","arguments":"[]"}]),
            ),
        ] {
            assert!(parse(value.clone()).is_err(), "accepted {value}");
        }
    }
    #[test]
    fn duplicate_call_ids_fail_before_execution() {
        let call = json!({"type":"function_call","call_id":"c","name":"Read","arguments":"{}"});
        assert!(parse(completed(json!([call, call])))
            .unwrap_err()
            .to_string()
            .contains("Duplicate"));
    }
    #[test]
    fn compaction_prunes_protocol_history_but_keeps_current_tool_results() {
        let compact = json!({"type":"compaction","id":"cmp_1","encrypted_content":"opaque"});
        let body = request("gpt-6-astra",json!([{"role":"user","content":"old"},{"role":"assistant","native_output":[compact,message()]},{"role":"user","content":"next"}]),None,None,false).unwrap();
        assert_eq!(body["input"][0], compact);
        assert_eq!(body["input"][2]["content"], "next");
    }
    #[test]
    fn refusal_and_usage_are_preserved() {
        let mut value = completed(
            json!([{"type":"message","content":[{"type":"refusal","refusal":"Cannot help"}]}]),
        );
        value["usage"] = json!({"input_tokens":100,"output_tokens":20,"total_tokens":120,"input_tokens_details":{"cached_tokens":50,"cache_write_tokens":25},"output_tokens_details":{"reasoning_tokens":10}});
        let parsed = parse(value).unwrap();
        assert_eq!(
            parsed.choices[0].message.content.as_deref(),
            Some("Cannot help")
        );
        let usage = parsed.usage.unwrap();
        assert_eq!(
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.cached_tokens,
                usage.cache_write_tokens
            ),
            (100, 20, 50, 25)
        );
    }
}
