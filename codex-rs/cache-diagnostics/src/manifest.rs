use std::io;
use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use codex_api::ResponsesApiRequest;
use hmac::Hmac;
use hmac::Mac;
use serde::Serialize;
use sha2::Sha256;

use crate::AttemptContext;
use crate::TransportMetadata;
use crate::TransportRequest;
use crate::WireRequest;

mod tools;
mod transport;
use self::tools::ToolsDetailManifest;
use self::tools::tools_manifest;
use self::transport::TransportManifest;

const RETAINED_LEAVES: usize = 1_024;
const FINGERPRINT_DOMAIN: &[u8] = b"codex-cache-diagnostics-v1\0";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub(crate) struct Fingerprinter(HmacSha256);

impl Fingerprinter {
    pub(crate) fn new(key: &[u8]) -> Result<Self, hmac::digest::InvalidLength> {
        HmacSha256::new_from_slice(key).map(Self)
    }

    pub(crate) fn fingerprint_bytes(&self, domain: &str, bytes: &[u8]) -> Fingerprint {
        let mut stream = self.stream(domain);
        stream.absorb(bytes);
        stream.finish().fingerprint
    }

    fn fingerprint_json<T: Serialize + ?Sized>(
        &self,
        domain: &str,
        value: &T,
    ) -> serde_json::Result<Fingerprint> {
        Ok(self.fingerprint_json_digest(domain, value)?.fingerprint)
    }

    fn fingerprint_json_digest<T: Serialize + ?Sized>(
        &self,
        domain: &str,
        value: &T,
    ) -> serde_json::Result<FinishedFingerprint> {
        let mut stream = self.stream(domain);
        serde_json::to_writer(&mut stream, value)?;
        Ok(stream.finish())
    }

    fn list_manifest<T: Serialize>(
        &self,
        body_domain: &str,
        item_domain: &str,
        omitted_domain: &str,
        values: &[T],
    ) -> serde_json::Result<ListManifest> {
        let body = self.fingerprint_json(body_domain, values)?;
        let mut retained = Vec::with_capacity(values.len().min(RETAINED_LEAVES));
        let mut omitted = self.stream(omitted_domain);
        for (index, value) in values.iter().enumerate() {
            let fingerprint = self.fingerprint_json_digest(item_domain, value)?;
            if index < RETAINED_LEAVES {
                retained.push(fingerprint.fingerprint);
            } else {
                omitted.absorb_entry(index, &fingerprint);
            }
        }
        let omitted_count = values.len() - retained.len();
        Ok(ListManifest {
            body,
            count: values.len(),
            retained,
            omitted: OmittedManifest {
                count: omitted_count,
                hmac: omitted.finish().fingerprint.hmac,
            },
        })
    }

    fn stream(&self, domain: &str) -> FingerprintStream {
        let mut mac = self.0.clone();
        mac.update(FINGERPRINT_DOMAIN);
        mac.update(&(domain.len() as u64).to_le_bytes());
        mac.update(domain.as_bytes());
        FingerprintStream { mac, bytes: 0 }
    }
}

#[derive(Serialize)]
pub(crate) struct LogicalManifest {
    body: Fingerprint,
    model: Fingerprint,
    instructions: Fingerprint,
    input: ListManifest,
    tools: ManifestObservation<ListManifest>,
    #[serde(rename = "toolsDetail")]
    tools_detail: ManifestObservation<ToolsDetailManifest>,
    #[serde(rename = "toolChoice")]
    tool_choice: Fingerprint,
    #[serde(rename = "parallelToolCalls")]
    parallel_tool_calls: Fingerprint,
    reasoning: ManifestObservation<Fingerprint>,
    store: Fingerprint,
    stream: Fingerprint,
    #[serde(rename = "streamOptions")]
    stream_options: ManifestObservation<Fingerprint>,
    include: Fingerprint,
    #[serde(rename = "serviceTier")]
    service_tier: ManifestObservation<Fingerprint>,
    #[serde(rename = "promptCacheKey")]
    prompt_cache_key: ManifestObservation<Fingerprint>,
    text: ManifestObservation<Fingerprint>,
    #[serde(rename = "clientMetadata")]
    client_metadata: ManifestObservation<Fingerprint>,
    #[serde(rename = "accessPrograms")]
    access_programs: ManifestObservation<Fingerprint>,
}

