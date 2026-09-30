STATUS: DONE
COMMIT: none
CHANGED: 0 files
OPEN: 4
BLOCKERS: none

---

# Smallest trustworthy cache-request diagnostic

## Decision

Add an always-on, local-only request diagnostic to the locally installed build. It must observe the finalized Responses request at the two actual serialization seams, persist only keyed fingerprints and structural counts, and attach the provider's later token-usage result to the same request attempt.

The first coherent slice should cover HTTP, WebSocket, ordinary turns, local compaction, fresh-context children, and cold resume. These cases already converge on `ModelClientSession::stream`, so one request observer plus one completion hook covers them without adding diagnostics to model history or to every caller.

Do not begin with rollout-only analysis. The existing rollout scanner is useful for finding cache-drop witnesses, but the research ledger correctly records that rollouts omit the full serialized request, tool array, cache key, and transport identity. It cannot name the first local difference.

The earlier ledger proposed deleting data after 30 days or 100 MiB. The user's current instruction supersedes that policy: there must be no age cutoff, no disk quota, and no automatic deletion. Each record and all live memory still have hard bounds. This means the directory can grow until the user archives or removes it manually.

## Source evidence at the assigned revision

The worktree is clean at `ea55207054217cc00a541e385b5ffa8898c87b30`. The codebase-memory index records the same commit, and direct reads in `/private/tmp/codex-cache-design` matched the indexed source. Coverage checks reported no recorded gap for all 20 source and test files relied on below. That remains a best-effort graph signal; the cited worktree source is the authority.

- `core/src/client.rs:891-997` builds the complete `ResponsesApiRequest`: model, base instructions, formatted input, tools, parallel-tool setting, reasoning, service tier, prompt cache key, text controls, client metadata, and access programs. Responses Lite moves tools and base instructions into leading input items.
- `core/src/client.rs:1562-1708` performs final HTTP mutations: guardian metadata, endpoint-specific service tier, routing hint, access programs, and response-item preparation, then calls `ApiResponsesClient::stream_request`.
- `codex-api/src/endpoint/responses.rs:102-129` serializes the finalized HTTP request with `EncodedJsonBody::encode`, adds `x-client-request-id`, `session-id`, `thread-id`, and subagent headers, then sends it. `http-client/src/request.rs:15-37` exposes the encoded bytes without another copy.
- `codex-api/src/endpoint/session.rs:122-155` merges provider and request headers, prepares compression once, and then applies authentication immediately before transport. The diagnostic must observe the prepared request before authentication and must receive only an explicit safe-header projection, never the full map.
- `core/src/client.rs:1725-1928` builds the WebSocket payload after incremental-request selection, item-ID preparation, guardian metadata, client metadata, and the request-start stamp. `codex-api/src/endpoint/responses_websocket.rs:235-341` serializes that exact payload before sending it.
- `core/src/client.rs:1385-1411` may replace the WebSocket full input with `previous_response_id` plus an incremental suffix. Therefore the diagnostic needs both the logical full request and the actual wire payload. Treating the delta as the full prompt would create false prefix failures.
- `core/src/client.rs:1235-1269` puts session/thread/request and routing identities in WebSocket handshake headers. `codex-api/src/endpoint/responses_websocket.rs:490-503` defines the final header precedence. These safe identities must be fingerprinted for both reused and new connections.
- `codex-api/src/sse/responses.rs:353-549` turns both HTTP SSE and WebSocket completion messages into `ResponseEvent::Completed` with `TokenUsage`. `core/src/client.rs:2175-2323` is their shared completion/failure/cancellation mapping point.
- `core/src/compact.rs:762-822` calls the same `ModelClientSession::stream` path for local compaction and consumes the same completed usage event. No compaction-specific writer is needed.
- `core/src/responses_metadata.rs:153-175,220-252,307-374` already provides request kind, session/thread/turn lineage, fresh-versus-fork markers, and the metadata projected into body and compatibility headers.
- Existing integration seams already exercise the required lifecycles: `core/tests/suite/compact.rs` for local compaction, `agents_md.rs:1438` for a fresh child without parent history, `multi_agent_resume.rs:168` and `core/src/session/tests.rs:6973-7039` for resume identity, `client_websockets.rs` for WebSocket transport, and `prompt_cache_key.rs` for cache-key/session identity.

