//! Privacy-preserving, immutable records for local prompt-cache diagnostics.

mod key;
mod manifest;
mod writer;

use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_api::ResponsesApiRequest;
use serde::Serialize;

use crate::manifest::Fingerprint;
use crate::manifest::Fingerprinter;
use crate::manifest::LineageManifest;
use crate::manifest::LogicalManifest;
use crate::manifest::ManifestObservation;
use crate::manifest::WireManifest;
use crate::manifest::optional_identity;
use crate::writer::RecordWriter;

const DIAGNOSTIC_DIRECTORY: &str = "cache-diagnostics";
const SCHEMA_VERSION: u32 = 1;

/// A local collection run with one private append-only record file.
pub struct Collector {
    run_id: String,
    key_scope: Fingerprint,
    fingerprinter: Fingerprinter,
    writer: RecordWriter,
}

impl Collector {
    /// Opens a collector below `codex_home`, reusing the installation fingerprint key.
    ///
    /// Failure details intentionally contain no file path or file contents. Callers that enable
    /// collection by default can disable diagnostics for the run without failing inference.
    pub fn open(codex_home: &Path) -> Result<Arc<Self>, OpenError> {
        let directory = codex_home.join(DIAGNOSTIC_DIRECTORY);
        let key = key::load_or_create(&directory)?;
        let fingerprinter = Fingerprinter::new(&key).map_err(|_| OpenError::unavailable())?;
        let run_id = key::random_id()?;
        let writer = RecordWriter::open(&directory, &run_id)?;
        Ok(Arc::new(Self {
            run_id,
            key_scope: fingerprinter.fingerprint_bytes("installation.keyScope", b""),
            fingerprinter,
            writer,
        }))
    }

    /// Starts one request attempt and fingerprints the finalized logical request immediately.
    ///
    /// The returned handle never exposes collection failures to the model-request path. Its
    /// request record is appended when wire bytes are observed, or with an explicit missing marker
    /// when the attempt reaches a terminal state first.
    pub fn start_attempt(
        self: &Arc<Self>,
        context: AttemptContext<'_>,
        logical: &ResponsesApiRequest,
    ) -> Arc<Attempt> {
        let attempt_id = key::random_id().ok();
        let request = attempt_id.as_ref().and_then(|attempt_id| {
            LogicalManifest::new(&self.fingerprinter, logical)
                .ok()
                .map(|logical| RequestRecord {
                    schema_version: SCHEMA_VERSION,
                    event: "request",
                    run_id: self.run_id.clone(),
                    key_scope: self.key_scope.clone(),
                    attempt_id: attempt_id.clone(),
                    sequence: 0,
                    timestamp_unix_ms: None,
                    request_kind: context.request_kind,
                    retry_ordinal: context.retry_ordinal,
                    lineage: LineageManifest::new(&self.fingerprinter, context),
                    logical,
                    wire: WireManifest::missing(),
                })
        });
        let attempt_id = request.as_ref().map(|record| record.attempt_id.clone());
        Arc::new(Attempt {
            collector: Arc::clone(self),
            attempt_id,
            state: Mutex::new(AttemptState {
                request,
                outcome_written: false,
            }),
        })
    }

}

/// Raw request context accepted only long enough to create keyed fingerprints.
#[derive(Clone, Copy)]
pub struct AttemptContext<'a> {
    /// The bounded category of logical request.
    pub request_kind: RequestKind,
    /// Zero-based retry number within the logical request.
    pub retry_ordinal: u32,
    /// Thread lineage identity, when known.
    pub thread_id: Option<&'a str>,
    /// Session identity, when distinct from the thread.
    pub session_id: Option<&'a str>,
    /// Turn identity, when known.
    pub turn_id: Option<&'a str>,
    /// Parent identity for child work, when known.
    pub parent_id: Option<&'a str>,
    /// Cache affinity identity, when known.
    pub affinity_id: Option<&'a str>,
    /// Previous provider response identity, when used.
    pub previous_response_id: Option<&'a str>,
}

/// Stable, low-cardinality logical request categories.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestKind {
    /// A normal model turn, including its retries.
    Turn,
    /// A `generate=false` request used to establish a WebSocket cache prefix.
    Warmup,
    /// A request that compacts prior conversation state.
    Compaction,
    /// An internal memory-generation model request.
    Memory,
}

