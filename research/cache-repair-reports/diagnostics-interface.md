STATUS: DONE
COMMIT: none
CHANGED: 0 files
OPEN: 2
BLOCKERS: none

---

# Pareto cache-diagnostic interface

## Decision

Persist immutable, keyed request and outcome records only. Compare them offline. Remove runtime latest-pointer files, cross-process locks, writer queues, and comparison LRUs.

Use one append-only file per collector run:

`~/.codex/cache-diagnostics/YYYY/MM/DD/run-<random-run-id>.jsonl`

The filename contains no thread identity. Every record carries HMACed thread, session, turn, parent, affinity, response, and previous-response identities. Each process/collector gets a distinct file, so concurrent processes never append to the same file. A process should share one `Arc<Collector>` across its sessions and children; creating a second collector is safe because it gets another run file.

No history file is rotated, truncated, compacted, aged out, quota-limited, or deleted automatically. The archive grows until the user handles it manually. The key file is configuration, not evidence, and must not be replaced silently.

## Evidence seams

The assigned worktree is clean at `ea55207054217cc00a541e385b5ffa8898c87b30`; graph and direct source reads agree, with no recorded coverage gap in the relied-on files.

- HTTP: observe the exact uncompressed `EncodedJsonBody` and prepared compressed body after provider/request header merge in `codex-api/src/endpoint/session.rs:122-155`, before authentication.
- WebSocket: observe the exact string returned by `serialize_websocket_request` in `codex-api/src/endpoint/responses_websocket.rs:235-341,920-923`, immediately before send.
- Logical request: fingerprint the finalized `ResponsesApiRequest` built in `core/src/client.rs:891-997`. Keep this separate from the WebSocket wire payload because `prepare_websocket_request` may send only a delta plus `previous_response_id` (`client.rs:1385-1411`).
- Usage/outcome: link `ResponseEvent::Completed`, failure, cancellation, and fallback in the shared `map_response_events` path (`client.rs:2175-2323`). Local compaction already uses the same stream (`core/src/compact.rs:762-822`).

Only a fixed safe identity projection may cross the API observer boundary: endpoint kind, compression, connection reuse, Responses Lite marker, and values of `session-id`, `thread-id`, `x-client-request-id`, `x-openai-subagent`, `x-codex-routing-hint`, `previous_response_id`, and selected client identity fields. Values are HMACed immediately. Never pass a full header map, auth/cookie/attestation data, arbitrary provider headers, or a URL with query data.

## Minimal workspace interfaces

`codex-api` owns the observer contract because it owns the serializers:

```rust
pub trait ResponsesRequestObserver: Send + Sync {
    fn observe_http(&self, observation: HttpRequestObservation<'_>);
    fn observe_websocket(&self, observation: WebsocketRequestObservation<'_>);
}

pub struct HttpRequestObservation<'a> {
    pub uncompressed_json: &'a [u8],
    pub prepared_body: &'a [u8],
    pub identity: SafeTransportIdentity<'a>,
}

pub struct WebsocketRequestObservation<'a> {
    pub wire_json: &'a [u8],
    pub identity: SafeTransportIdentity<'a>,
    pub connection_reused: bool,
    pub incremental: bool,
}
```

`SafeTransportIdentity` has explicit optional fields, not a map. Observer failures are recorded as content-free local warnings and never fail the model request.

The new private workspace crate `codex-cache-diagnostics` owns privacy, persistence, and analysis:

```rust
impl Collector {
    pub fn open(codex_home: &Path) -> Result<Arc<Self>, OpenError>;
    pub fn start_attempt(
        self: &Arc<Self>,
        context: AttemptContext<'_>,
        logical: &ResponsesApiRequest,
        tools: &[ToolSpec],
    ) -> Arc<Attempt>;
}

impl Attempt {
    pub fn completed(&self, outcome: CompletedOutcome<'_>);
    pub fn terminal(&self, outcome: TerminalOutcome);
}

impl ResponsesRequestObserver for Attempt { /* exact HTTP/WS record */ }

pub fn compare_attempt(root: &Path, target: AttemptId) -> Result<Comparison, ScanError>;
pub fn compare_pair(root: &Path, earlier: AttemptId, later: AttemptId)
    -> Result<Comparison, ScanError>;
```

`AttemptContext` contains request kind, retry ordinal, and raw lineage/cache identities; `start_attempt` HMACs them before any record is built. `CompletedOutcome` contains numeric token usage and a raw response ID that is also HMACed immediately. `TerminalOutcome` is a bounded enum, never provider error text.

