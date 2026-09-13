use axum::{body::Body, extract::State, http::Response, routing::post, Json, Router};
use eunice::{
    agent,
    client::Client,
    display_sink::{DisplayEvent, DisplaySink},
    models::{Provider, ProviderInfo},
    runtime::Conversation,
    tools::ToolRegistry,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Display(Mutex<Vec<DisplayEvent>>);
impl DisplaySink for Display {
    fn write_event(&self, event: DisplayEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn local_stream_executes_read_and_returns_result_to_model() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app=Router::new().route("/v1/chat/completions",post(|State(requests):State<Arc<Mutex<Vec<Value>>>>,Json(body):Json<Value>|async move {
        let mut history=requests.lock().unwrap();
        assert_eq!(body["stream"],true);
        assert_eq!(body["tool_choice"],"auto");
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["messages"][0]["content"].as_str().unwrap().contains("terminal agent"));
        assert_eq!(body["messages"].as_array().unwrap().iter().filter(|m| m["role"]=="system").count(), 1);
        assert!(body["tools"].as_array().unwrap().iter().any(|t|t["function"]["name"]=="Read"));
        let chunks=if history.is_empty() {
            vec![json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"read_1","type":"function","function":{"name":"Read","arguments":"{\"path\":"}}]}}]}),
                 json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"notes.txt\"}"}}]},"finish_reason":"tool_calls"}]})]
        } else {
            assert_eq!(body["messages"][2]["tool_calls"][0]["id"],"read_1");
            assert!(body["messages"].as_array().unwrap().iter().any(|m|m["role"]=="tool" && m["tool_call_id"]=="read_1" && m["content"].as_str().unwrap().contains("PHOSPHOR")));
            vec![json!({"choices":[{"index":0,"delta":{"content":"The note says PHOSPHOR."},"finish_reason":"stop"}]})]
        };
        history.push(body);
        let mut sse=chunks.into_iter().map(|v|format!("data: {v}\r\n\r\n")).collect::<String>();
        sse.push_str("data: [DONE]\r\n\r\n");
        Response::builder().header("content-type","text/event-stream").body(Body::from_stream(futures::stream::iter(sse.into_bytes().into_iter().map(|b|Ok::<_,std::io::Error>(vec![b]))))).unwrap()
    })).with_state(requests.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::new(&ProviderInfo {
        provider: Provider::Local,
        base_url: format!("http://{}/v1/", listener.local_addr().unwrap()),
        api_key: "local".into(),
        resolved_model: "Qwen3.5-2B-Q4_K_M".into(),
        use_native_gemini_api: false,
        azure_api_version: None,
    })
    .unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "PHOSPHOR").unwrap();
    let registry = ToolRegistry::with_cwd(Some(dir.path().into()));
    let display = Arc::new(Display::default());
    let mut conversation = Conversation::default();
    let result = agent::run_agent_cancellable(
        &client,
        "Qwen3.5-2B-Q4_K_M",
        "Read notes.txt",
        50,
        &registry,
        display.clone(),
        &mut conversation,
        None,
        None,
        None,
    )
    .await;
    task.abort();
    result.unwrap();
    assert_eq!(conversation.turn_stats.calls, 2);
    assert_eq!(conversation.turn_stats.responses, 2);
    assert_eq!(requests.lock().unwrap().len(), 2);
    let events = display.0.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e,DisplayEvent::ToolCall{name,..} if name=="Read")));
    assert!(events
        .iter()
        .any(|e| matches!(e,DisplayEvent::StreamChunk{content} if content.contains("PHOSPHOR"))));
}

