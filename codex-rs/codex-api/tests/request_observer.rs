#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use codex_api::AuthProvider;
use codex_api::Compression;
use codex_api::HttpRequestObservation;
use codex_api::Provider;
use codex_api::ResponseCreateWsRequest;
use codex_api::ResponseEvent;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesClient;
use codex_api::ResponsesEndpoint;
use codex_api::ResponsesOptions;
use codex_api::ResponsesRequestObserver;
use codex_api::ResponsesWebsocketClient;
use codex_api::ResponsesWsRequest;
use codex_api::SafeTransportIdentity;
use codex_api::WebsocketRequestObservation;
use codex_client::HttpTransport;
use codex_client::Request;
use codex_client::RequestBody;
use codex_client::Response;
use codex_client::StreamResponse;
use codex_client::TransportError;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use futures::SinkExt;
use futures::StreamExt;
use http::HeaderMap;
use http::HeaderValue;
use http::StatusCode;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async_with_config;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::extensions::ExtensionsConfig;
use tokio_tungstenite::tungstenite::extensions::compression::deflate::DeflateConfig;
use tokio_tungstenite::tungstenite::handshake::server::Request as HandshakeRequest;
use tokio_tungstenite::tungstenite::handshake::server::Response as HandshakeResponse;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

const RESPONSES_LITE_HEADER: &str = "x-openai-internal-codex-responses-lite";
const RESPONSES_LITE_METADATA_KEY: &str =
    "ws_request_header_x_openai_internal_codex_responses_lite";

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedTransportIdentity {
    session_id: Option<Vec<u8>>,
    thread_id: Option<Vec<u8>>,
    client_request_id: Option<Vec<u8>>,
    subagent: Option<Vec<u8>>,
    routing_hint: Option<Vec<u8>>,
    responses_lite: Option<Vec<u8>>,
    previous_response_id: Option<Vec<u8>>,
    originator: Option<Vec<u8>>,
    user_agent: Option<Vec<u8>>,
}