/// Exact wire bytes observed immediately before a transport send.
#[derive(Clone, Copy)]
pub struct WireRequest<'a> {
    /// The transport whose serializer produced `body`.
    pub kind: WireRequestKind,
    /// The exact serialized bytes sent by that transport.
    pub body: &'a [u8],
}

/// Exact wire bytes plus the bounded transport facts that shaped the send.
pub struct TransportRequest<'a> {
    /// The exact serialized bytes sent by the transport.
    pub body: &'a [u8],
    /// Transport-specific, privacy-safe metadata observed at the send boundary.
    pub transport: TransportMetadata<'a>,
}

/// Privacy-safe transport metadata for one request send.
pub enum TransportMetadata<'a> {
    /// An HTTP Responses request.
    Http {
        /// The bounded Responses route category.
        endpoint: EndpointKind,
        /// The request-body compression applied to the prepared bytes.
        compression: CompressionKind,
        /// A closed projection of identity-bearing transport fields.
        identity: TransportIdentity<'a>,
    },
    /// A WebSocket `response.create` request.
    Websocket {
        /// The bounded Responses route category.
        endpoint: EndpointKind,
        /// Whether this send reused an existing physical connection.
        connection_reused: bool,
        /// Whether the frame carries an incremental request.
        incremental: bool,
        /// A closed projection of identity-bearing transport fields.
        identity: TransportIdentity<'a>,
    },
}

/// Responses-compatible route categories retained without a URL or path.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointKind {
    /// Regular model inference.
    Responses,
    /// Full Guardian approval review.
    Guardian,
    /// Lightweight Guardian classification.
    GuardianClassifier,
}

/// Prepared HTTP body compression retained as a bounded enum.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompressionKind {
    /// The prepared body is uncompressed JSON.
    None,
    /// The prepared body uses Zstandard compression.
    Zstd,
}

/// Closed transport identity projection fingerprinted before persistence.
#[derive(Clone, Copy, Default)]
pub struct TransportIdentity<'a> {
    /// Session header value, when present.
    pub session_id: Option<&'a [u8]>,
    /// Thread header value, when present.
    pub thread_id: Option<&'a [u8]>,
    /// Client request header value, when present.
    pub client_request_id: Option<&'a [u8]>,
    /// Subagent header value, when present.
    pub subagent: Option<&'a [u8]>,
    /// Routing-hint header value, when present.
    pub routing_hint: Option<&'a [u8]>,
    /// Responses Lite marker, when present.
    pub responses_lite: Option<&'a [u8]>,
    /// Previous response identity, when present.
    pub previous_response_id: Option<&'a [u8]>,
    /// Client originator value, when present.
    pub originator: Option<&'a [u8]>,
    /// User-agent value, when present.
    pub user_agent: Option<&'a [u8]>,
}

/// Stable, low-cardinality request transports.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WireRequestKind {
    /// An HTTP request body.
    Http,
    /// A WebSocket `response.create` message.
    Websocket,
}

/// Numeric token usage safe to persist alongside an outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    /// Tokens supplied to the model.
    pub input: i64,
    /// Input tokens reported as cached by the provider.
    pub cached_input: i64,
    /// Input tokens reported as written to a cache.
    pub cache_write: i64,
    /// Tokens produced by the model.
    pub output: i64,
    /// Output tokens attributed to reasoning.
    pub reasoning: i64,
    /// Total tokens reported by the provider.
    pub total: i64,
}

/// Data recorded for a successfully completed attempt.
#[derive(Clone, Copy)]
pub struct CompletedOutcome<'a> {
    /// Provider response identity, fingerprinted before persistence.
    pub response_id: Option<&'a str>,
    /// Provider-reported numeric usage.
    pub usage: TokenUsage,
}

/// Bounded terminal categories that never contain provider error text.
#[derive(Clone, Copy)]
pub enum TerminalOutcome {
    /// The attempt failed.
    Failed,
    /// The attempt was cancelled.
    Cancelled,
    /// The transport or protocol fell back to another path.
    Fallback,
}

/// One in-flight diagnostic attempt.
pub struct Attempt {
    collector: Arc<Collector>,
    attempt_id: Option<String>,
    state: Mutex<AttemptState>,
}

impl Attempt {
    /// Appends the immutable request record using the exact observed bytes.
    ///
    /// Duplicate or late observations are ignored because a persisted request record is never
    /// rewritten.
    pub fn observe_wire_request(&self, request: WireRequest<'_>) {
        self.observe_wire_manifest(WireManifest::observed(
            &self.collector.fingerprinter,
            request,
        ));
    }

