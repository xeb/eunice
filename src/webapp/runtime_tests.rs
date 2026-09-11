//! Exercise the binary's web runtime, which is distinct from the CLI module tree.
use super::{handlers::*, persistence::SessionStorage, server::AppState};
use crate::{
    client::Client,
    models::{Provider, ProviderInfo},
    runtime::Runtime,
    tools::ToolRegistry,
};
use axum::{
    body::to_bytes,
    extract::{Request, State},
    http::HeaderMap,
    response::IntoResponse,
    Json, Router,
};
use serde_json::{json, Value};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{broadcast, mpsc, watch, Mutex};

pub(super) fn app(storage: SessionStorage, url: String, cwd: Option<std::path::PathBuf>) -> Arc<AppState> {
    let info = ProviderInfo {
        provider: Provider::OpenAI,
        api_key: "fixture".into(),
        base_url: url,
        resolved_model: "gpt-6-astra".into(),
        use_native_gemini_api: false,
        azure_api_version: None,
    };
    let mut client = Client::new(&info).unwrap();
    client.set_runtime(Runtime::OpenaiAgents).unwrap();
    Arc::new(AppState {
        client: Arc::new(client),
        provider_info: info,
        tool_registry: Arc::new(ToolRegistry::with_cwd(cwd)),
        tool_output_limit: 50,
        cancellations: Default::default(),
        session_locks: Default::default(),
        cancel_tx: Default::default(),
        storage,
        system_prompt: Some("Project instructions".into()),
        agents: None,
    })
}
fn page(items: Value) -> Value {
    json!({"data":items,"has_more":false})
}
async fn run(state: Arc<AppState>, id: &str, prompt: &str) -> Result<Vec<SseEvent>, String> {
    let (tx, mut rx) = mpsc::channel(100);
    let collect = tokio::spawn(async move {
        let mut all = Vec::new();
        while let Some(event) = rx.recv().await {
            all.push(event);
        }
        all
    });
    let (btx, _) = broadcast::channel(100);
    let (_cancel, rx_cancel) = watch::channel(false);
    let sender = EventSender::new(tx, state.clone(), id.into(), btx);
    run_agent_with_events(
        state,
        prompt.into(),
        id.into(),
        "test".into(),
        sender,
        rx_cancel,
        None,
        false,
    )
    .await?;
    Ok(collect.await.unwrap())
}

#[tokio::test]
async fn web_recovers_committed_tool_result_from_sqlite_without_reexecution() {
    let temp = tempfile::tempdir().unwrap();
    let action = json!({"type":"function_call","turn_id":"turn_1","call_id":"call_1","name":"Bash","arguments":{"command":"printf x >> count"}});
    let waiting = page(json!([{"id":"turn_1","subagent_id":null,"status":"waiting"}]));
    let pending = json!({"status":"requires_action","required_actions":[action]});
    let mut steps = VecDeque::from(vec![
        (200, json!({"id":"sess_web"})),
        (200, pending.clone()),
        (200, waiting.clone()),
        (200, page(json!([]))),
        (502, json!({"error":"lost tool acknowledgement"})),
        (200, pending),
        (200, waiting),
        (200, page(json!([]))),
        (200, Value::Null),
        (200, json!({"status":"idle","required_actions":[]})),
        (
            200,
            page(json!([{"id":"turn_1","subagent_id":null,"status":"completed"}])),
        ),
        (
            200,
            page(
                json!([{"id":"msg_1","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Done 🦀"}]}]),
            ),
        ),
    ]);
    // The second prompt is sent to the same managed session after restart.
    steps.extend(vec![(200,Value::Null),(200,json!({"status":"idle","required_actions":[]})),
        (200,page(json!([{"id":"turn_1","subagent_id":null,"status":"completed"},{"id":"turn_2","subagent_id":null,"status":"completed"}]))),
        (200,page(json!([{"id":"msg_1","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Done 🦀"}]},{"id":"msg_2","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Followup"}]}])))]);
    let steps = Arc::new(Mutex::new(steps));
    let remaining = steps.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let router = Router::new().fallback(move |req: Request| {
        let steps = steps.clone();
        let requests = requests.clone();
        async move {
            let method = req.method().to_string();
            let path = req.uri().to_string();
            let bytes = to_bytes(req.into_body(), 1_000_000).await.unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            requests
                .lock()
                .await
                .push(json!({"method":method,"path":path,"body":body}));
            let (status, body) = steps.lock().await.pop_front().expect("unexpected request");
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                Json(body),
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let db = temp.path().join("sessions.db");
    let store = SessionStorage::new_sqlite(db.to_str().unwrap()).unwrap();
    let session = store.create_agent_session("scheduled").await.unwrap();
    let state = app(store, url.clone(), Some(temp.path().into()));
    assert!(run(state.clone(), &session.id, "write once")
        .await
        .unwrap_err()
        .contains("502"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("count")).unwrap(),
        "x"
    );
    assert!(state
        .storage
        .has_pending_managed_agent("scheduled")
        .await
        .unwrap());
    drop(state);
    let state = app(
        SessionStorage::new_sqlite(db.to_str().unwrap()).unwrap(),
        url,
        Some(temp.path().into()),
    );
    let events = run(state.clone(), &session.id, "").await.unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e,SseEvent::Response {content} if content == "Done 🦀")));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("count")).unwrap(),
        "x"
    );
    assert!(!state
        .storage
        .has_pending_managed_agent("scheduled")
        .await
        .unwrap());
    run(state.clone(), &session.id, "followup").await.unwrap();
    let history = state.storage.get_history(&session.id).await.unwrap();
    assert_eq!(history.len(), 6);
    let serialized = serde_json::to_string(&history).unwrap();
    assert_eq!(serialized.matches("Project instructions").count(), 1);
    let requests = captured.lock().await;
    assert_eq!(
        requests
            .iter()
            .filter(|r| r["path"] == "/v1/agents/sessions")
            .count(),
        1
    );
    assert_eq!(requests[4]["body"], requests[8]["body"]);
    assert_eq!(
        requests[12]["body"]["events"][0]["input"][0]["content"][0]["text"],
        "followup"
    );
    assert!(remaining.lock().await.is_empty());
    server.abort();
}