impl From<SafeTransportIdentity<'_>> for OwnedTransportIdentity {
    fn from(identity: SafeTransportIdentity<'_>) -> Self {
        Self {
            session_id: identity.session_id.map(<[u8]>::to_vec),
            thread_id: identity.thread_id.map(<[u8]>::to_vec),
            client_request_id: identity.client_request_id.map(<[u8]>::to_vec),
            subagent: identity.subagent.map(<[u8]>::to_vec),
            routing_hint: identity.routing_hint.map(<[u8]>::to_vec),
            responses_lite: identity.responses_lite.map(<[u8]>::to_vec),
            previous_response_id: identity.previous_response_id.map(<[u8]>::to_vec),
            originator: identity.originator.map(<[u8]>::to_vec),
            user_agent: identity.user_agent.map(<[u8]>::to_vec),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedHttpObservation {
    uncompressed_json: Vec<u8>,
    prepared_body: Vec<u8>,
    endpoint: ResponsesEndpoint,
    compression: Compression,
    identity: OwnedTransportIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedWebsocketObservation {
    wire_json: Vec<u8>,
    endpoint: ResponsesEndpoint,
    identity: OwnedTransportIdentity,
    connection_reused: bool,
    incremental: bool,
}

#[derive(Default)]
struct RecordingObserver {
    http: Mutex<Vec<OwnedHttpObservation>>,
    websocket: Mutex<Vec<OwnedWebsocketObservation>>,
}

impl RecordingObserver {
    fn take_http(&self) -> Vec<OwnedHttpObservation> {
        std::mem::take(&mut *self.http.lock().expect("HTTP observations lock"))
    }

    fn take_websocket(&self) -> Vec<OwnedWebsocketObservation> {
        std::mem::take(&mut *self.websocket.lock().expect("WebSocket observations lock"))
    }
}

impl ResponsesRequestObserver for RecordingObserver {
    fn observe_http(&self, observation: HttpRequestObservation<'_>) {
        self.http
            .lock()
            .expect("HTTP observations lock")
            .push(OwnedHttpObservation {
                uncompressed_json: observation.uncompressed_json.to_vec(),
                prepared_body: observation.prepared_body.to_vec(),
                endpoint: observation.endpoint,
                compression: observation.compression,
                identity: observation.identity.into(),
            });
    }

    fn observe_websocket(&self, observation: WebsocketRequestObservation<'_>) {
        self.websocket
            .lock()
            .expect("WebSocket observations lock")
            .push(OwnedWebsocketObservation {
                wire_json: observation.wire_json.to_vec(),
                endpoint: observation.endpoint,
                identity: observation.identity.into(),
                connection_reused: observation.connection_reused,
                incremental: observation.incremental,
            });
    }
}

#[derive(Default)]
struct RetryTransportState {
    attempts: usize,
    requests: Vec<Request>,
}

#[derive(Clone, Default)]
struct RetryRecordingTransport {
    state: Arc<Mutex<RetryTransportState>>,
}

impl RetryRecordingTransport {
    fn take_requests(&self) -> Vec<Request> {
        std::mem::take(
            &mut self
                .state
                .lock()
                .expect("retry transport state lock")
                .requests,
        )
    }
}

impl HttpTransport for RetryRecordingTransport {
    async fn execute(&self, _request: Request) -> Result<Response, TransportError> {
        Err(TransportError::Build("execute should not run".to_string()))
    }

    async fn stream(&self, request: Request) -> Result<StreamResponse, TransportError> {
        let mut state = self.state.lock().expect("retry transport state lock");
        state.attempts += 1;
        state.requests.push(request);
        if state.attempts == 1 {
            return Err(TransportError::Network("retry once".to_string()));
        }
        Ok(StreamResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            bytes: Box::pin(futures::stream::empty::<Result<Bytes, TransportError>>()),
        })
    }
}

#[derive(Clone)]
struct StaticAuth;

impl AuthProvider for StaticAuth {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        headers.insert(
            http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer observer-must-not-see-this"),
        );
        headers.insert(
            "chatgpt-account-id",
            HeaderValue::from_static("account-observer-must-not-see"),
        );
    }
}

fn provider(base_url: String) -> Provider {
    let mut headers = HeaderMap::new();
    headers.insert("session-id", HeaderValue::from_static("provider-session"));
    headers.insert(
        "x-codex-routing-hint",
        HeaderValue::from_static("provider-routing"),
    );
    headers.insert(
        "originator",
        HeaderValue::from_static("provider-originator"),
    );
    headers.insert(
        "user-agent",
        HeaderValue::from_static("provider-user-agent"),
    );
    headers.insert(
        "x-private-provider-secret",
        HeaderValue::from_static("provider-secret"),
    );

    Provider {
        name: "observer-test".to_string(),
        base_url,
        query_params: None,
        headers,
        retry: codex_api::RetryConfig {
            max_attempts: 1,
            base_delay: Duration::ZERO,
            retry_429: false,
            retry_5xx: false,
            retry_transport: true,
        },
        stream_idle_timeout: Duration::from_secs(5),
    }
}

fn api_request() -> ResponsesApiRequest {
    ResponsesApiRequest {
        model: "gpt-test".to_string(),
        instructions: "observer fixture".to_string(),
        input: Vec::new(),
        tools: None,
        tool_choice: "auto".to_string(),
        parallel_tool_calls: false,
        reasoning: None,
        store: false,
        stream: true,
        stream_options: None,
        include: Vec::new(),
        service_tier: None,
        prompt_cache_key: None,
        text: None,
        client_metadata: None,
        access_programs: None,
    }
}

fn body_bytes(request: &Request) -> &[u8] {
    let Some(RequestBody::EncodedJson(body)) = request.body.as_ref() else {
        panic!("expected encoded JSON body");
    };
    body.as_bytes()
}

fn bytes(value: &str) -> Option<Vec<u8>> {
    Some(value.as_bytes().to_vec())
}

#[tokio::test]
async fn http_observer_sees_one_exact_prepared_request_before_retry_cloning() -> Result<()> {
    const EXPECTED_JSON: &[u8] = br#"{"model":"gpt-test","instructions":"observer fixture","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[]}"#;

    let transport = RetryRecordingTransport::default();
    let observer = Arc::new(RecordingObserver::default());
    let client = ResponsesClient::new(
        transport.clone(),
        provider("https://example.com/v1".to_string()),
        Arc::new(StaticAuth),
    )
    .with_endpoint(ResponsesEndpoint::Guardian)
    .with_request_observer(observer.clone());

    let mut extra_headers = HeaderMap::new();
    extra_headers.insert(
        "x-codex-routing-hint",
        HeaderValue::from_static("request-routing"),
    );
    extra_headers.insert("originator", HeaderValue::from_static("request-originator"));
    extra_headers.insert(RESPONSES_LITE_HEADER, HeaderValue::from_static("true"));
    extra_headers.insert(
        "x-private-request-secret",
        HeaderValue::from_static("request-secret"),
    );

    let _stream = client
        .stream_request(
            api_request(),
            ResponsesOptions {
                session_id: Some("request-session".to_string()),
                thread_id: Some("request-thread".to_string()),
                session_source: Some(SessionSource::SubAgent(SubAgentSource::Review)),
                extra_headers,
                compression: Compression::Zstd,
                turn_state: None,
            },
        )
        .await?;

    let sent = transport.take_requests();
    let [first_attempt, retry_attempt] = sent.as_slice() else {
        panic!(
            "expected an initial request and one retry, got {}",
            sent.len()
        );
    };
    assert_eq!(body_bytes(first_attempt), body_bytes(retry_attempt));
    assert_eq!(
        body_bytes(first_attempt).as_ptr(),
        body_bytes(retry_attempt).as_ptr(),
        "retry clones should share the prepared body allocation"
    );
    assert_ne!(body_bytes(first_attempt), EXPECTED_JSON);
    assert_eq!(
        first_attempt.headers.get(http::header::CONTENT_ENCODING),
        Some(&HeaderValue::from_static("zstd"))
    );
    assert_eq!(
        first_attempt.headers.get(http::header::AUTHORIZATION),
        Some(&HeaderValue::from_static(
            "Bearer observer-must-not-see-this"
        ))
    );
    assert_eq!(
        first_attempt.headers.get("x-private-provider-secret"),
        Some(&HeaderValue::from_static("provider-secret"))
    );
    assert_eq!(
        first_attempt.headers.get("x-private-request-secret"),
        Some(&HeaderValue::from_static("request-secret"))
    );

    assert_eq!(
        observer.take_http(),
        vec![OwnedHttpObservation {
            uncompressed_json: EXPECTED_JSON.to_vec(),
            prepared_body: body_bytes(first_attempt).to_vec(),
            endpoint: ResponsesEndpoint::Guardian,
            compression: Compression::Zstd,
            identity: OwnedTransportIdentity {
                session_id: bytes("request-session"),
                thread_id: bytes("request-thread"),
                client_request_id: bytes("request-thread"),
                subagent: bytes("review"),
                routing_hint: bytes("request-routing"),
                responses_lite: bytes("true"),
                previous_response_id: None,
                originator: bytes("request-originator"),
                user_agent: bytes("provider-user-agent"),
            },
        }]
    );
    Ok(())
}

async fn spawn_websocket_server() -> (
    String,
    Arc<Mutex<Option<HeaderMap>>>,
    tokio::task::JoinHandle<Vec<Vec<u8>>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind WebSocket server");
    let address = listener.local_addr().expect("WebSocket server address");
    let handshake_headers = Arc::new(Mutex::new(None));
    let recorded_headers = Arc::clone(&handshake_headers);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept WebSocket client");
        let mut extensions = ExtensionsConfig::default();
        extensions.permessage_deflate = Some(DeflateConfig::default());
        let mut config = WebSocketConfig::default();
        config.extensions = extensions;
        let mut websocket = accept_hdr_async_with_config(
            stream,
            move |request: &HandshakeRequest, response: HandshakeResponse| {
                *recorded_headers.lock().expect("handshake headers lock") =
                    Some(request.headers().clone());
                Ok(response)
            },
            Some(config),
        )
        .await
        .expect("complete WebSocket handshake");

        let mut frames = Vec::new();
        let first_create = websocket
            .next()
            .await
            .expect("receive first request frame")
            .expect("valid first request frame")
            .into_text()
            .expect("text first request frame");
        frames.push(first_create.as_bytes().to_vec());
        websocket
            .send(Message::Text(
                r#"{"type":"response.created","response":{"id":"resp-1"}}"#.into(),
            ))
            .await
            .expect("send first response.created");

        let interrupt = websocket
            .next()
            .await
            .expect("receive interrupt frame")
            .expect("valid interrupt frame")
            .into_text()
            .expect("text interrupt frame");
        frames.push(interrupt.as_bytes().to_vec());
        websocket
            .send(Message::Text(
                r#"{"type":"response.completed","response":{"id":"resp-1","usage":{"input_tokens":0,"input_tokens_details":null,"output_tokens":0,"output_tokens_details":null,"total_tokens":0}}}"#
                    .into(),
            ))
            .await
            .expect("send first response.completed");

        let second_create = websocket
            .next()
            .await
            .expect("receive second request frame")
            .expect("valid second request frame")
            .into_text()
            .expect("text second request frame");
        frames.push(second_create.as_bytes().to_vec());
        websocket
            .send(Message::Text(
                r#"{"type":"response.created","response":{"id":"resp-2"}}"#.into(),
            ))
            .await
            .expect("send second response.created");
        websocket
            .send(Message::Text(
                r#"{"type":"response.completed","response":{"id":"resp-2","usage":{"input_tokens":0,"input_tokens_details":null,"output_tokens":0,"output_tokens_details":null,"total_tokens":0}}}"#
                    .into(),
            ))
            .await
            .expect("send second response.completed");
        frames
    });

    (format!("http://{address}"), handshake_headers, server)
}