The research document names snapshot `ffabed54df`; its mechanism conclusions are useful, but implementation must use the newer assigned revision above.

## Placement and ownership

Use a small new private workspace crate, `codex-rs/cache-diagnostics`, rather than growing `codex-core` with hashing, comparison, persistence, and retention concerns. The crate owns:

- HMAC key creation/loading;
- bounded request manifests;
- comparison and append-prefix classification;
- the bounded asynchronous JSONL writer and latest-record pointers;
- request/outcome record schemas;
- unit tests for privacy, bounds, and comparison.

`codex-api` should own only a narrow observer contract and the two transport hook calls because it owns the real serializers. It must not own filesystem paths or retention. `codex-core` should own only lifecycle wiring: constructing an attempt from the finalized logical request and lineage metadata, passing the observer to the transport, and reporting completed/failed/cancelled outcomes.

This adds files rather than embedding a large block in the high-touch `core/src/client.rs`. The required edits there are limited to creating/passing an attempt handle and giving that handle to `map_response_stream`. Session startup can attach the service through a builder such as `ModelClient::with_cache_diagnostics(...)`, avoiding a new positional parameter at every `ModelClient::new` call site.

Expected file footprint:

| Area | Change |
|---|---|
| `codex-rs/cache-diagnostics/` | New crate: record types, fingerprinting, comparison, writer, tests |
| `codex-rs/codex-api/src/request_observer.rs` | Trait and safe observation structs; no persistence |
| `codex-rs/codex-api/src/endpoint/responses.rs` and `endpoint/session.rs` | Pass observer to the exact encoded HTTP body and prepared safe transport projection |
| `codex-rs/codex-api/src/endpoint/responses_websocket.rs` | Observe the exact serialized `response.create` string and safe connection identity |
| `codex-rs/core/src/client.rs` | Start attempt, pass observer, report completion/failure/cancellation |
| `codex-rs/core/src/session/session.rs` | Initialize from `codex_home/cache-diagnostics` by default in the local binary |
| `codex-rs/core/tests/suite/cache_diagnostics.rs` | Focused end-to-end coverage |
| Cargo/Bazel manifests | Add the crate and direct `hmac`/`sha2` dependencies; refresh `Cargo.lock` and `MODULE.bazel.lock` |

Do not add a `ContextualUserFragment`, `ResponseItem`, `RolloutItem`, inference-trace item, or session-history entry. Sidecar records must be reachable only from the observer/writer, so there is no path back into a later model request or resumed history.

## Exact observation seams

### Logical request

In each HTTP/WebSocket attempt loop, create `CacheDiagnosticAttempt` only after all request mutations and `prepare_response_items_for_request` have completed. Give it:

- the finalized full `ResponsesApiRequest`;
- the source `ToolSpec` slice, so individual tools can be fingerprinted without parsing or retaining schemas;
- request kind (`turn`, `prewarm`, `compaction`, `memory`);
- lineage flags: thread, session, turn, parent thread, fork markers;
- provider/endpoint, model, transport plan, and retry ordinal.

For WebSocket this logical manifest remains the full request even when the wire payload later becomes incremental.

### HTTP wire request

Pass the attempt observer through `ResponsesOptions` into `EndpointSession::stream_encoded_json_with`. Immediately after provider/request header merge and `Request::into_prepared`, but before `auth.apply_auth`, call the observer with:

- the original uncompressed `EncodedJsonBody::as_bytes()`;
- the prepared compressed-body byte length and keyed digest, when compression is active;
- method and endpoint kind;
- only allowlisted cache/routing identity values selected by `codex-api`.

The allowlist is `session-id`, `thread-id`, `x-client-request-id`, `x-openai-subagent`, `x-codex-routing-hint`, the Responses Lite marker, and connection/request compression state. Values are transient inputs to HMAC and never serialized as plaintext. Never pass `Authorization`, cookies, attestation, API keys, arbitrary provider headers, URLs with query strings, or the full `HeaderMap` to the diagnostics crate.

This seam captures the actual JSON serialization and provider/request header precedence. Authentication remains deliberately outside the observer boundary.

### WebSocket wire request

Keep the safe allowlisted handshake projection on `ResponsesWebsocketConnection` after final header merge. When `ResponsesWebsocketConnection::stream_request` has called the existing `serialize_websocket_request`, invoke the attempt observer before `send_websocket_request` with:

