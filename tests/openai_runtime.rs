use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    response::Response,
    Router,
};
use eunice::{
    agent::{run_agent_cancellable, AgentStatus},
    client::Client,
    display_sink::{DisplayEvent, DisplaySink},
    models::{Provider, ProviderInfo},
    runtime::{Conversation, Runtime},
    tools::ToolRegistry,
};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

struct Step {
    method: &'static str,
    path: &'static str,
    status: u16,
    body: String,
    sse: bool,
}
fn step(method: &'static str, path: &'static str, body: Value) -> Step {
    Step {
        method,
        path,
        status: 200,
        body: body.to_string(),
        sse: false,
    }
}
fn response(output: Value) -> Value {
    json!({"status":"completed","output":output,"usage":{"input_tokens":100,"output_tokens":20,"total_tokens":120}})
}
fn text_item(id: &str, text: &str) -> Value {
    json!({"id":id,"type":"message","role":"assistant","status":"completed","phase":"final_answer","content":[{"type":"output_text","text":text}]})
}
fn stream_response(output: Value) -> Step {
    let data = json!({"type":"response.completed","response":response(output)});
    Step {
        method: "POST",
        path: "/v1/responses",
        status: 200,
        body: format!("event: response.completed\ndata: {data}\n\n"),
        sse: true,
    }
}
fn page(items: Value) -> Value {
    json!({"data":items,"has_more":false})
}
fn action(name: &str, args: Value) -> Value {
    json!({"type":"function_call","turn_id":"turn_1","call_id":"call_1","name":name,"arguments":args})
}
fn turn(id: &str, status: &str) -> Value {
    json!({"id":id,"status":status,"subagent_id":null,"usage":{"input_tokens":10,"output_tokens":5}})
}
#[derive(Clone)]
struct MockState {
    steps: Arc<Mutex<VecDeque<Step>>>,
    requests: Arc<Mutex<Vec<Value>>>,
}
struct Mock {
    state: MockState,
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Mock {
    async fn new(steps: Vec<Step>) -> Self {
        let state = MockState {
            steps: Arc::new(Mutex::new(steps.into())),
            requests: Default::default(),
        };
        let app =
            Router::new()
                .fallback(|State(state): State<MockState>, req: Request| async move {
                    let path = req.uri().to_string();
                    let method = req.method().to_string();
                    let auth = req
                        .headers()
                        .get("authorization")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    let beta = req
                        .headers()
                        .get("openai-beta")
                        .map(|v| v.to_str().unwrap().to_owned());
                    let bytes = to_bytes(req.into_body(), 32 * 1024 * 1024).await.unwrap();
                    let body: Value = if bytes.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&bytes).unwrap()
                    };
                    state.requests.lock().unwrap().push(
                        json!({"path":path,"method":method,"auth":auth,"beta":beta,"body":body}),
                    );
                    let expected = state
                        .steps
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected HTTP request");
                    assert_eq!(method, expected.method);
                    assert_eq!(path, expected.path);
                    let response_body = if expected.sse {
                        // Every byte is a distinct network chunk, including UTF-8 continuation bytes.
                        Body::from_stream(futures::stream::iter(
                            expected
                                .body
                                .into_bytes()
                                .into_iter()
                                .map(|b| Ok::<_, std::io::Error>(vec![b])),
                        ))
                    } else {
                        Body::from(expected.body)
                    };
                    Response::builder()
                        .status(expected.status)
                        .header(
                            "content-type",
                            if expected.sse {
                                "text/event-stream"
                            } else {
                                "application/json"
                            },
                        )
                        .body(response_body)
                        .unwrap()
                })
                .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1/", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self { state, url, task }
    }
    fn client(&self, runtime: Runtime) -> Client {
        let mut client = Client::new(&ProviderInfo {
            provider: Provider::OpenAI,
            base_url: self.url.clone(),
            api_key: "test-key".into(),
            resolved_model: "gpt-6-astra".into(),
            use_native_gemini_api: false,
            azure_api_version: None,
        })
        .unwrap();
        client.set_runtime(runtime).unwrap();
        client
    }
    fn done(&self) -> Vec<Value> {
        assert!(
            self.state.steps.lock().unwrap().is_empty(),
            "not all expected requests were made"
        );
        self.state.requests.lock().unwrap().clone()
    }
}
#[derive(Default)]
struct Display(Mutex<Vec<DisplayEvent>>);
impl DisplaySink for Display {
    fn write_event(&self, event: DisplayEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn astra_streaming_runs_write_then_read_and_preserves_native_history() {
    let reasoning =
        json!({"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"opaque"});
    let mock = Mock::new(vec![
        stream_response(json!([reasoning,{"type":"function_call","id":"fc_1","call_id":"call_write","name":"Write","arguments":"{\"path\":\"note.txt\",\"content\":\"hi 🦀\"}"}])),
        stream_response(json!([{"type":"function_call","id":"fc_2","call_id":"call_read","name":"Read","arguments":"{\"path\":\"note.txt\"}"}])),
        stream_response(json!([text_item("msg_1","Done 🦀")])),
    ]).await;
    let dir = tempfile::tempdir().unwrap();
    let tools = ToolRegistry::with_cwd(Some(dir.path().into()));
    let mut conversation = Conversation::default();
    let display = Arc::new(Display::default());
    let result = run_agent_cancellable(
        &mock.client(Runtime::Eunice),
        "gpt-6-astra",
        "Write and read a note",
        50,
        &tools,
        display.clone(),
        &mut conversation,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.status, AgentStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("note.txt")).unwrap(),
        "hi 🦀"
    );
    let requests = mock.done();
    assert_eq!(requests[0]["body"]["stream"], true);
    assert_eq!(requests[0]["body"]["store"], false);
    assert_eq!(requests[1]["body"]["input"][1], reasoning);
    assert_eq!(requests[1]["body"]["input"][3]["call_id"], "call_write");
    assert_eq!(
        requests[1]["body"]["input"][3]["type"],
        "function_call_output"
    );
    assert_eq!(requests[0]["body"]["tools"][0]["strict"], false);
    assert!(display
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e,DisplayEvent::StreamChunk {content} if content == "Done 🦀")));
}

#[tokio::test]
async fn legacy_requests_strip_native_metadata() {
    let mock = Mock::new(vec![step(
        "POST",
        "/v1/chat/completions",
        json!({"choices":[{"message":{"content":"hello","tool_calls":null}}]}),
    )])
    .await;
    mock.client(Runtime::Eunice)
        .chat_completion(
            "gpt-5.6",
            json!([{"role":"assistant","content":"old","native_output":[{"type":"reasoning"}]}]),
            None,
        )
        .await
        .unwrap();
    assert!(mock.done()[0]["body"]["messages"][0]
        .get("native_output")
        .is_none());
}

#[tokio::test]
async fn responses_eof_cannot_execute_partial_tools() {
    let mock = Mock::new(vec![Step {
        method: "POST",
        path: "/v1/responses",
        status: 200,
        body: "data: {\"type\":\"response.output_text.delta\",\"delta\":\"working\"}\n\n".into(),
        sse: true,
    }])
    .await;
    let err = mock
        .client(Runtime::Eunice)
        .chat_completion_streaming("gpt-6-astra", json!([]), None, |_| {})
        .await
        .unwrap_err();
    assert!(err.to_string().contains("before completion"));
    mock.done();
}

#[tokio::test]
async fn managed_tool_results_are_durable_and_repeated_actions_do_not_execute_twice() {
    let call = action("Bash", json!({"command":"printf x >> count"}));
    let mut steps = vec![step("POST", "/v1/agents/sessions", json!({"id":"sess_1"}))];
    for _ in 0..2 {
        steps.extend([
            step(
                "GET",
                "/v1/agents/sessions/sess_1",
                json!({"status":"requires_action","required_actions":[call]}),
            ),
            step(
                "GET",
                "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
                page(json!([turn("turn_1", "waiting")])),
            ),
            step(
                "GET",
                "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
                page(json!([])),
            ),
            step("POST", "/v1/agents/sessions/sess_1/events", Value::Null),
        ]);
    }
    steps.extend([
        step(
            "GET",
            "/v1/agents/sessions/sess_1",
            json!({"status":"idle","required_actions":[]}),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
            page(json!([turn("turn_1", "completed")])),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
            page(json!([text_item("msg_1", "Done")])),
        ),
    ]);
    let mock = Mock::new(steps).await;
    let dir = tempfile::tempdir().unwrap();
    let tools = ToolRegistry::with_cwd(Some(dir.path().into()));
    let mut conversation = Conversation::default();
    let result = run_agent_cancellable(
        &mock.client(Runtime::OpenaiAgents),
        "gpt-6-astra",
        "Run once",
        50,
        &tools,
        Arc::new(Display::default()),
        &mut conversation,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.status, AgentStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("count")).unwrap(),
        "x"
    );
    assert!(!conversation.managed.active);
    assert!(conversation.managed.journal["turn_1:call_1"]
        .result
        .is_some());
    let requests = mock.done();
    assert!(requests
        .iter()
        .all(|r| r["beta"] == "agents=v1" && r["auth"] == "Bearer test-key"));
    assert_eq!(requests[0]["body"]["environment"], json!({"type":"none"}));
    assert_eq!(requests[4]["body"], requests[8]["body"]);
}