async fn interrupt_after_created(stream: &mut codex_api::ResponseStream) -> Result<()> {
    let interrupt = stream.interrupt.take().expect("interrupt sender");
    while let Some(event) = stream.next().await {
        if matches!(event?, ResponseEvent::Created { .. }) {
            interrupt.send(()).expect("request interrupt");
            return Ok(());
        }
    }
    panic!("response stream ended before response.created");
}

async fn drain_completed(mut stream: codex_api::ResponseStream) -> Result<()> {
    while let Some(event) = stream.next().await {
        if matches!(event?, ResponseEvent::Completed { .. }) {
            return Ok(());
        }
    }
    panic!("response stream ended before completion");
}

#[tokio::test]
async fn websocket_observers_are_send_scoped_and_ignore_interrupt_frames() -> Result<()> {
    const FIRST_WIRE_JSON: &[u8] = br#"{"type":"response.create","model":"gpt-test","instructions":"observer fixture","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[]}"#;
    const SECOND_WIRE_JSON: &[u8] = br#"{"type":"response.create","model":"gpt-test","instructions":"observer fixture","previous_response_id":"resp-1","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[],"client_metadata":{"ws_request_header_x_openai_internal_codex_responses_lite":"true"}}"#;

    let (base_url, handshake_headers, server) = spawn_websocket_server().await;
    let first_observer = Arc::new(RecordingObserver::default());
    let second_observer = Arc::new(RecordingObserver::default());
    let client = ResponsesWebsocketClient::new(provider(base_url), Arc::new(StaticAuth))
        .with_endpoint(ResponsesEndpoint::GuardianClassifier);

    let mut extra_headers = HeaderMap::new();
    extra_headers.insert(
        "x-codex-routing-hint",
        HeaderValue::from_static("request-routing"),
    );
    extra_headers.insert("user-agent", HeaderValue::from_static("request-user-agent"));
    extra_headers.insert(
        "x-private-request-secret",
        HeaderValue::from_static("request-secret"),
    );
    let mut default_headers = HeaderMap::new();
    default_headers.insert("session-id", HeaderValue::from_static("default-session"));
    default_headers.insert("thread-id", HeaderValue::from_static("default-thread"));
    default_headers.insert(
        "x-client-request-id",
        HeaderValue::from_static("default-client-request"),
    );
    default_headers.insert(
        "x-openai-subagent",
        HeaderValue::from_static("default-subagent"),
    );
    default_headers.insert(
        "x-codex-routing-hint",
        HeaderValue::from_static("default-routing"),
    );
    default_headers.insert("originator", HeaderValue::from_static("default-originator"));

    let connection = client
        .connect(
            &HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            extra_headers,
            default_headers,
            /*turn_state*/ None,
            /*telemetry*/ None,
        )
        .await?;
    let request = api_request();

    let mut first = connection
        .stream_request_observed(
            ResponsesWsRequest::ResponseCreate(ResponseCreateWsRequest::from(&request)),
            /*connection_reused*/ false,
            /*turn_state*/ None,
            first_observer.clone(),
        )
        .await?;
    interrupt_after_created(&mut first).await?;
    drain_completed(first).await?;

    let mut incremental = ResponseCreateWsRequest::from(&request);
    incremental.previous_response_id = Some("resp-1".to_string());
    incremental.client_metadata = Some(HashMap::from([(
        RESPONSES_LITE_METADATA_KEY.to_string(),
        "true".to_string(),
    )]));
    let second = connection
        .stream_request_observed(
            ResponsesWsRequest::ResponseCreate(incremental),
            /*connection_reused*/ true,
            /*turn_state*/ None,
            second_observer.clone(),
        )
        .await?;
    drain_completed(second).await?;

    let sent_frames = server.await.expect("WebSocket server task");
    let [first_create, interrupt, second_create] = sent_frames.as_slice() else {
        panic!("expected two model requests and one interrupt frame");
    };
    assert_eq!(first_create, FIRST_WIRE_JSON);
    assert_eq!(second_create, SECOND_WIRE_JSON);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(interrupt)?,
        json!({
            "type": "response.interrupt",
            "response_id": "resp-1",
            "mode": "discard_partial_items",
        })
    );

    let handshake_headers = handshake_headers
        .lock()
        .expect("handshake headers lock")
        .take()
        .expect("captured handshake headers");
    assert_eq!(
        handshake_headers.get("session-id"),
        Some(&HeaderValue::from_static("provider-session"))
    );
    assert_eq!(
        handshake_headers.get("x-codex-routing-hint"),
        Some(&HeaderValue::from_static("request-routing"))
    );
    assert_eq!(
        handshake_headers.get(http::header::AUTHORIZATION),
        Some(&HeaderValue::from_static(
            "Bearer observer-must-not-see-this"
        ))
    );
    assert_eq!(
        handshake_headers.get("x-private-provider-secret"),
        Some(&HeaderValue::from_static("provider-secret"))
    );
    assert_eq!(
        handshake_headers.get("x-private-request-secret"),
        Some(&HeaderValue::from_static("request-secret"))
    );

    let handshake_identity = OwnedTransportIdentity {
        session_id: bytes("provider-session"),
        thread_id: bytes("default-thread"),
        client_request_id: bytes("default-client-request"),
        subagent: bytes("default-subagent"),
        routing_hint: bytes("request-routing"),
        responses_lite: None,
        previous_response_id: None,
        originator: bytes("provider-originator"),
        user_agent: bytes("request-user-agent"),
    };
    assert_eq!(
        first_observer.take_websocket(),
        vec![OwnedWebsocketObservation {
            wire_json: FIRST_WIRE_JSON.to_vec(),
            endpoint: ResponsesEndpoint::GuardianClassifier,
            identity: handshake_identity.clone(),
            connection_reused: false,
            incremental: false,
        }]
    );
    assert_eq!(
        second_observer.take_websocket(),
        vec![OwnedWebsocketObservation {
            wire_json: SECOND_WIRE_JSON.to_vec(),
            endpoint: ResponsesEndpoint::GuardianClassifier,
            identity: OwnedTransportIdentity {
                responses_lite: bytes("true"),
                previous_response_id: bytes("resp-1"),
                ..handshake_identity
            },
            connection_reused: true,
            incremental: true,
        }]
    );
    Ok(())
}
