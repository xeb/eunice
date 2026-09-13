//! OpenAI-compatible SSE assembly for the local inference server.
//! Text can be displayed incrementally; tool calls are returned only after a
//! complete, successful stream with valid JSON arguments.
use crate::models::{
    AssistantMessage, ChatCompletionResponse, Choice, FunctionCall, ToolCall, UsageStats,
};
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
}
impl SseDecoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<String>> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 8 * 1024 * 1024 {
            bail!("Local SSE line exceeds 8 MiB");
        }
        let mut events = Vec::new();
        while let Some(end) = self.pending.iter().position(|c| *c == b'\n') {
            let raw = self.pending.drain(..=end).collect::<Vec<_>>();
            let line = std::str::from_utf8(&raw)?.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if !self.data.is_empty() {
                    events.push(self.data.join("\n"));
                    self.data.clear();
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                self.data
                    .push(value.strip_prefix(' ').unwrap_or(value).to_string());
            }
        }
        Ok(events)
    }
}

#[derive(Default)]
struct PartialCall {
    id: String,
    name: String,
    arguments: String,
}
#[derive(Default)]
pub struct CompletionStream {
    text: String,
    calls: BTreeMap<u64, PartialCall>,
    usage: Option<UsageStats>,
    finish: Option<String>,
    done: bool,
}
impl CompletionStream {
    pub fn push(&mut self, event: &str) -> Result<Option<String>> {
        if self.done {
            bail!("Data received after local stream completion");
        }
        if event == "[DONE]" {
            self.done = true;
            return Ok(None);
        }
        let value: Value = serde_json::from_str(event)?;
        if let Some(error) = value.get("error") {
            bail!("Local inference error: {error}");
        }
        if let Some(usage) = value.get("usage").filter(|v| !v.is_null()) {
            self.usage = Some(serde_json::from_value(usage.clone())?);
        }
        let mut displayed = String::new();
        if let Some(choices) = value["choices"].as_array() {
            for choice in choices {
                if choice["index"].as_u64().unwrap_or(0) != 0 {
                    bail!("Unexpected multiple local completions");
                }
                let delta = &choice["delta"];
                if self.finish.is_some()
                    && (delta.get("content").is_some_and(|v| !v.is_null())
                        || delta.get("tool_calls").is_some())
                {
                    bail!("Local completion continued after finish_reason");
                }
                if let Some(text) = delta["content"].as_str() {
                    self.text.push_str(text);
                    displayed.push_str(text);
                }
                if let Some(calls) = delta["tool_calls"].as_array() {
                    for fragment in calls {
                        let index = fragment["index"]
                            .as_u64()
                            .ok_or_else(|| anyhow!("Tool fragment lacks index"))?;
                        if index > 127 {
                            bail!("Too many tool calls in local completion");
                        }
                        if let Some(kind) = fragment["type"].as_str() {
                            if kind != "function" {
                                bail!("Unsupported local tool type: {kind}");
                            }
                        }
                        let call = self.calls.entry(index).or_default();
                        if let Some(id) = fragment["id"].as_str() {
                            call.id.push_str(id);
                        }
                        if let Some(name) = fragment["function"]["name"].as_str() {
                            call.name.push_str(name);
                        }
                        if let Some(args) = fragment["function"]["arguments"].as_str() {
                            call.arguments.push_str(args);
                        }
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    self.finish = Some(reason.into());
                }
            }
        }
        Ok((!displayed.is_empty()).then_some(displayed))
    }

    pub fn finish(self) -> Result<ChatCompletionResponse> {
        if !self.done {
            bail!("Local stream ended before [DONE]; incomplete tool calls were not executed");
        }
        if !matches!(self.finish.as_deref(), Some("stop" | "tool_calls")) {
            bail!("Local generation did not finish successfully ({:?}); no tool calls were executed. Shorten the request or increase EUNICE_LOCAL_PREDICT.", self.finish);
        }
        if !self.calls.is_empty() && self.finish.as_deref() != Some("tool_calls") {
            bail!("Local tool calls lack a tool_calls finish reason");
        }
        if self.calls.is_empty() && self.finish.as_deref() == Some("tool_calls") {
            bail!("Local server reported tool_calls without any calls");
        }
        let mut ids = std::collections::HashSet::new();
        let mut calls = Vec::new();
        for (_, call) in self.calls {
            if call.id.is_empty() || call.name.is_empty() || !ids.insert(call.id.clone()) {
                bail!("Incomplete or duplicate local tool call");
            }
            let args: Value = serde_json::from_str(&call.arguments)?;
            if !args.is_object() {
                bail!("Tool arguments must be a JSON object");
            }
            calls.push(ToolCall {
                id: call.id,
                call_type: "function".into(),
                function: FunctionCall {
                    name: call.name,
                    arguments: call.arguments,
                },
            });
        }
        Ok(ChatCompletionResponse {
            choices: vec![Choice {
                message: AssistantMessage {
                    content: (!self.text.is_empty()).then_some(self.text),
                    tool_calls: (!calls.is_empty()).then_some(calls),
                    native_output: None,
                },
            }],
            usage: self.usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn fragmented_utf8_and_crlf() {
        let mut decoder = SseDecoder::default();
        let mut events = Vec::new();
        for byte in "data: {\"text\":\"héllo\"}\r\n\r\ndata: [DONE]\n\n".as_bytes() {
            events.extend(decoder.feed(&[*byte]).unwrap());
        }
        assert_eq!(events, vec!["{\"text\":\"héllo\"}", "[DONE]"]);
    }
    #[test]
    fn assembles_interleaved_tool_calls_and_usage() {
        let mut stream = CompletionStream::default();
        stream.push(&json!({"choices":[{"index":0,"delta":{"content":"Checking.","tool_calls":[
            {"index":0,"id":"one","type":"function","function":{"name":"Read","arguments":"{\"path\":"}},
            {"index":1,"id":"two","type":"function","function":{"name":"Bash","arguments":"{\"command\":"}}
        ]}}]}).to_string()).unwrap();
        stream
            .push(
                &json!({"choices":[{"index":0,"delta":{"tool_calls":[
            {"index":1,"function":{"arguments":"\"pwd\"}"}},
            {"index":0,"function":{"arguments":"\"notes.txt\"}"}}
        ]},"finish_reason":"tool_calls"}]})
                .to_string(),
            )
            .unwrap();
        stream.push(r#"{"choices":[],"usage":{"prompt_tokens":42,"completion_tokens":16,"total_tokens":58}}"#).unwrap();
        stream.push("[DONE]").unwrap();
        let response = stream.finish().unwrap();
        assert_eq!(response.usage.unwrap().total_tokens, 58);
        let calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(calls[0].function.arguments, r#"{"path":"notes.txt"}"#);
        assert_eq!(calls[1].function.arguments, r#"{"command":"pwd"}"#);
    }
    #[test]
    fn rejects_truncated_or_token_limited_streams() {
        for done in [false, true] {
            let mut stream = CompletionStream::default();
            stream.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"x","function":{"name":"Bash","arguments":"{}"}}]},"finish_reason":"length"}]}"#).unwrap();
            if done {
                stream.push("[DONE]").unwrap();
            }
            assert!(stream.finish().is_err());
        }
    }
    #[test]
    fn rejects_malformed_arguments_even_after_done() {
        let mut stream = CompletionStream::default();
        stream.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"x","function":{"name":"Bash","arguments":"{"}}]},"finish_reason":"tool_calls"}]}"#).unwrap();
        stream.push("[DONE]").unwrap();
        assert!(stream.finish().is_err());
    }
    #[test]
    fn accepts_text_only_completion() {
        let mut stream = CompletionStream::default();
        assert_eq!(
            stream
                .push(r#"{"choices":[{"delta":{"content":"hello"},"finish_reason":"stop"}]}"#)
                .unwrap(),
            Some("hello".into())
        );
        stream.push("[DONE]").unwrap();
        assert_eq!(
            stream.finish().unwrap().choices[0]
                .message
                .content
                .as_deref(),
            Some("hello")
        );
    }
}