#[tokio::test]
async fn managed_cancel_sends_remote_event_and_confirms_root_outcome() {
    for resumed in [false, true] {
        let mut steps = Vec::new();
        if !resumed {
            steps.push(step("POST", "/v1/agents/sessions", json!({"id":"sess_1"})));
        }
        steps.extend(vec![
            step("POST", "/v1/agents/sessions/sess_1/events", Value::Null),
            step(
                "GET",
                "/v1/agents/sessions/sess_1",
                json!({"status":"idle","required_actions":[]}),
            ),
            step(
                "GET",
                "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
                page(json!([turn("turn_1", "cancelled")])),
            ),
            step(
                "GET",
                "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
                page(json!([])),
            ),
        ]);
        let mock = Mock::new(steps).await;
        let (_tx, rx) = tokio::sync::watch::channel(true);
        let mut conversation = Conversation::default();
        if resumed {
            conversation.managed = serde_json::from_value(json!({"id":"sess_1","model":"gpt-6-astra","endpoint":mock.url,"working_dir":std::env::current_dir().unwrap(),"active":true,"cancel_pending":true})).unwrap();
        }
        let result = run_agent_cancellable(
            &mock.client(Runtime::OpenaiAgents),
            "gpt-6-astra",
            "Stop",
            50,
            &ToolRegistry::new(),
            Arc::new(Display::default()),
            &mut conversation,
            Some(rx),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.status, AgentStatus::Cancelled);
        assert_eq!(
            mock.done()[usize::from(!resumed)]["body"]["events"][0]["type"],
            "agent.session.input.cancel"
        );
    }
}

