use crate::endpoint::responses::ResponsesEndpoint;
use crate::requests::Compression;
use http::HeaderMap;
use std::sync::Arc;

const RESPONSES_LITE_HEADER: &str = "x-openai-internal-codex-responses-lite";
pub(crate) const WS_REQUEST_HEADER_RESPONSES_LITE_CLIENT_METADATA_KEY: &str =
    "ws_request_header_x_openai_internal_codex_responses_lite";

/// Receives exact request bytes and a closed set of safe transport identities.
///
/// Implementations are responsible for containing their own failures so observation never
/// interrupts an inference request.
pub trait ResponsesRequestObserver: Send + Sync {
    /// Observes one prepared Responses HTTP request before authentication.
    fn observe_http(&self, observation: HttpRequestObservation<'_>);

    /// Observes one exact Responses WebSocket text request immediately before it is sent.
    fn observe_websocket(&self, observation: WebsocketRequestObservation<'_>);

    /// Observes one prepared unary compaction request before authentication.
    fn observe_compaction_http(&self, _observation: CompactionHttpRequestObservation<'_>) {}
}

/// Exact HTTP request bytes and safe identity fields for `/responses/compact`.
pub struct CompactionHttpRequestObservation<'a> {
    pub prepared_body: &'a [u8],
    pub identity: SafeTransportIdentity<'a>,
}

/// Exact HTTP request bytes and safe identity fields captured before authentication.
pub struct HttpRequestObservation<'a> {
    pub uncompressed_json: &'a [u8],
    pub prepared_body: &'a [u8],
    pub endpoint: ResponsesEndpoint,
    pub compression: Compression,
    pub identity: SafeTransportIdentity<'a>,
}

/// Exact WebSocket request bytes and safe identity fields captured at the send boundary.
pub struct WebsocketRequestObservation<'a> {
    pub wire_json: &'a [u8],
    pub endpoint: ResponsesEndpoint,
    pub identity: SafeTransportIdentity<'a>,
    pub connection_reused: bool,
    pub incremental: bool,
}

/// A fixed projection of transport identity fields that are safe to fingerprint locally.
pub struct SafeTransportIdentity<'a> {
    pub session_id: Option<&'a [u8]>,
    pub thread_id: Option<&'a [u8]>,
    pub client_request_id: Option<&'a [u8]>,
    pub subagent: Option<&'a [u8]>,
    pub routing_hint: Option<&'a [u8]>,
    pub responses_lite: Option<&'a [u8]>,
    pub previous_response_id: Option<&'a [u8]>,
    pub originator: Option<&'a [u8]>,
    pub user_agent: Option<&'a [u8]>,
}

impl<'a> SafeTransportIdentity<'a> {
    pub(crate) fn from_http_headers(headers: &'a HeaderMap) -> Self {
        Self {
            session_id: header_bytes(headers, "session-id"),
            thread_id: header_bytes(headers, "thread-id"),
            client_request_id: header_bytes(headers, "x-client-request-id"),
            subagent: header_bytes(headers, "x-openai-subagent"),
            routing_hint: header_bytes(headers, "x-codex-routing-hint"),
            responses_lite: header_bytes(headers, RESPONSES_LITE_HEADER),
            previous_response_id: None,
            originator: header_bytes(headers, "originator"),
            user_agent: header_bytes(headers, "user-agent"),
        }
    }
}

struct OwnedSafeTransportIdentity {
    session_id: Option<Vec<u8>>,
    thread_id: Option<Vec<u8>>,
    client_request_id: Option<Vec<u8>>,
    subagent: Option<Vec<u8>>,
    routing_hint: Option<Vec<u8>>,
    originator: Option<Vec<u8>>,
    user_agent: Option<Vec<u8>>,
}

pub(crate) struct WebsocketObservationState {
    transport_identity: OwnedSafeTransportIdentity,
    pub(crate) endpoint: ResponsesEndpoint,
}

impl WebsocketObservationState {
    pub(crate) fn new(headers: &HeaderMap, endpoint: ResponsesEndpoint) -> Self {
        Self {
            transport_identity: OwnedSafeTransportIdentity::from_websocket_headers(headers),
            endpoint,
        }
    }

    pub(crate) fn identity(&self) -> SafeTransportIdentity<'_> {
        self.transport_identity.as_borrowed()
    }
}

pub(crate) struct PreparedWebsocketRequest {
    pub(crate) text: String,
    pub(crate) observation: Arc<WebsocketObservationState>,
    pub(crate) observer: Option<Arc<dyn ResponsesRequestObserver>>,
    pub(crate) responses_lite: Option<String>,
    pub(crate) previous_response_id: Option<String>,
    pub(crate) connection_reused: bool,
}

impl OwnedSafeTransportIdentity {
    pub(crate) fn from_websocket_headers(headers: &HeaderMap) -> Self {
        Self {
            session_id: owned_header_bytes(headers, "session-id"),
            thread_id: owned_header_bytes(headers, "thread-id"),
            client_request_id: owned_header_bytes(headers, "x-client-request-id"),
            subagent: owned_header_bytes(headers, "x-openai-subagent"),
            routing_hint: owned_header_bytes(headers, "x-codex-routing-hint"),
            originator: owned_header_bytes(headers, "originator"),
            user_agent: owned_header_bytes(headers, "user-agent"),
        }
    }

    pub(crate) fn as_borrowed(&self) -> SafeTransportIdentity<'_> {
        SafeTransportIdentity {
            session_id: self.session_id.as_deref(),
            thread_id: self.thread_id.as_deref(),
            client_request_id: self.client_request_id.as_deref(),
            subagent: self.subagent.as_deref(),
            routing_hint: self.routing_hint.as_deref(),
            responses_lite: None,
            previous_response_id: None,
            originator: self.originator.as_deref(),
            user_agent: self.user_agent.as_deref(),
        }
    }
}

fn header_bytes<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a [u8]> {
    headers.get(name).map(http::HeaderValue::as_bytes)
}

fn owned_header_bytes(headers: &HeaderMap, name: &str) -> Option<Vec<u8>> {
    header_bytes(headers, name).map(<[u8]>::to_vec)
}