#[tokio::test]
async fn cancelling_local_stream_never_executes_an_unfinished_write() {
    let ready = Arc::new(tokio::sync::Notify::new());
    let app=Router::new().route("/v1/chat/completions",post(|State(ready):State<Arc<tokio::sync::Notify>>|async move {
        use futures::StreamExt;
        let value=json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"partial","type":"function","function":{"name":"Write","arguments":"{\"path\":\"must-not-exist.txt\",\"content\":\"oops\"}"}}]}}]});
        let first=futures::stream::once(async move {Ok::<_,std::io::Error>(format!("data: {value}\n\n"))});
        ready.notify_one();
        Response::builder().header("content-type","text/event-stream").body(Body::from_stream(first.chain(futures::stream::pending()))).unwrap()
    })).with_state(ready.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::new(&ProviderInfo {
        provider: Provider::Local,
        base_url: format!("http://{}/v1/", listener.local_addr().unwrap()),
        api_key: "local".into(),
        resolved_model: "Qwen3.5-2B-Q4_K_M".into(),
        use_native_gemini_api: false,
        azure_api_version: None,
    })
    .unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (tx, rx) = tokio::sync::watch::channel(false);
    let cancellation = tokio::spawn(async move {
        ready.notified().await;
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        tx.send(true).unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let registry = ToolRegistry::with_cwd(Some(dir.path().into()));
    let mut history = Conversation::default();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        agent::run_agent_cancellable(
            &client,
            "Qwen3.5-2B-Q4_K_M",
            "Write a file",
            50,
            &registry,
            Arc::new(Display::default()),
            &mut history,
            Some(rx),
            None,
            None,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    task.abort();
    cancellation.await.unwrap();
    assert_eq!(result.status, agent::AgentStatus::Cancelled);
    assert!(!dir.path().join("must-not-exist.txt").exists());
}

#[tokio::test]
async fn local_stream_retries_a_connection_closed_before_response() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for attempt in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut input = Vec::new();
            loop {
                let mut chunk = [0u8; 4096];
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                input.extend_from_slice(&chunk[..n]);
                if let Some(end) = input.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&input[..end]).to_lowercase();
                    let length: usize = header.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().parse().unwrap();
                    if input.len() >= end + 4 + length { break; }
                }
            }
            if attempt == 0 { continue; } // Reproduce a server closing before headers.
            let payload = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Recovered.\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", payload.len(), payload);
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let client = Client::new(&ProviderInfo {
        provider: Provider::Local,
        base_url: format!("http://{address}/v1/"),
        api_key: "local".into(),
        resolved_model: "Qwen3.5-2B-Q4_K_M".into(),
        use_native_gemini_api: false,
        azure_api_version: None,
    }).unwrap();
    let mut visible = String::new();
    tokio::time::timeout(std::time::Duration::from_secs(5), client.chat_completion_streaming(
        "Qwen3.5-2B-Q4_K_M", json!([{"role":"user","content":"Hello"}]), None,
        |chunk| visible.push_str(chunk),
    )).await.unwrap().unwrap();
    assert_eq!(visible, "Recovered.");
    server.await.unwrap();
}