#[tokio::test]
async fn managed_followup_reuses_session_and_sends_only_new_input() {
    let mut steps = Vec::new();
    for i in 1..=2 {
        steps.push(if i == 1 {
            step("POST", "/v1/agents/sessions", json!({"id":"sess_1"}))
        } else {
            step("POST", "/v1/agents/sessions/sess_1/events", Value::Null)
        });
        steps.push(step(
            "GET",
            "/v1/agents/sessions/sess_1",
            json!({"status":"idle","required_actions":[]}),
        ));
        let turns: Vec<_> = (1..=i)
            .map(|n| turn(&format!("turn_{n}"), "completed"))
            .collect();
        let items: Vec<_> = (1..=i)
            .map(|n| text_item(&format!("msg_{n}"), &format!("answer {n}")))
            .collect();
        steps.push(step(
            "GET",
            "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
            page(json!(turns)),
        ));
        steps.push(step(
            "GET",
            "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
            page(json!(items)),
        ));
    }
    let mock = Mock::new(steps).await;
    let client = mock.client(Runtime::OpenaiAgents);
    let mut conversation = Conversation::default();
    for prompt in ["first", "second"] {
        run_agent_cancellable(
            &client,
            "gpt-6-astra",
            prompt,
            50,
            &ToolRegistry::new(),
            Arc::new(Display::default()),
            &mut conversation,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    }
    assert_eq!(conversation.len(), 4);
    let requests = mock.done();
    assert_eq!(
        requests[4]["body"]["events"][0]["input"][0]["content"][0]["text"],
        "second"
    );
}

#[tokio::test]
async fn managed_auth_failure_does_not_fall_back_or_repeat_creation() {
    let mut failed = step(
        "POST",
        "/v1/agents/sessions",
        json!({"error":"permission denied"}),
    );
    failed.status = 403;
    let mock = Mock::new(vec![failed]).await;
    let client = mock.client(Runtime::OpenaiAgents);
    let mut conversation = Conversation::default();
    let error = run_agent_cancellable(
        &client,
        "gpt-6-astra",
        "hello",
        50,
        &ToolRegistry::new(),
        Arc::new(Display::default()),
        &mut conversation,
        None,
        None,
        None,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("403"));
    assert!(!conversation.managed.uncertain_input);
    assert!(conversation.is_empty());
    mock.done();
}

async fn managed_run(
    mock: &Mock,
    conversation: &mut Conversation,
    prompt: &str,
    tools: &ToolRegistry,
) -> anyhow::Result<eunice::agent::AgentResult> {
    run_agent_cancellable(
        &mock.client(Runtime::OpenaiAgents),
        "gpt-6-astra",
        prompt,
        50,
        tools,
        Arc::new(Display::default()),
        conversation,
        None,
        None,
        None,
    )
    .await
}

#[tokio::test]
async fn ambiguous_creation_is_not_repeated() {
    let mut failed = step(
        "POST",
        "/v1/agents/sessions",
        json!({"error":"upstream lost"}),
    );
    failed.status = 502;
    let mock = Mock::new(vec![failed]).await;
    let mut conversation = Conversation::default();
    assert!(
        managed_run(&mock, &mut conversation, "hello", &ToolRegistry::new())
            .await
            .is_err()
    );
    assert!(conversation.managed.uncertain_input);
    assert!(
        managed_run(&mock, &mut conversation, "hello", &ToolRegistry::new())
            .await
            .unwrap_err()
            .to_string()
            .contains("uncertain outcome")
    );
    mock.done();
}

#[tokio::test]
async fn managed_paginates_past_subagents_and_reports_root_failure() {
    let mock = Mock::new(vec![
        step("POST","/v1/agents/sessions",json!({"id":"sess_1"})),
        step("GET","/v1/agents/sessions/sess_1",json!({"status":"idle","required_actions":[]})),
        step("GET","/v1/agents/sessions/sess_1/turns?order=asc&limit=100",json!({"data":[{"id":"child","subagent_id":"sub_1","status":"completed"}],"has_more":true,"last_id":"child"})),
        step("GET","/v1/agents/sessions/sess_1/turns?order=asc&limit=100&after=child",page(json!([turn("root","failed")]))),
        step("GET","/v1/agents/sessions/sess_1/items?order=asc&limit=100",json!({"data":[],"has_more":true,"last_id":"item_1"})),
        step("GET","/v1/agents/sessions/sess_1/items?order=asc&limit=100&after=item_1",page(json!([]))),
    ]).await;
    let mut conversation = Conversation::default();
    assert!(
        managed_run(&mock, &mut conversation, "hello", &ToolRegistry::new())
            .await
            .unwrap_err()
            .to_string()
            .contains("Managed turn failed")
    );
    assert!(!conversation.managed.active);
    assert!(conversation.managed.finished_turns.contains("root"));
    mock.done();
}

#[tokio::test]
async fn interrupted_tool_claim_is_never_reexecuted_after_reload() {
    let temp = tempfile::tempdir().unwrap();
    let tools = ToolRegistry::with_cwd(Some(temp.path().into()));
    let pending = action("Bash", json!({"command":"printf x >> count"}));
    let mock = Mock::new(vec![
        step(
            "GET",
            "/v1/agents/sessions/sess_1",
            json!({"status":"requires_action","required_actions":[pending]}),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
            page(json!([turn("turn_1", "waiting")])),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
            page(json!([])),
        ),
    ])
    .await;
    let mut conversation = Conversation::default();
    conversation.managed = serde_json::from_value(json!({
        "id":"sess_1","model":"gpt-6-astra","endpoint":mock.url,"working_dir":temp.path(),"active":true,
        "journal":{"turn_1:call_1":{"action":pending,"result":null,"full_output":null}}
    })).unwrap();
    let error = managed_run(&mock, &mut conversation, "", &tools)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("uncertain outcome"));
    assert!(!temp.path().join("count").exists());
    mock.done();
}

