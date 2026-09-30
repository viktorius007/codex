PASS — Stage R bounded-record semantics by source audit
LANDING STATUS: CORRECTIONS PENDING — one visibility warning and two focused proof gaps remain; the planned size split and repository gates remain separate.
Target: integrated dirty patch at base `fdca0be4f4dedc8bf299f66b569f2cb9f409e858` in `/private/tmp/codex-cache-integration`.
Prior tests: `records-green-2.log` — `7 tests run: 7 passed, 0 skipped`; `transport-green-2.log` — `2 tests run: 2 passed, 0 skipped`.
Current run evidence: `core-diagnostics-green-2.log` compiled the extended crate but was still running when reviewed; it exposed the visibility warning below.
Checks: `git diff --check` clean; the new/changed Stage R files were read directly because graph coverage reports their metadata missing.

## Extension criteria -> evidence

| Criterion | Evidence | Verdict |
|---|---|---|
| Only allowlisted transport facts persist | public input is the closed `TransportMetadata` enum and nine-field `TransportIdentity` at `lib.rs:137-216`; persisted shape is the closed `TransportManifest` at `manifest/transport.rs:11-66` | PASS |
| Identity values are never plaintext | all nine optional byte slices pass immediately through distinct HMAC domains at `manifest/transport.rs:68-130`; wire body is HMACed at `manifest.rs:243-276` | PASS |
| Inapplicable and absent facts stay explicit | HTTP marks connection/incremental unavailable; WebSocket marks compression unavailable at `manifest/transport.rs:21-51`; missing identity values serialize `missing` at `120-130`; legacy wire observations serialize transport `unavailable` at `manifest.rs:233-240` | PASS |
| Exact signed usage and absent usage | all six fields are `i64` at `lib.rs:224-240`; `completed_without_usage` records terminal `completed` with `usage: null` rather than fabricated zeroes at `lib.rs:301-365,402-415` | PASS by source; focused tests pending |
| Key scope and dated archive preserved | `keyScope` remains on request/outcome at `lib.rs:378-415`; UTC archive creation remains at `writer.rs:26-38` | PASS |
| Added manifest cannot silently cross 256 KiB | only two dynamic retained arrays exist, each capped at 1,024; each serialized SHA-256 fingerprint is at most 83 bytes on 64-bit targets, so both arrays plus separators are under 168 KiB. All remaining fingerprints, field names, counts, IDs, statuses, enums, and booleans are fixed-count and conservatively keep the full request below 192 KiB; writer rejects at 256 KiB at `writer.rs:40-58` | PASS by static bound |
| Indefinite preservation | extension introduces no removal, rotation, truncation, age, or quota path | PASS |

## Exact remaining corrections

1. **Fix before lint/landing — `private_interfaces` warning.** The current compile log reports that `WireManifest::Observed::transport` is reachable at `pub(crate)` while `TransportManifest` is only `pub(super)` (`manifest.rs:213-220`; `manifest/transport.rs:11-19`). Align those visibilities at the narrowest level that still lets the crate root use `WireManifest`; rerun the scoped crate lint. This is a code-shape warning, not a privacy or persistence failure.

2. **Add one Stage R transport privacy/bounds test.** `records_tests.rs` is byte-identical to the pre-extension suite and exercises only legacy `observe_wire_request`; its 10,000-item/tool bound case terminates with a missing wire observation. The core integration assertions at `core/tests/suite/cache_diagnostics.rs:140-188` check `wire.kind`, body HMAC/bytes, linkage, and usage, but never inspect `wire.transport`. Extend the existing large-record test or add one focused test that calls `observe_transport_request` with all nine identity canaries, asserts no canary/plaintext survives, checks the closed HTTP/WebSocket fields and explicit unavailable statuses, and proves the resulting full line remains present and `<= 256 KiB`. This converts the static no-silent-loss proof into executable regression evidence.

3. **Add one outcome-shape test for the two new semantics.** Record a completed outcome with negative values in all six `i64` usage fields and assert exact preservation; record `completed_without_usage` and assert `terminal == "completed"`, linked attempt/response HMAC, and `usage == null`. Existing positive-value tests cannot distinguish signed preservation from a lossy unsigned/clamping adapter, and no current test calls `completed_without_usage`.

## Cleared files and scope

- Fully read and cleared for record semantics: `cache-diagnostics/src/lib.rs`, `manifest.rs`, new `manifest/transport.rs`, unchanged `manifest/tools.rs`, `writer.rs`, `key.rs`, and crate/workspace manifests.
- No arbitrary/raw data persistence route exists in the Stage R API: no URL, header map, arbitrary provider metadata map, auth/cookie/attestation field, provider error text, prompt/tool plaintext, or response-id plaintext is accepted by the transport manifest.
- Core lifecycle ownership, retry/fallback behavior, observer attachment, and collector sharing were not re-reviewed here.

## Pending landing work, separate from semantic correctness

- Keep the planned R1/R2 split; the extension increases production Rust size further and does not remove the repository's 800-line landing constraint.
- Complete the currently running focused tests, `just bazel-lock-update`, scoped lint/fix, and commit gates after the three corrections.
- Cleanup check: this read-only review created no build artifacts and updated only this requested report.