- the exact UTF-8 string bytes that will be sent;
- endpoint, connection-reused flag, and safe handshake projection;
- `previous_response_id` presence and keyed value;
- `generate=false` for prewarm;
- safe, explicitly named client-metadata identity values.

The full logical and wire manifests are linked by one attempt ID. A second request over a reused connection can therefore report “logical input is append-only; wire request is an incremental suffix referencing a prior response” instead of calling the different wire body a bust.

### Outcome and usage

Carry `Arc<CacheDiagnosticAttempt>` into `map_response_events`. Write one linked outcome record on every terminal path:

- completed: HMAC of response ID plus input, cached-input, cache-write, output, reasoning-output, and total token counts;
- failed: bounded error category and HTTP status, never response body text;
- cancelled/stream closed: bounded enum reason;
- fallback: explicit WebSocket-to-HTTP terminal status followed by a new HTTP attempt ID.

Each unauthorized retry and each higher-level sampling retry is a distinct attempt because it can have a different final request, auth recovery state, or transport. `turn_id`, request kind, and a monotonically increasing retry ordinal link them without conflating sends.

## Privacy-safe record design

Use HMAC-SHA-256 with one random 256-bit installation key. Plain SHA-256 is insufficient because short prompts, tool names, and common schemas can be guessed from a dictionary. Create the key once with exclusive creation under `codex_home/cache-diagnostics/.fingerprint-key`, mode `0600` on Unix and inherited user-only directory permissions on Windows. Record a short key ID, never the key. If the key is missing after older records exist or is malformed, stop recording and emit one content-free warning; do not silently replace it and make old/new fingerprints look comparable.

The JSONL record may contain plaintext only for schema names and low-cardinality structural enums: schema version, event kind, request kind, transport, endpoint kind, compression, item variant, counts, byte lengths, indices, timestamps, and comparison verdicts. The following are always HMACed before enqueue:

- instructions, messages, reasoning, tool calls, tool results, image/file references, and all other input payloads;
- tool names, descriptions, schemas, dynamic schema keys, arguments, and outputs;
- cache keys, session/thread/turn/response/previous-response/window/connection IDs;
- routing hints, subagent names/roles, provider-supplied identity values, paths, URLs, and metadata values.

Credentials and non-allowlisted headers are never presented to the diagnostics crate at all. No prompt, tool, header, or metadata plaintext may enter an error message. Serialization errors report only an enum category.

Suggested request line, shown with placeholder digests:

```json
{"schema":1,"event":"request","attempt":"a-17","time_unix_ms":0,"kind":"turn","transport":"http","logical":{"body_hmac":"h1:...","bytes":1234,"components":[{"field":"instructions","hmac":"h1:...","bytes":400}],"input":{"count":5,"items":[{"index":0,"kind":"message","hmac":"h1:...","bytes":100}],"overflow":null},"tools":{"count":2,"items":[{"index":0,"hmac":"h1:...","bytes":300}],"overflow":null}},"wire":{"body_hmac":"h1:...","bytes":1234,"compression":"none"},"identity":{"cache_key":"h1:...","session":"h1:...","thread":"h1:..."},"comparison":{"target":"a-16","relation":"same_thread","first_difference":{"scope":"input","index":3},"input_relation":"prior_is_prefix","wire_equal":false}}
```

The actual schema should use typed Rust structs with `deny_unknown_fields` in tests and a version field. Never persist arbitrary JSON copied from the request.

## Comparable manifests and first differences

Compute manifests directly from typed fields with a streaming HMAC writer; do not build a duplicate `serde_json::Value` tree. For each request retain:

1. HMAC and byte count for the exact uncompressed serialized body.
2. HMAC and byte count for every fixed top-level field in the struct's serialized order.
3. Ordered per-item HMACs for input and ordered per-tool HMACs for the source tool list.
4. An aggregate HMAC and count for any item/tool tail omitted by the record cap.
5. A separate safe transport-identity map with fixed field names and HMACed values.

Compare in a fixed order and report all changed component names plus one `first_difference`:

- exact wire body equal/different;
- fixed component equality (`model`, `instructions`, `input`, `tools`, `parallel_tool_calls`, reasoning, service tier, text controls, cache key, metadata, access programs, and the remaining serialized fields);
- input relation: `equal`, `prior_is_prefix`, `current_is_prefix`, `diverged_at(index)`, or `indeterminate_after(index)`;
- tool relation with the same states;
- first changed transport identity field;
- WebSocket logical-versus-incremental relation.

“First” means the first observable component or item in this declared serialization order. It is not claimed to be the backend's cache-key order or the cause of a miss. If a difference lies in an overflow tail, say `indeterminate_after`, never guess an index. If exact body HMACs differ while all retained manifests match, report `unlocalized_wire_difference`; likely causes include an omitted tail or map ordering.

The append-prefix check is exact only over fully retained item/tool sequences. Aggregate tail equality proves whole-tail equality but cannot locate a difference inside it. This is the honest tradeoff that keeps every record bounded.

## Comparison target selection

Maintain bounded latest-record pointers in addition to immutable history:

- history: `~/.codex/cache-diagnostics/YYYY/MM/DD/<thread-id>.jsonl`;
- same-thread pointer: `~/.codex/cache-diagnostics/latest/threads/<thread-id>.json`;
- affinity pointer: `~/.codex/cache-diagnostics/latest/groups/<HMAC(prompt_cache_key + provider scope)>.json`.

Pointers are atomically replaced, but every version remains in the JSONL history; replacing a pointer does not delete diagnostic history. The pointers avoid scanning an unbounded archive on cold resume and across fresh child threads. Lock pointer read/compare/replace within the process. Across processes, use a small advisory lock for the group pointer or record `comparison_raced=true` when a compare-and-replace detects a newer target.

Target precedence:

1. same attempt lineage for a transport/auth retry;
2. latest same-thread request for ordinary turns, compaction, and cold resume;
3. latest same-affinity request for a fresh child or related thread;
4. no comparator, with a reason enum.

Classify a cold resume when the same-thread pointer exists but has a different process-run ID. Classify a fresh child from `parent_thread_id` with no fork-history marker. Prewarm records are kept but excluded as comparison targets for inference usage unless explicitly requested.

## Hard bounds and persistence behavior

Recommended starting constants, to be measured before implementation is finalized:

- maximum serialized JSONL line: 256 KiB;
- maximum retained input-item digests: 2,048;
- maximum retained tool digests: 2,048;
- bounded writer queue: 32 records, at most 8 MiB by construction;
- maximum loaded pointer: 256 KiB;
- process comparison LRU: 256 manifests, at most 64 MiB by construction;
- maximum error/category text: enums only, no provider strings.

Continue hashing omitted tails with constant memory so the root digest and count remain complete. Before sending a provider request, enqueue its already-redacted record and await the append acknowledgement. This makes “request was sent but its diagnostic was still buffered” unlikely. A persistence failure must never block the user's model request indefinitely or inject an error into the conversation: emit one bounded, content-free local warning and mark the diagnostics service unhealthy. Outcome writes can use the same bounded queue and acknowledgement.

Do not rotate, truncate, age out, quota, compact, or delete history automatically. The only overwritten files are the reproducible latest-record pointers. This satisfies the user's override but creates a real long-term disk-growth risk. A read-only status command may report bytes/files; cleanup remains a manual user decision.

## Tests that make the slice trustworthy

### Unit tests in `codex-cache-diagnostics`

- **No plaintext escape:** construct a request containing unique canaries in instructions, user text, tool name/description/schema/property, arguments, output, path, routing header, IDs, credential-shaped values, and error text. Serialize request and outcome records and assert none of the canaries or full request/header JSON appears.
- **Deterministic keyed comparison:** a fixed test key produces stable digests across processes; a different key produces different digests and a different key ID.
- **Bounds:** inputs and tool arrays beyond each cap keep the line at or below 256 KiB, the queue/LRU bounds hold, and comparison returns `indeterminate_after` rather than a false exact location.
- **Comparator:** cover equal, strict append, strict truncation, first-item divergence, first-tool divergence, fixed-field change, transport-identity change, exact-wire-only change, and WebSocket incremental suffix.
- **Persistence:** request is appended before acknowledgement; linked outcome is appended later; partial final line is ignored on pointer recovery; same-thread and affinity pointers restore comparison without archive scans; no retention/deletion code exists.
- **Key failure:** malformed or unexpectedly missing key disables recording without replacement and without emitting request content.

