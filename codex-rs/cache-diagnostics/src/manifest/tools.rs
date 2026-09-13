use std::io;
use std::io::Write;

use codex_api::ResponsesApiTools;

use super::Fingerprint;
use super::FingerprintStream;
use super::Fingerprinter;
use super::ListManifest;
use super::ManifestObservation;
use super::ObservationStatus;
use super::OmittedManifest;
use super::RETAINED_LEAVES;

pub(super) fn tools_manifest(
    fingerprinter: &Fingerprinter,
    tools: Option<&ResponsesApiTools>,
) -> ManifestObservation<ListManifest> {
    let Some(tools) = tools else {
        return unavailable(ObservationStatus::Missing);
    };
    let mut writer = ToolManifestWriter::new(fingerprinter);
    if serde_json::to_writer(&mut writer, tools).is_err() {
        return unavailable(ObservationStatus::Unavailable);
    }
    writer.finish().map_or_else(
        || unavailable(ObservationStatus::Unavailable),
        ManifestObservation::Observed,
    )
}

fn unavailable(status: ObservationStatus) -> ManifestObservation<ListManifest> {
    ManifestObservation::Status { status }
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
    retained: Vec<Fingerprint>,
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
            retained: Vec::with_capacity(RETAINED_LEAVES),
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
        if self.count < RETAINED_LEAVES {
            self.retained.push(fingerprint.fingerprint);
        } else {
            self.omitted.absorb_entry(self.count, &fingerprint);
        }
        self.count += 1;
    }

    fn finish(self) -> Option<ListManifest> {
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
        Some(ListManifest {
            body: total,
            count: self.count,
            omitted: OmittedManifest {
                count: self.count - self.retained.len(),
                hmac: omitted,
            },
            retained: self.retained,
        })
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
