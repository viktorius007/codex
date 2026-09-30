use serde::Serialize;

use super::Fingerprint;
use super::Fingerprinter;
use super::ManifestObservation;
use super::ObservationStatus;
use crate::CompressionKind;
use crate::EndpointKind;
use crate::TransportIdentity;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportManifest {
    endpoint: EndpointKind,
    compression: ManifestObservation<CompressionKind>,
    connection_reused: ManifestObservation<bool>,
    incremental: ManifestObservation<bool>,
    identity: TransportIdentityManifest,
}

impl TransportManifest {
    pub(super) fn http(
        fingerprinter: &Fingerprinter,
        endpoint: EndpointKind,
        compression: CompressionKind,
        identity: TransportIdentity<'_>,
    ) -> Self {
        Self {
            endpoint,
            compression: ManifestObservation::Observed(compression),
            connection_reused: unavailable(),
            incremental: unavailable(),
            identity: TransportIdentityManifest::new(fingerprinter, identity),
        }
    }

    pub(super) fn websocket(
        fingerprinter: &Fingerprinter,
        endpoint: EndpointKind,
        connection_reused: bool,
        incremental: bool,
        identity: TransportIdentity<'_>,
    ) -> Self {
        Self {
            endpoint,
            compression: unavailable(),
            connection_reused: ManifestObservation::Observed(connection_reused),
            incremental: ManifestObservation::Observed(incremental),
            identity: TransportIdentityManifest::new(fingerprinter, identity),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransportIdentityManifest {
    session_id: ManifestObservation<Fingerprint>,
    thread_id: ManifestObservation<Fingerprint>,
    client_request_id: ManifestObservation<Fingerprint>,
    subagent: ManifestObservation<Fingerprint>,
    routing_hint: ManifestObservation<Fingerprint>,
    responses_lite: ManifestObservation<Fingerprint>,
    previous_response_id: ManifestObservation<Fingerprint>,
    originator: ManifestObservation<Fingerprint>,
    user_agent: ManifestObservation<Fingerprint>,
}

impl TransportIdentityManifest {
    fn new(fingerprinter: &Fingerprinter, identity: TransportIdentity<'_>) -> Self {
        Self {
            session_id: optional_byte_slice(
                fingerprinter,
                "transport.identity.session",
                identity.session_id,
            ),
            thread_id: optional_byte_slice(
                fingerprinter,
                "transport.identity.thread",
                identity.thread_id,
            ),
            client_request_id: optional_byte_slice(
                fingerprinter,
                "transport.identity.client_request",
                identity.client_request_id,
            ),
            subagent: optional_byte_slice(
                fingerprinter,
                "transport.identity.subagent",
                identity.subagent,
            ),
            routing_hint: optional_byte_slice(
                fingerprinter,
                "transport.identity.routing_hint",
                identity.routing_hint,
            ),
            responses_lite: optional_byte_slice(
                fingerprinter,
                "transport.identity.responses_lite",
                identity.responses_lite,
            ),
            previous_response_id: optional_byte_slice(
                fingerprinter,
                "transport.identity.previous_response",
                identity.previous_response_id,
            ),
            originator: optional_byte_slice(
                fingerprinter,
                "transport.identity.originator",
                identity.originator,
            ),
            user_agent: optional_byte_slice(
                fingerprinter,
                "transport.identity.user_agent",
                identity.user_agent,
            ),
        }
    }
}

fn optional_byte_slice(
    fingerprinter: &Fingerprinter,
    domain: &str,
    value: Option<&[u8]>,
) -> ManifestObservation<Fingerprint> {
    value.map_or_else(
        || ManifestObservation::Status {
            status: ObservationStatus::Missing,
        },
        |value| ManifestObservation::Observed(fingerprinter.fingerprint_bytes(domain, value)),
    )
}

fn unavailable<T>() -> ManifestObservation<T> {
    ManifestObservation::Status {
        status: ObservationStatus::Unavailable,
    }
}