#[tokio::test]
async fn unknown_function_returns_error_result_and_allows_turn_to_finish() {
    let mock = Mock::new(vec![
        step("POST", "/v1/agents/sessions", json!({"id":"sess_1"})),
        step(
            "GET",
            "/v1/agents/sessions/sess_1",
            json!({"status":"requires_action","required_actions":[action("missing",json!({}))]}),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
            page(json!([turn("turn_1", "waiting")])),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
            page(json!([])),
        ),
        step("POST", "/v1/agents/sessions/sess_1/events", Value::Null),
        step(
            "GET",
            "/v1/agents/sessions/sess_1",
            json!({"status":"idle","required_actions":[]}),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/turns?order=asc&limit=100",
            page(json!([turn("turn_1", "completed")])),
        ),
        step(
            "GET",
            "/v1/agents/sessions/sess_1/items?order=asc&limit=100",
            page(json!([text_item("msg_1", "unavailable")])),
        ),
    ])
    .await;
    managed_run(
        &mock,
        &mut Conversation::default(),
        "hello",
        &ToolRegistry::new(),
    )
    .await
    .unwrap();
    let requests = mock.done();
    let result = &requests[4]["body"]["events"][0];
    assert_eq!(result["success"], false);
    assert!(result["error"].as_str().unwrap().contains("missing"));
}

