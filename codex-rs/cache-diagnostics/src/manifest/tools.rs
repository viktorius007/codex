use std::io;
use std::io::Write;

use codex_api::ResponsesApiTools;
use serde::Serialize;

use super::Fingerprint;
use super::FingerprintStream;
use super::Fingerprinter;
use super::ListManifest;
use super::ManifestObservation;
use super::ObservationStatus;
use super::OmittedManifest;
use super::RETAINED_LEAVES;

/// Per-tool detail entries kept small enough that a maximal record stays under
/// the writer's record-size cap.
const RETAINED_TOOL_DETAILS: usize = 256;

/// Sub-field fingerprints for one serialized tool, keyed by its plaintext name.
///
/// Tool names are intentionally stored in the clear: they are bounded,
/// low-sensitivity identifiers, and name-keyed diffs are what let an analyzer
/// say which tool changed rather than only which array position changed.
#[derive(Serialize)]
pub(crate) struct ToolDetailEntry {
    name: String,
    descriptor: Fingerprint,
    description: ManifestObservation<Fingerprint>,
    parameters: ManifestObservation<Fingerprint>,
}

/// Name-keyed manifest of the serialized tool array.
#[derive(Serialize)]
pub(crate) struct ToolsDetailManifest {
    count: usize,
    retained: Vec<ToolDetailEntry>,
    omitted: OmittedManifest,
}

pub(super) struct ToolManifests {
    pub(super) list: ManifestObservation<ListManifest>,
    pub(super) detail: ManifestObservation<ToolsDetailManifest>,
}

pub(super) fn tools_manifest(
    fingerprinter: &Fingerprinter,
    tools: Option<&ResponsesApiTools>,
) -> ToolManifests {
    let Some(tools) = tools else {
        return unavailable(ObservationStatus::Missing);
    };
    let mut writer = ToolManifestWriter::new(fingerprinter);
    if serde_json::to_writer(&mut writer, tools).is_err() {
        return unavailable(ObservationStatus::Unavailable);
    }
    writer.finish().map_or_else(
        || unavailable(ObservationStatus::Unavailable),
        |(list, detail)| ToolManifests {
            list: ManifestObservation::Observed(list),
            detail: ManifestObservation::Observed(detail),
        },
    )
}