#[tokio::test]
async fn nonstreaming_local_qwen_also_receives_agent_role_and_all_tools() {
    let app = Router::new().route("/v1/chat/completions", post(|Json(body): Json<Value>| async move {
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["messages"][0]["content"].as_str().unwrap().contains("terminal agent"));
        assert_eq!(body["messages"][1]["content"], "what directory is this");
        let names: Vec<_> = body["tools"].as_array().unwrap().iter().map(|t| t["function"]["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Bash", "Read", "Write", "Skill"]);
        assert_eq!(body["tool_choice"], "auto");
        Json(json!({"choices":[{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}]}))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::new(&ProviderInfo {
        provider: Provider::Local, base_url: format!("http://{}/v1/", listener.local_addr().unwrap()),
        api_key: "local".into(), resolved_model: "Qwen3.5-2B-Q4_K_M".into(),
        use_native_gemini_api: false, azure_api_version: None,
    }).unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    client.chat_completion("Qwen3.5-2B-Q4_K_M", json!([{"role":"user","content":"what directory is this"}]), Some(&ToolRegistry::new().get_tools())).await.unwrap();
    task.abort();
}

#[tokio::test]
async fn compaction_is_atomic_and_preserves_explicit_instructions() {
    use eunice::models::Message;
    let app = Router::new().route("/v1/chat/completions", post(|Json(body): Json<Value>| async move {
        assert!(body.get("tools").is_none_or(|v| v.is_null()));
        Json(json!({"choices":[{"message":{"role":"assistant","content":"The user is reviewing files. The secret is PHOSPHOR."}}]}))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::new(&ProviderInfo {
        provider: Provider::Local, base_url: format!("http://{}/v1/", listener.local_addr().unwrap()),
        api_key:"local".into(), resolved_model:"Qwen3.5-2B-Q4_K_M".into(), use_native_gemini_api:false, azure_api_version:None,
    }).unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let mut history: Conversation = vec![Message::User { content: "Detailed review notes. ".repeat(500) }].into();
    history.turn_stats.calls = 3;
    eunice::compact::compact_manually(&client, "Qwen3.5-2B-Q4_K_M", &mut history, Some("Never modify files.")).await.unwrap();
    assert_eq!(history.compactions, 1);
    assert_eq!(history.turn_stats.calls, 3);
    let text = serde_json::to_string(&history).unwrap();
    assert!(text.contains("Never modify files.") && text.contains("PHOSPHOR"));
    // An empty/smaller conversation cannot grow from compaction.
    history.clear(); history.push(Message::User { content:"Hi".into() });
    let before = serde_json::to_value(&history).unwrap();
    eunice::compact::compact_manually(&client, "Qwen3.5-2B-Q4_K_M", &mut history, None).await.unwrap();
    assert_eq!(serde_json::to_value(&history).unwrap(), before);
    assert_eq!(history.compactions, 0);
    task.abort();
    let result = eunice::compact::compact_manually(&client, "Qwen3.5-2B-Q4_K_M", &mut history, None).await;
    assert!(result.is_err());
    assert_eq!(serde_json::to_value(&history).unwrap(), before);
}

#[tokio::test]
async fn cancelling_manual_compaction_preserves_history() {
    use eunice::models::Message;
    let ready = Arc::new(tokio::sync::Notify::new());
    let app = Router::new().route("/v1/chat/completions", post(|State(ready): State<Arc<tokio::sync::Notify>>| async move {
        ready.notify_one();
        std::future::pending::<Json<Value>>().await
    })).with_state(ready.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::new(&ProviderInfo {
        provider:Provider::Local, base_url:format!("http://{}/v1/",listener.local_addr().unwrap()), api_key:"local".into(), resolved_model:"Qwen3.5-2B-Q4_K_M".into(),use_native_gemini_api:false,azure_api_version:None,
    }).unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let mut history: Conversation = vec![Message::User { content:"Preserve these instructions".into() }].into();
    let before = serde_json::to_value(&history).unwrap();
    tokio::select! {
        _ = ready.notified() => {},
        r = eunice::compact::compact_manually(&client,"Qwen3.5-2B-Q4_K_M",&mut history,None) => panic!("unexpected completion: {r:?}"),
    }
    assert_eq!(serde_json::to_value(&history).unwrap(),before);
    assert_eq!(history.compactions,0);
    task.abort();
}

#[tokio::test]
async fn automatic_compaction_counts_successful_cycles_and_retry_rounds() {
    use eunice::{compact::CompactionConfig, models::Message};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let attempts = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route("/v1/chat/completions",post(|State(count):State<Arc<AtomicUsize>>|async move {
        if count.fetch_add(1,Ordering::SeqCst)==0 {
            Response::builder().status(400).body(Body::from("request exceeds context window; n_ctx is 4096")).unwrap()
        } else {
            Response::builder().header("content-type","text/event-stream").body(Body::from("data: {\"choices\":[{\"delta\":{\"content\":\"Recovered\"},\"finish_reason\":\"stop\"}],\"usage\":{\"completion_tokens\":10},\"timings\":{\"predicted_n\":10,\"predicted_ms\":1000}}\n\ndata: [DONE]\n\n")).unwrap()
        }
    })).with_state(attempts.clone());
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client=Client::new(&ProviderInfo{provider:Provider::Local,base_url:format!("http://{}/v1/",listener.local_addr().unwrap()),api_key:"local".into(),resolved_model:"Qwen3.5-2B-Q4_K_M".into(),use_native_gemini_api:false,azure_api_version:None}).unwrap();
    let task=tokio::spawn(async move{axum::serve(listener,app).await.unwrap();});
    let mut history:Conversation=vec![Message::User{content:"Old verbose context ".repeat(1000)}].into();
    history.compactions=2;
    agent::run_agent_cancellable(&client,"Qwen3.5-2B-Q4_K_M","Continue",50,&ToolRegistry::new(),Arc::new(Display::default()),&mut history,None,Some(CompactionConfig::default()),None).await.unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst),2);
    assert_eq!(history.turn_stats.calls,2);
    assert_eq!(history.compactions,3);
    assert_eq!(history.turn_stats.generation_tokens,10);
    task.abort();
}