#[tokio::test]
async fn incomplete_response_never_executes_partial_tools() {
    let temp = tempfile::tempdir().unwrap();
    let data = json!({"type":"response.incomplete","response":{"status":"incomplete","output":[{"type":"function_call","call_id":"call_1","name":"Write","arguments":"{\"path\":\"bad\",\"content\":\"bad\"}"}]}});
    let mock = Mock::new(vec![Step {
        method: "POST",
        path: "/v1/responses",
        status: 200,
        body: format!("data: {data}\n\n"),
        sse: true,
    }])
    .await;
    let result = run_agent_cancellable(
        &mock.client(Runtime::Eunice),
        "gpt-6-astra",
        "hello",
        50,
        &ToolRegistry::with_cwd(Some(temp.path().into())),
        Arc::new(Display::default()),
        &mut Conversation::default(),
        None,
        None,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(!temp.path().join("bad").exists());
    mock.done();
}

#[tokio::test]
#[ignore = "Paid live Astra/Agents API smoke test; requires EUNICE_LIVE_OPENAI=1 and OPENAI_API_KEY"]
async fn live_openai_two_runtimes() {
    assert_eq!(std::env::var("EUNICE_LIVE_OPENAI").as_deref(), Ok("1"));
    let key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is required");
    for runtime in [Runtime::Eunice, Runtime::OpenaiAgents] {
        let info = ProviderInfo {
            provider: Provider::OpenAI,
            api_key: key.clone(),
            base_url: "https://api.openai.com/v1/".into(),
            resolved_model: "gpt-6-astra".into(),
            use_native_gemini_api: false,
            azure_api_version: None,
        };
        let mut client = Client::new(&info).unwrap();
        client.set_runtime(runtime).unwrap();
        client.set_effort(Some("low".into()));
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("input.txt"), "eunice-smoke-🦀").unwrap();
        let mut history = Conversation::default();
        let result=run_agent_cancellable(&client,"gpt-6-astra","Read input.txt using the Read tool and reply with its exact contents. Use no other tools.",50,&ToolRegistry::with_cwd(Some(temp.path().into())),Arc::new(Display::default()),&mut history,None,None,None).await.unwrap();
        assert_eq!(result.status, AgentStatus::Completed);
        assert!(history.iter().any(|message|matches!(message,eunice::models::Message::Tool {content,..} if content.contains("eunice-smoke-🦀"))));
    }
}
