STATUS: CLEAR — no proven in-scope defect
COMMIT: d5f8ef066a86255fc1e84c1b2a9759c222aa9f4d
CHANGED: report.md; findings.json
OPEN: None in the assigned diagnostics/request-observer slice.
BLOCKERS: None. No build requested because source comparison produced no defect candidate requiring a red test.
FILES_REVIEWED: codex-api request observer, HTTP and WebSocket endpoints, session/telemetry; core cache diagnostics and client; cache-diagnostics collector, manifest, key, writer; memory startup; relevant request and test callers. Residual production diff classified below.
PROVEN_DEFECTS: 0
---

## Audit boundary and behavior

Reviewed the local patch against `rust-v0.156.1` for changes that could alter provider request bytes, lose a usable WebSocket continuation, or cause extra model turns. The HTTP path constructs the same `RequestBody::EncodedJson` and prepares it once before retry as upstream; the new observer receives the prepared request after `into_prepared` and before `apply_auth` (`codex-rs/codex-api/src/endpoint/session.rs:122-158`, `codex-rs/codex-api/src/endpoint/responses.rs:173-212`, `codex-rs/codex-api/src/telemetry.rs:68-101`). `Request::into_prepared` shares prepared body bytes across clones (`codex-rs/http-client/src/request.rs:122-152`). No diagnostic field is added to the request.

The WebSocket observer receives the already serialized frame immediately before the existing send (`codex-rs/codex-api/src/endpoint/responses_websocket.rs:286-300,919-960`). The connection's route remains `/responses` for the core caller; `stream_request_observed` adds only an observer to that send (`codex-rs/core/src/client.rs:2219-2236`). The new continuation report is built alongside the decision: the comparison covers the same non-input properties and input-prefix equality as upstream, and the sole call passes `allow_empty_delta=true`, preserving an empty incremental suffix (`codex-rs/core/src/client.rs:337-419,1501-1615`). On mismatch, `input_item_divergence` only serializes the rejected pair for a diagnostic offset; it does not choose the request mode (`client.rs:479-495,1540-1554`).

The collector fingerprints logical and wire representations and writes local records. It returns no value used to build a provider request (`codex-rs/cache-diagnostics/src/lib.rs:47-91,386-511`; `codex-rs/core/src/cache_diagnostics.rs:53-114,171-203`). Collector opening failures disable diagnostics (`codex-rs/core/src/cache_diagnostics.rs:53-58`; `codex-rs/core/src/client.rs:662-665`), and append failures are discarded (`codex-rs/cache-diagnostics/src/lib.rs:418-426,478-502`). The HTTP and WebSocket request observers therefore do not introduce a retry or model-turn branch. The diagnostic stream wrapper forwards each original poll result unchanged to `map_response_events` (`codex-rs/core/src/client.rs:2525-2564`). Memory startup enables the same collector on its model client (`codex-rs/memories/write/src/runtime.rs:339-344`), without changing its request construction.

Existing tests cover exact prepared HTTP bytes and WebSocket frames against independent captured sends (`codex-rs/codex-api/tests/request_observer.rs:256-340,414-550`), unavailable diagnostic storage (`codex-rs/core/tests/suite/cache_diagnostics.rs:360`), and failed-stream retry, HTTP fallback, and cancellation records (`codex-rs/core/tests/suite/cache_diagnostics_lifecycle.rs:90,202,278,347`). These tests were read, not run. No live provider test or Cargo build was run, consistent with the brief's build-allocation requirement and the absence of a concrete red-test candidate.

## Remaining local production changes

* **Relevant, covered by other assigned slices:** pre-turn compaction projection and Guardian budget/error handling in `codex-rs/core/src/session/turn.rs:178-190,341-365,1308-1344` can change when or how many model calls occur. Tool inclusion in local compaction at `codex-rs/core/src/compact.rs:296-309` can change provider tool bytes. The code-mode `wait` description in `codex-rs/code-mode-protocol/src/description.rs:39-47` is model-visible guidance that can affect polling behavior. These are outside the observer/diagnostics scope.
* **Instrumentation-only:** `ToolCatalogChanged` protocol event and its rollout persistence/trace classification (`codex-rs/protocol/src/protocol.rs:1399-1404,2146-2168`, `codex-rs/rollout/src/policy.rs:116-123`, `codex-rs/rollout-trace/src/protocol_event.rs:430-463`) record catalog changes without entering a provider request. Cache analyzer scripts and research documents are offline.
* **No request-path effect in reviewed hunks:** local Cargo/V8 and nextest configuration, SemVer `+` metric tag handling, and test-only TUI/V8 adjustments. The TUI version override is gated by `cfg!(test)` (`codex-rs/tui/src/version.rs:1-6`).

## Limits

`codebase-memory` coverage returned `no_recorded_issue`/`metadata_match` for relied-on paths, but Rust semantic coverage is partial (binding oracle, expanded calls, and impl relationships unavailable). I used the actual source and upstream diff for the material call boundaries. This is a source audit of the assigned slice, not a full workspace behavioral test. No production files or tests were edited.