impl LogicalManifest {
    pub(crate) fn new(
        fingerprinter: &Fingerprinter,
        request: &ResponsesApiRequest,
    ) -> serde_json::Result<Self> {
        let tool_manifests = tools_manifest(fingerprinter, request.tools.as_ref());
        Ok(Self {
            body: fingerprinter.fingerprint_json("logical.body", request)?,
            model: fingerprinter.fingerprint_json("logical.model", &request.model)?,
            instructions: fingerprinter
                .fingerprint_json("logical.instructions", &request.instructions)?,
            input: fingerprinter.list_manifest(
                "logical.input",
                "logical.input.item",
                "logical.input.omitted",
                &request.input,
            )?,
            tools: tool_manifests.list,
            tools_detail: tool_manifests.detail,
            tool_choice: fingerprinter
                .fingerprint_json("logical.tool_choice", &request.tool_choice)?,
            parallel_tool_calls: fingerprinter
                .fingerprint_json("logical.parallel_tool_calls", &request.parallel_tool_calls)?,
            reasoning: optional_json(
                fingerprinter,
                "logical.reasoning",
                request.reasoning.as_ref(),
            )?,
            store: fingerprinter.fingerprint_json("logical.store", &request.store)?,
            stream: fingerprinter.fingerprint_json("logical.stream", &request.stream)?,
            stream_options: optional_json(
                fingerprinter,
                "logical.stream_options",
                request.stream_options.as_ref(),
            )?,
            include: fingerprinter.fingerprint_json("logical.include", &request.include)?,
            service_tier: optional_json(
                fingerprinter,
                "logical.service_tier",
                request.service_tier.as_ref(),
            )?,
            prompt_cache_key: optional_json(
                fingerprinter,
                "logical.prompt_cache_key",
                request.prompt_cache_key.as_ref(),
            )?,
            text: optional_json(fingerprinter, "logical.text", request.text.as_ref())?,
            client_metadata: optional_json(
                fingerprinter,
                "logical.client_metadata",
                request.client_metadata.as_ref(),
            )?,
            access_programs: optional_json(
                fingerprinter,
                "logical.access_programs",
                request.access_programs.as_ref(),
            )?,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LineageManifest {
    thread_id: ManifestObservation<Fingerprint>,
    session_id: ManifestObservation<Fingerprint>,
    turn_id: ManifestObservation<Fingerprint>,
    parent_id: ManifestObservation<Fingerprint>,
    affinity_id: ManifestObservation<Fingerprint>,
    previous_response_id: ManifestObservation<Fingerprint>,
}

impl LineageManifest {
    pub(crate) fn new(fingerprinter: &Fingerprinter, context: AttemptContext<'_>) -> Self {
        Self {
            thread_id: optional_bytes(fingerprinter, "identity.thread", context.thread_id),
            session_id: optional_bytes(fingerprinter, "identity.session", context.session_id),
            turn_id: optional_bytes(fingerprinter, "identity.turn", context.turn_id),
            parent_id: optional_bytes(fingerprinter, "identity.parent", context.parent_id),
            affinity_id: optional_bytes(fingerprinter, "identity.affinity", context.affinity_id),
            previous_response_id: optional_bytes(
                fingerprinter,
                "identity.previous_response",
                context.previous_response_id,
            ),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum WireManifest {
    Observed {
        kind: crate::WireRequestKind,
        body: Fingerprint,
        transport: ManifestObservation<Box<TransportManifest>>,
    },
    Status {
        status: ObservationStatus,
    },
}

impl WireManifest {
    pub(crate) fn missing() -> Self {
        Self::Status {
            status: ObservationStatus::Missing,
        }
    }

    pub(crate) fn observed(fingerprinter: &Fingerprinter, request: WireRequest<'_>) -> Self {
        Self::Observed {
            kind: request.kind,
            body: fingerprinter.fingerprint_bytes("wire.body", request.body),
            transport: ManifestObservation::Status {
                status: ObservationStatus::Unavailable,
            },
        }
    }

    pub(crate) fn observed_transport(
        fingerprinter: &Fingerprinter,
        request: TransportRequest<'_>,
    ) -> Self {
        let (kind, transport) = match request.transport {
            TransportMetadata::Http {
                endpoint,
                compression,
                identity,
            } => (
                crate::WireRequestKind::Http,
                TransportManifest::http(fingerprinter, endpoint, compression, identity),
            ),
            TransportMetadata::Websocket {
                endpoint,
                connection_reused,
                incremental,
                identity,
            } => (
                crate::WireRequestKind::Websocket,
                TransportManifest::websocket(
                    fingerprinter,
                    endpoint,
                    connection_reused,
                    incremental,
                    identity,
                ),
            ),
        };
        Self::Observed {
            kind,
            body: fingerprinter.fingerprint_bytes("wire.body", request.body),
            transport: ManifestObservation::Observed(Box::new(transport)),
        }
    }
}

pub(crate) fn optional_identity(
    fingerprinter: &Fingerprinter,
    domain: &str,
    value: Option<&str>,
) -> ManifestObservation<Fingerprint> {
    optional_bytes(fingerprinter, domain, value)
}

fn optional_bytes(
    fingerprinter: &Fingerprinter,
    domain: &str,
    value: Option<&str>,
) -> ManifestObservation<Fingerprint> {
    value.map_or_else(
        || ManifestObservation::Status {
            status: ObservationStatus::Missing,
        },
        |value| {
            ManifestObservation::Observed(fingerprinter.fingerprint_bytes(domain, value.as_bytes()))
        },
    )
}

fn optional_json<T: Serialize + ?Sized>(
    fingerprinter: &Fingerprinter,
    domain: &str,
    value: Option<&T>,
) -> serde_json::Result<ManifestObservation<Fingerprint>> {
    value.map_or_else(
        || {
            Ok(ManifestObservation::Status {
                status: ObservationStatus::Missing,
            })
        },
        |value| {
            fingerprinter
                .fingerprint_json(domain, value)
                .map(ManifestObservation::Observed)
        },
    )
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum ManifestObservation<T> {
    Observed(T),
    Status { status: ObservationStatus },
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ObservationStatus {
    Missing,
    Unavailable,
}

#[derive(Clone, Serialize)]
pub(crate) struct Fingerprint {
    hmac: String,
    bytes: usize,
}

struct FinishedFingerprint {
    fingerprint: Fingerprint,
    digest: [u8; 32],
}

struct FingerprintStream {
    mac: HmacSha256,
    bytes: usize,
}

impl FingerprintStream {
    fn absorb(&mut self, bytes: &[u8]) {
        self.mac.update(bytes);
        self.bytes += bytes.len();
    }

    fn absorb_entry(&mut self, index: usize, fingerprint: &FinishedFingerprint) {
        self.absorb(&index.to_le_bytes());
        self.absorb(&fingerprint.fingerprint.bytes.to_le_bytes());
        self.absorb(&fingerprint.digest);
    }

    fn finish(self) -> FinishedFingerprint {
        let digest: [u8; 32] = self.mac.finalize().into_bytes().into();
        FinishedFingerprint {
            fingerprint: Fingerprint {
                hmac: URL_SAFE_NO_PAD.encode(digest),
                bytes: self.bytes,
            },
            digest,
        }
    }
}

impl Write for FingerprintStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.absorb(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Serialize)]
pub(crate) struct ListManifest {
    body: Fingerprint,
    count: usize,
    retained: Vec<Fingerprint>,
    omitted: OmittedManifest,
}

#[derive(Serialize)]
pub(crate) struct OmittedManifest {
    count: usize,
    hmac: String,
}