fn unavailable(status: ObservationStatus) -> ToolManifests {
    ToolManifests {
        list: ManifestObservation::Status { status },
        detail: ManifestObservation::Status { status },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolScanState {
    BeforeArray,
    BetweenItems,
    InItem,
    AfterArray,
}

struct ToolManifestWriter<'a> {
    fingerprinter: &'a Fingerprinter,
    total: FingerprintStream,
    omitted: FingerprintStream,
    current: Option<FingerprintStream>,
    current_bytes: Vec<u8>,
    retained: Vec<Fingerprint>,
    details: Vec<ToolDetailEntry>,
    details_omitted: FingerprintStream,
    count: usize,
    state: ToolScanState,
    depth: usize,
    in_string: bool,
    escaped: bool,
    invalid: bool,
}

impl<'a> ToolManifestWriter<'a> {
    fn new(fingerprinter: &'a Fingerprinter) -> Self {
        Self {
            fingerprinter,
            total: fingerprinter.stream("logical.tools"),
            omitted: fingerprinter.stream("logical.tools.omitted"),
            current: None,
            current_bytes: Vec::new(),
            retained: Vec::with_capacity(RETAINED_LEAVES),
            details: Vec::with_capacity(RETAINED_TOOL_DETAILS),
            details_omitted: fingerprinter.stream("logical.tools.detail.omitted"),
            count: 0,
            state: ToolScanState::BeforeArray,
            depth: 0,
            in_string: false,
            escaped: false,
            invalid: false,
        }
    }

    fn scan(&mut self, byte: u8) {
        match self.state {
            ToolScanState::BeforeArray => {
                if byte == b'[' {
                    self.state = ToolScanState::BetweenItems;
                } else if !byte.is_ascii_whitespace() {
                    self.invalid = true;
                }
            }
            ToolScanState::BetweenItems => {
                if byte == b']' {
                    self.state = ToolScanState::AfterArray;
                } else if byte != b',' && !byte.is_ascii_whitespace() {
                    self.current = Some(self.fingerprinter.stream("logical.tools.item"));
                    self.current_bytes.clear();
                    self.state = ToolScanState::InItem;
                    self.scan_item_byte(byte);
                }
            }
            ToolScanState::InItem => {
                if !self.in_string && self.depth == 0 && (byte == b',' || byte == b']') {
                    self.finish_item();
                    self.state = if byte == b',' {
                        ToolScanState::BetweenItems
                    } else {
                        ToolScanState::AfterArray
                    };
                } else {
                    self.scan_item_byte(byte);
                }
            }
            ToolScanState::AfterArray => {
                if !byte.is_ascii_whitespace() {
                    self.invalid = true;
                }
            }
        }
    }

    fn scan_item_byte(&mut self, byte: u8) {
        if let Some(current) = &mut self.current {
            current.absorb(&[byte]);
            if self.count < RETAINED_TOOL_DETAILS {
                self.current_bytes.push(byte);
            }
        }
        if self.in_string {
            if self.escaped {
                self.escaped = false;
            } else if byte == b'\\' {
                self.escaped = true;
            } else if byte == b'"' {
                self.in_string = false;
            }
            return;
        }
        match byte {
            b'"' => self.in_string = true,
            b'{' | b'[' => self.depth += 1,
            b'}' | b']' if self.depth > 0 => self.depth -= 1,
            b'}' => self.invalid = true,
            _ => {}
        }
    }

    fn finish_item(&mut self) {
        let Some(current) = self.current.take() else {
            self.invalid = true;
            return;
        };
        let fingerprint = current.finish();
        if self.count < RETAINED_TOOL_DETAILS {
            let bytes = std::mem::take(&mut self.current_bytes);
            self.details
                .push(self.tool_detail(&bytes, &fingerprint.fingerprint));
        } else {
            self.details_omitted.absorb_entry(self.count, &fingerprint);
        }
        if self.count < RETAINED_LEAVES {
            self.retained.push(fingerprint.fingerprint);
        } else {
            self.omitted.absorb_entry(self.count, &fingerprint);
        }
        self.count += 1;
    }

    /// Reduces one serialized tool to its plaintext name plus keyed sub-field
    /// fingerprints; a tool without a parseable string name is retained with an
    /// empty name rather than dropped, so counts stay exact.
    fn tool_detail(&self, item_bytes: &[u8], descriptor: &Fingerprint) -> ToolDetailEntry {
        let parsed: Option<serde_json::Map<String, serde_json::Value>> =
            serde_json::from_slice(item_bytes).ok();
        let field = |key: &str, domain: &str| -> ManifestObservation<Fingerprint> {
            parsed.as_ref().and_then(|object| object.get(key)).map_or(
                ManifestObservation::Status {
                    status: ObservationStatus::Missing,
                },
                |value| {
                    self.fingerprinter.fingerprint_json(domain, value).map_or(
                        ManifestObservation::Status {
                            status: ObservationStatus::Unavailable,
                        },
                        ManifestObservation::Observed,
                    )
                },
            )
        };
        let name = parsed
            .as_ref()
            .and_then(|object| object.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        ToolDetailEntry {
            name,
            descriptor: descriptor.clone(),
            description: field("description", "logical.tools.detail.description"),
            parameters: field("parameters", "logical.tools.detail.parameters"),
        }
    }

    fn finish(self) -> Option<(ListManifest, ToolsDetailManifest)> {
        if self.invalid
            || self.state != ToolScanState::AfterArray
            || self.current.is_some()
            || self.in_string
            || self.depth != 0
        {
            return None;
        }
        let total = self.total.finish().fingerprint;
        let omitted = self.omitted.finish().fingerprint.hmac;
        let details_omitted = self.details_omitted.finish().fingerprint.hmac;
        let detail = ToolsDetailManifest {
            count: self.count,
            omitted: OmittedManifest {
                count: self.count - self.details.len(),
                hmac: details_omitted,
            },
            retained: self.details,
        };
        let list = ListManifest {
            body: total,
            count: self.count,
            omitted: OmittedManifest {
                count: self.count - self.retained.len(),
                hmac: omitted,
            },
            retained: self.retained,
        };
        Some((list, detail))
    }
}

impl Write for ToolManifestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.total.absorb(bytes);
        for byte in bytes {
            self.scan(*byte);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