    /// Appends exact wire bytes with bounded transport metadata.
    pub fn observe_transport_request(&self, request: TransportRequest<'_>) {
        self.observe_wire_manifest(WireManifest::observed_transport(
            &self.collector.fingerprinter,
            request,
        ));
    }

    fn observe_wire_manifest(&self, wire: WireManifest) {
        let mut state = self.lock_state();
        let Some(mut record) = state.request.take() else {
            return;
        };
        record.wire = wire;
        let _ = self
            .collector
            .writer
            .append(record, RequestRecord::set_metadata);
    }

    /// Records a completed outcome and links it to this attempt.
    pub fn completed(&self, outcome: CompletedOutcome<'_>) {
        self.write_outcome(
            TerminalStatus::Completed,
            outcome.response_id,
            Some(outcome.usage),
        );
    }

    /// Records a successful terminal response that did not report token usage.
    pub fn completed_without_usage(&self, response_id: Option<&str>) {
        self.write_outcome(TerminalStatus::Completed, response_id, None);
    }

    /// Records a non-successful terminal outcome and links it to this attempt.
    pub fn terminal(&self, outcome: TerminalOutcome) {
        let terminal = match outcome {
            TerminalOutcome::Failed => TerminalStatus::Failed,
            TerminalOutcome::Cancelled => TerminalStatus::Cancelled,
            TerminalOutcome::Fallback => TerminalStatus::Fallback,
        };
        self.write_outcome(terminal, None, None);
    }

    fn write_outcome(
        &self,
        terminal: TerminalStatus,
        response_id: Option<&str>,
        usage: Option<TokenUsage>,
    ) {
        let mut state = self.lock_state();
        if state.outcome_written {
            return;
        }
        let Some(attempt_id) = &self.attempt_id else {
            state.outcome_written = true;
            return;
        };
        if let Some(request) = state.request.take() {
            let _ = self
                .collector
                .writer
                .append(request, RequestRecord::set_metadata);
        }
        let outcome = OutcomeRecord {
            schema_version: SCHEMA_VERSION,
            event: "outcome",
            run_id: self.collector.run_id.clone(),
            key_scope: self.collector.key_scope.clone(),
            attempt_id: attempt_id.clone(),
            sequence: 0,
            timestamp_unix_ms: None,
            terminal,
            response_id: optional_identity(
                &self.collector.fingerprinter,
                "identity.response",
                response_id,
            ),
            usage,
        };
        let _ = self
            .collector
            .writer
            .append(outcome, OutcomeRecord::set_metadata);
        state.outcome_written = true;
    }

    fn lock_state(&self) -> MutexGuard<'_, AttemptState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct AttemptState {
    request: Option<RequestRecord>,
    outcome_written: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestRecord {
    schema_version: u32,
    event: &'static str,
    run_id: String,
    key_scope: Fingerprint,
    attempt_id: String,
    sequence: u64,
    timestamp_unix_ms: Option<u64>,
    request_kind: RequestKind,
    retry_ordinal: u32,
    lineage: LineageManifest,
    logical: LogicalManifest,
    wire: WireManifest,
}

impl RequestRecord {
    fn set_metadata(&mut self, sequence: u64, timestamp_unix_ms: Option<u64>) {
        self.sequence = sequence;
        self.timestamp_unix_ms = timestamp_unix_ms;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutcomeRecord {
    schema_version: u32,
    event: &'static str,
    run_id: String,
    key_scope: Fingerprint,
    attempt_id: String,
    sequence: u64,
    timestamp_unix_ms: Option<u64>,
    terminal: TerminalStatus,
    response_id: ManifestObservation<Fingerprint>,
    usage: Option<TokenUsage>,
}

impl OutcomeRecord {
    fn set_metadata(&mut self, sequence: u64, timestamp_unix_ms: Option<u64>) {
        self.sequence = sequence;
        self.timestamp_unix_ms = timestamp_unix_ms;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum TerminalStatus {
    Completed,
    Failed,
    Cancelled,
    Fallback,
}

/// Content-free collector initialization error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenError;

impl OpenError {
    pub(crate) fn unavailable() -> Self {
        Self
    }

    pub(crate) fn from_io(_error: std::io::Error) -> Self {
        Self
    }
}

impl fmt::Display for OpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cache diagnostic collection is unavailable")
    }
}

impl std::error::Error for OpenError {}

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