### Integration tests in `codex-core`

Use `TestCodexBuilder::build_with_auto_env()` and the existing response mocks.

1. **Ordinary HTTP turns:** issue two turns with completed events that include input/cached/cache-write counts. Assert request/outcome pairing, exact usage values, stable instructions/tools, and the expected input prefix relation. Read the sidecar directly; do not add test-only production accessors.
2. **Local compaction:** normal request, manual local compact, then follow-up. Assert all three attempts are present, compaction is classified, usage is correlated, and the comparator reports actual component differences. Do not assert the known tool-parity defect as desired behavior.
3. **Fresh-context child:** adapt the existing fresh-child fixture. Assert parent and child share the affinity group, have distinct thread fingerprints, child is classified fresh, parent task history is absent from the provider request, and the diagnostic names the maximum retained common prefix without containing either task in plaintext.
4. **Cold resume:** create a thread, shut it down, clear the live diagnostics service, resume from rollout, and send a turn. Assert comparison came from the on-disk same-thread pointer and is classified resume.
5. **WebSocket:** exercise first full request and second incremental request. Assert both logical manifests exist, the exact sent WebSocket payload digest is recorded, the second wire record carries previous-response identity and connection reuse, and logical append comparison is separate from wire-delta comparison.
6. **Retry/fallback:** make WebSocket fall back to HTTP or trigger one recoverable HTTP retry. Assert distinct attempt IDs and terminal outcomes, with one usage record attached only to the successful send.
7. **Model invisibility:** after writing diagnostics, inspect the next outbound Responses body, rollout JSONL, reconstructed resume history, and inference-trace fixture. Assert no diagnostic schema/event/attempt data appears in any of them.

Run `just fmt`, `just test -p codex-cache-diagnostics`, `just test -p codex-api`, and the focused `codex-core` integration tests. Because this touches `codex-core`, the repository rules require asking before the complete `just test`; do that only after focused tests pass. Dependency changes also require `just bazel-lock-update`, and the new crate needs a minimal `BUILD.bazel`.

## Interpretation rules

Every report and CLI view must use these exact epistemic limits:

- A changed request field, item, tool, cache key, or transport identity is a cache-bust candidate, not proof that it caused the observed cached-token result.
- Equal observed body and identity fingerprints are evidence that this client did not expose a difference. They are not proof of a backend fault; unobserved provider routing, cache eviction, authorization scope, concurrency, or backend policy may still explain the result.
- `cached_input_tokens` and cache-write tokens are provider-reported accounting. They do not prove physical KV-cache reuse.
- WebSocket incremental equality is judged against its logical full request and previous-response chain, not by demanding identical wire bodies.
- Comparisons made with omitted manifest tails, failed persistence, missing usage, a changed HMAC key, or a pointer race must be labeled incomplete.

## Smallest coherent first slice

Implement only the version-1 request/outcome records, keyed manifests, same-thread/affinity pointers, HTTP and WebSocket hooks, shared completion correlation, and the seven focused tests above. Keep the installed local binary enabled by default and provide at most an emergency environment-variable opt-out; do not add a public config/schema surface in this slice.

Do not add a UI, network upload, automatic incident verdict, retention worker, full archive rescan, canonical JSON rewriter, provider-specific fix, or cache-prefix behavior change. Once this slice produces real normal/compaction/fresh-child/resume records, use those records to choose the first causal fix.

## Open decisions

1. **HMAC key storage:** recommended first slice is a user-private file for cross-platform simplicity. macOS Keychain would protect the key better at rest but adds platform-specific behavior and makes Linux/Windows parity harder.
2. **Exact cap values:** the proposed 256 KiB/2,048-item/32-record/256-manifest values are safe initial bounds, but should be confirmed against a sample of current local requests before coding them as constants. The existence of hard caps is not optional.
3. **Emergency opt-out name:** default-on is required for the installed local build. Choose one content-free environment switch only if operational recovery from a broken filesystem writer is needed; avoid a public config/schema change in the first slice.
4. **Cross-process group locking:** advisory locking gives the strongest fresh-child/concurrent-session target selection; compare-and-replace plus an explicit race marker is smaller. Either is acceptable only if a race is never presented as an exact comparison.