Core wiring is limited to constructing the attempt after final logical mutations, passing `Arc<Attempt>` as the API observer, and calling its outcome method from the shared response mapper. Attach a shared collector with `ModelClient::with_cache_diagnostics(...)` from session/thread-manager setup so existing positional constructors remain stable.

## Record and memory bounds

Use HMAC-SHA-256 with one random 256-bit installation key at `cache-diagnostics/.fingerprint-key` (`0600` on Unix; user-private inherited ACL on Windows). Plain hashes are dictionary-guessable and are not acceptable.

Each request record contains exact whole-body HMAC/byte counts, fixed top-level component HMACs, ordered item/tool HMACs, aggregate HMAC/count for omitted tails, safe transport identity HMACs, run ID, sequence, and wall-clock time. Outcome records contain the attempt ID, bounded terminal kind, HMACed response ID, and numeric usage. Plaintext is limited to schema field names and low-cardinality enums/counts; prompt/tool content, dynamic keys, paths, IDs, metadata values, and routing values are never plaintext.

Initial hard bounds:

- 256 KiB maximum JSONL record;
- 2,048 retained item digests and 2,048 retained tool digests;
- one record buffer plus one manifest per active attempt;
- 256 KiB maximum line accepted by the offline scanner; oversized/corrupt lines are skipped with a count;
- synchronous mutex-protected append to the collector's private run file, before network send for request records.

Continue hashing omitted tails with constant memory. Comparisons beyond retained leaves return `indeterminate_after(index)`, never a guessed first difference.

## Offline comparison

`compare_attempt` performs bounded two-pass streaming scans over immutable run files:

1. Find the target request/outcome by random attempt ID.
2. Rescan and select the latest eligible earlier request using retry lineage, then same thread, then same affinity (`HMAC(prompt_cache_key + provider scope)`). Keep only the target and best candidate in memory.
3. Report whole-wire equality, changed fixed components, first retained input/tool difference, append/truncation/divergence relation, transport identity changes, logical-versus-WebSocket-delta relation, and correlated cached-token usage.

File enumeration and line reads must be streaming; do not collect the archive or file list in memory. `compare_pair` bypasses target selection when the caller knows both attempts.

Same-thread ordinary turns and compaction compare by thread HMAC. Cold resume works across run files by the same thread HMAC. Fresh children compare by affinity HMAC while retaining distinct thread HMACs. No runtime pointer is required.

Every verdict is evidence language: a difference is a cache-bust candidate, not proof of cause; identical observed data is not proof of backend fault; provider-reported cached tokens do not prove physical cache reuse.

## Capability and cost of offline comparison

No request evidence or first-difference capability is lost when all immutable records are readable. The real losses are immediacy and lookup speed: a comparison is unavailable until analysis runs, and scan time grows linearly with the retained archive.

Cross-process chronology has one honest limit. Sequence numbers totally order one run, but wall clocks cannot prove order between concurrent processes. If candidate selection depends on overlapping/tied cross-run timestamps, return `order_ambiguous` and require `compare_pair`; never choose silently. Explicit pair comparison remains exact within manifest bounds.

## Coherent commit stages

1. **Record library:** add `codex-cache-diagnostics`, HMAC key handling, bounded immutable writer, two-pass `compare_attempt`/`compare_pair`, privacy/bounds/comparator unit tests, Cargo/Bazel entries and lock refresh. No runtime wiring yet.
2. **Transport observer:** add the small `codex-api` trait and exact HTTP/WS observation calls. API tests use a fake observer to prove bytes and safe identity projection match the transmitted request and never include auth/full headers.
3. **First usable vertical slice:** initialize default-on collection in core, wire ordinary HTTP request/outcome usage, and add two focused integrations: usage correlation/append comparison and model invisibility.
4. **WebSocket slice:** wire logical/full versus incremental/wire observations and add one focused reused-connection test plus one retry/fallback test.
5. **Lifecycle evidence:** add three small independent tests/commits for local compaction, fresh-context child affinity, and cold resume across run files. Reuse existing fixtures rather than one seven-scenario test.

After each code stage run its crate tests. The eventual core change requires `just fmt`, focused `just test -p ...`, `just bazel-lock-update` for dependencies, and user approval before the complete workspace `just test`.

## Open decisions

1. Measure current request shapes before freezing the proposed 256 KiB/2,048-leaf constants; hard bounds are mandatory.
2. Name the emergency opt-out switch. The locally installed build remains enabled by default, with no public config/schema work in the first slice.