#[tokio::test]
async fn concurrent_query_keeps_original_cancellation_and_events() {
    let state = app(
        SessionStorage::new_memory(),
        "http://127.0.0.1:1/v1/".into(),
        None,
    );
    let session = state.storage.create_session(None).await.unwrap();
    let lock = Arc::new(Mutex::new(()));
    state
        .session_locks
        .lock()
        .await
        .insert(session.id.clone(), lock.clone());
    let _lease = lock.lock().await;
    let (tx, rx) = watch::channel(false);
    state
        .cancellations
        .lock()
        .await
        .insert(session.id.clone(), tx);
    state
        .storage
        .set_runtime_state(
            &session.id,
            vec![SseEvent::Info {
                message: "keep".into(),
            }],
            None,
            true,
        )
        .await;
    let request: QueryRequest =
        serde_json::from_value(json!({"prompt":"duplicate","session_id":session.id})).unwrap();
    let response = query(HeaderMap::new(), State(state.clone()), Json(request))
        .await
        .into_response();
    let body = to_bytes(response.into_body(), 10000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("already running"));
    assert_eq!(
        state
            .storage
            .get_runtime_state(&session.id)
            .await
            .unwrap()
            .0
            .len(),
        1
    );
    let cancel_request = serde_json::from_value(json!({"session_id":session.id})).unwrap();
    let _ = cancel(State(state), HeaderMap::new(), Some(Json(cancel_request))).await;
    assert!(*rx.borrow());
}

#[tokio::test]
async fn cancellation_is_scoped_to_session_owner() {
    let state = app(
        SessionStorage::new_memory(),
        "http://127.0.0.1:1/v1/".into(),
        None,
    );
    let a = state.storage.create_session(Some("alice")).await.unwrap();
    let b = state.storage.create_session(Some("bob")).await.unwrap();
    let (tx_a, rx_a) = watch::channel(false);
    let (tx_b, rx_b) = watch::channel(false);
    state
        .cancellations
        .lock()
        .await
        .extend([(a.id.clone(), tx_a), (b.id, tx_b)]);
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-user", "bob".parse().unwrap());
    let request = || {
        Some(Json(
            serde_json::from_value(json!({"session_id":a.id})).unwrap(),
        ))
    };
    assert_eq!(
        cancel(State(state.clone()), headers.clone(), request())
            .await
            .0["cancelled"],
        false
    );
    headers.insert("x-forwarded-user", "alice".parse().unwrap());
    assert_eq!(
        cancel(State(state.clone()), headers, request()).await.0["cancelled"],
        true
    );
    assert!(*rx_a.borrow());
    assert!(!*rx_b.borrow());
}
