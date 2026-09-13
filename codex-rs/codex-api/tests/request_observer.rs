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

#[derive(Clone, Default)]
struct RecordingTransport {
    requests: Arc<Mutex<Vec<Request>>>,
}

impl RecordingTransport {
    fn take_requests(&self) -> Vec<Request> {
        std::mem::take(&mut *self.requests.lock().expect("request lock"))
    }
}

impl HttpTransport for RecordingTransport {
    async fn execute(&self, _request: Request) -> Result<Response, TransportError> {
        Err(TransportError::Build("execute should not run".to_string()))
    }

    async fn stream(&self, request: Request) -> Result<StreamResponse, TransportError> {
        self.requests.lock().expect("request lock").push(request);
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
            base_delay: Duration::from_millis(1),
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
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
async fn http_observer_matches_exact_prepared_send_and_safe_merged_identity() -> Result<()> {
    const EXPECTED_JSON: &[u8] = br#"{"model":"gpt-test","instructions":"observer fixture","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[]}"#;

    let transport = RecordingTransport::default();
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
    let [sent] = sent.as_slice() else {
        panic!("expected exactly one sent request, got {}", sent.len());
    };
    assert_eq!(
        sent.headers.get(http::header::CONTENT_ENCODING),
        Some(&HeaderValue::from_static("zstd"))
    );
    assert_eq!(
        sent.headers.get(http::header::AUTHORIZATION),
        Some(&HeaderValue::from_static(
            "Bearer observer-must-not-see-this"
        ))
    );
    assert_eq!(
        sent.headers.get("x-private-provider-secret"),
        Some(&HeaderValue::from_static("provider-secret"))
    );
    assert_eq!(
        sent.headers.get("x-private-request-secret"),
        Some(&HeaderValue::from_static("request-secret"))
    );

    let prepared_body = body_bytes(sent).to_vec();
    assert_ne!(prepared_body, EXPECTED_JSON);
    assert_eq!(
        observer.take_http(),
        vec![OwnedHttpObservation {
            uncompressed_json: EXPECTED_JSON.to_vec(),
            prepared_body,
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

        let mut requests = Vec::new();
        for response_id in ["resp-1", "resp-2"] {
            let message = websocket
                .next()
                .await
                .expect("receive request frame")
                .expect("valid request frame");
            let text = message.into_text().expect("text request frame");
            requests.push(text.as_bytes().to_vec());
            websocket
                .send(Message::Text(
                    format!(r#"{{"type":"response.created","response":{{"id":"{response_id}"}}}}"#)
                        .into(),
                ))
                .await
                .expect("send response.created");
            websocket
                .send(Message::Text(
                    format!(
                        r#"{{"type":"response.completed","response":{{"id":"{response_id}","usage":{{"input_tokens":0,"input_tokens_details":null,"output_tokens":0,"output_tokens_details":null,"total_tokens":0}}}}}}"#
                    )
                    .into(),
                ))
                .await
                .expect("send response.completed");
        }
        requests
    });

    (format!("http://{address}"), handshake_headers, server)
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
async fn websocket_observer_matches_two_exact_frames_on_reused_connection() -> Result<()> {
    const FIRST_WIRE_JSON: &[u8] = br#"{"type":"response.create","model":"gpt-test","instructions":"observer fixture","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[]}"#;
    const SECOND_WIRE_JSON: &[u8] = br#"{"type":"response.create","model":"gpt-test","instructions":"observer fixture","previous_response_id":"resp-1","input":[],"tool_choice":"auto","parallel_tool_calls":false,"reasoning":null,"store":false,"stream":true,"include":[],"client_metadata":{"ws_request_header_x_openai_internal_codex_responses_lite":"true"}}"#;

    let (base_url, handshake_headers, server) = spawn_websocket_server().await;
    let observer = Arc::new(RecordingObserver::default());
    let client = ResponsesWebsocketClient::new(provider(base_url), Arc::new(StaticAuth))
        .with_endpoint(ResponsesEndpoint::GuardianClassifier)
        .with_request_observer(observer.clone());

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

    let first = connection
        .stream_request(
            ResponsesWsRequest::ResponseCreate(ResponseCreateWsRequest::from(&request)),
            /*connection_reused*/ false,
            /*turn_state*/ None,
        )
        .await?;
    drain_completed(first).await?;

    let mut incremental = ResponseCreateWsRequest::from(&request);
    incremental.previous_response_id = Some("resp-1".to_string());
    incremental.client_metadata = Some(HashMap::from([(
        RESPONSES_LITE_METADATA_KEY.to_string(),
        "true".to_string(),
    )]));
    let second = connection
        .stream_request(
            ResponsesWsRequest::ResponseCreate(incremental),
            /*connection_reused*/ true,
            /*turn_state*/ None,
        )
        .await?;
    drain_completed(second).await?;

    let sent_frames = server.await.expect("WebSocket server task");
    assert_eq!(
        sent_frames,
        vec![FIRST_WIRE_JSON.to_vec(), SECOND_WIRE_JSON.to_vec()]
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
        observer.take_websocket(),
        vec![
            OwnedWebsocketObservation {
                wire_json: FIRST_WIRE_JSON.to_vec(),
                endpoint: ResponsesEndpoint::GuardianClassifier,
                identity: handshake_identity.clone(),
                connection_reused: false,
                incremental: false,
            },
            OwnedWebsocketObservation {
                wire_json: SECOND_WIRE_JSON.to_vec(),
                endpoint: ResponsesEndpoint::GuardianClassifier,
                identity: OwnedTransportIdentity {
                    responses_lite: bytes("true"),
                    previous_response_id: bytes("resp-1"),
                    ..handshake_identity
                },
                connection_reused: true,
                incremental: true,
            },
        ]
    );
    Ok(())
}
