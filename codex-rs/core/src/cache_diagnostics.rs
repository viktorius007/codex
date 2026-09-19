//! Private runtime adapter for privacy-preserving prompt-cache diagnostics.

use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use codex_api::Compression;
use codex_api::HttpRequestObservation;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesEndpoint;
use codex_api::ResponsesRequestObserver;
use codex_api::SafeTransportIdentity;
use codex_api::WebsocketRequestObservation;
use codex_cache_diagnostics::Attempt;
use codex_cache_diagnostics::AttemptContext;
use codex_cache_diagnostics::Collector;
use codex_cache_diagnostics::CompletedOutcome;
use codex_cache_diagnostics::CompressionKind;
use codex_cache_diagnostics::EndpointKind;
use codex_cache_diagnostics::RequestKind;
use codex_cache_diagnostics::TerminalOutcome;
use codex_cache_diagnostics::TokenUsage;
use codex_cache_diagnostics::TransportIdentity;
use codex_cache_diagnostics::TransportMetadata;
use codex_cache_diagnostics::TransportRequest;
use codex_protocol::protocol::TokenUsage as ProtocolTokenUsage;

use crate::responses_metadata::CodexResponsesMetadata;
use crate::responses_metadata::CodexResponsesRequestKind;

#[derive(Default)]
pub(crate) struct CacheDiagnosticAttemptSequencer(AtomicU32);

impl CacheDiagnosticAttemptSequencer {
    pub(crate) fn next(&self) -> u32 {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(1))
            })
            .unwrap_or(u32::MAX)
    }
}

#[derive(Clone)]
pub(crate) struct CacheDiagnostics {
    collector: Arc<Collector>,
}

impl CacheDiagnostics {
    pub(crate) fn open(codex_home: &Path) -> Option<Self> {
        Collector::open(codex_home)
            .ok()
            .map(|collector| Self { collector })
    }

    pub(crate) fn start_responses_attempt(
        &self,
        metadata: &CodexResponsesMetadata,
        request: &ResponsesApiRequest,
        retry_ordinal: u32,
        previous_response_id: Option<&str>,
    ) -> Arc<CacheDiagnosticAttempt> {
        self.start_responses_attempt_with_kind(
            metadata,
            request,
            retry_ordinal,
            previous_response_id,
            request_kind(metadata.request_kind),
        )
    }

    pub(crate) fn start_warmup_attempt(
        &self,
        metadata: &CodexResponsesMetadata,
        request: &ResponsesApiRequest,
        retry_ordinal: u32,
        previous_response_id: Option<&str>,
    ) -> Arc<CacheDiagnosticAttempt> {
        self.start_responses_attempt_with_kind(
            metadata,
            request,
            retry_ordinal,
            previous_response_id,
            RequestKind::Warmup,
        )
    }

    fn start_responses_attempt_with_kind(
        &self,
        metadata: &CodexResponsesMetadata,
        request: &ResponsesApiRequest,
        retry_ordinal: u32,
        previous_response_id: Option<&str>,
        request_kind: RequestKind,
    ) -> Arc<CacheDiagnosticAttempt> {
        let parent_id = metadata.parent_thread_id.map(|id| id.to_string());
        let attempt = self.collector.start_attempt(
            AttemptContext {
                request_kind,
                retry_ordinal,
                thread_id: Some(&metadata.thread_id),
                session_id: Some(&metadata.session_id),
                turn_id: metadata.turn_id.as_deref(),
                parent_id: parent_id.as_deref(),
                affinity_id: request.prompt_cache_key.as_deref(),
                previous_response_id,
            },
            request,
        );
        Arc::new(CacheDiagnosticAttempt { attempt })
    }
}

impl fmt::Debug for CacheDiagnostics {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CacheDiagnostics")
    }
}

pub(crate) struct CacheDiagnosticAttempt {
    attempt: Arc<Attempt>,
}

impl CacheDiagnosticAttempt {
    /// Attaches the continuation decision to the pending request record.
    pub(crate) fn record_continuation(&self, report: codex_cache_diagnostics::ContinuationReport) {
        self.attempt.record_continuation(report);
    }

    /// Attaches tool-construction provenance to the pending request record.
    pub(crate) fn record_tool_provenance(
        &self,
        provenance: codex_cache_diagnostics::ToolProvenance<'_>,
    ) {
        self.attempt.record_tool_provenance(provenance);
    }

    pub(crate) fn completed(&self, response_id: &str, usage: Option<&ProtocolTokenUsage>) {
        let Some(usage) = usage else {
            self.attempt.completed_without_usage(Some(response_id));
            return;
        };
        self.attempt.completed(CompletedOutcome {
            response_id: Some(response_id),
            usage: TokenUsage {
                input: usage.input_tokens,
                cached_input: usage.cached_input_tokens,
                cache_write: usage.cache_write_input_tokens,
                output: usage.output_tokens,
                reasoning: usage.reasoning_output_tokens,
                total: usage.total_tokens,
            },
        });
    }

    pub(crate) fn failed(&self) {
        self.attempt.terminal(TerminalOutcome::Failed);
    }
}

impl Drop for CacheDiagnosticAttempt {
    fn drop(&mut self) {
        self.attempt.terminal(TerminalOutcome::Cancelled);
    }
}

impl ResponsesRequestObserver for CacheDiagnosticAttempt {
    fn observe_http(&self, observation: HttpRequestObservation<'_>) {
        self.attempt.observe_transport_request(TransportRequest {
            body: observation.prepared_body,
            transport: TransportMetadata::Http {
                endpoint: endpoint_kind(observation.endpoint),
                compression: match observation.compression {
                    Compression::None => CompressionKind::None,
                    Compression::Zstd => CompressionKind::Zstd,
                },
                identity: transport_identity(observation.identity),
            },
        });
    }

    fn observe_websocket(&self, observation: WebsocketRequestObservation<'_>) {
        self.attempt.observe_transport_request(TransportRequest {
            body: observation.wire_json,
            transport: TransportMetadata::Websocket {
                endpoint: endpoint_kind(observation.endpoint),
                connection_reused: observation.connection_reused,
                incremental: observation.incremental,
                identity: transport_identity(observation.identity),
            },
        });
    }
}

fn request_kind(kind: Option<CodexResponsesRequestKind>) -> RequestKind {
    match kind {
        Some(CodexResponsesRequestKind::Prewarm) => RequestKind::Warmup,
        Some(CodexResponsesRequestKind::Compaction(_)) => RequestKind::Compaction,
        Some(CodexResponsesRequestKind::Memory) => RequestKind::Memory,
        Some(CodexResponsesRequestKind::Turn) | None => RequestKind::Turn,
    }
}

fn endpoint_kind(endpoint: ResponsesEndpoint) -> EndpointKind {
    match endpoint {
        ResponsesEndpoint::Responses => EndpointKind::Responses,
        ResponsesEndpoint::Guardian => EndpointKind::Guardian,
        ResponsesEndpoint::GuardianClassifier => EndpointKind::GuardianClassifier,
    }
}

fn transport_identity(identity: SafeTransportIdentity<'_>) -> TransportIdentity<'_> {
    TransportIdentity {
        session_id: identity.session_id,
        thread_id: identity.thread_id,
        client_request_id: identity.client_request_id,
        subagent: identity.subagent,
        routing_hint: identity.routing_hint,
        responses_lite: identity.responses_lite,
        previous_response_id: identity.previous_response_id,
        originator: identity.originator,
        user_agent: identity.user_agent,
    }
}
