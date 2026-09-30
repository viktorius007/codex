PASS: the integrated follow-up source correctly covers warmup, legacy and v2 compaction, outer retry ordinals, and real HTTP fallback without fabricating handshake attempts.

Target: incremental lifecycle source integrated in `/private/tmp/codex-cache-integration` over the previously reviewed first slice, base commit `3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18`.

Pinned review-scope digest: `09a9e104ee2f49323f8c199913b4ba7f4f8af16441e19beb6f049d399df6c05f` (SHA-256 of the ordered SHA-256 listing for the ten relied-on production files).

Method: incremental read-only source review of the builder's warmup, legacy/v2 compaction, outer retry, and HTTP fallback wiring, plus caller tracing for the reset-on-call public stream wrapper. No Cargo command, build, test, formatter, repository edit, or commit was run. The parent supplied the prior evidence that the first-slice 56 tests and both transport tests were green.

The prior three test gaps are being handled by the active lifecycle test author and are not repeated here. This is a source-only verdict; the follow-up still needs its assigned compile and runtime evidence.

## Finding resolved during review

### Local compaction now shares one sequencer across its outer retry loop

- The initial incremental source reset every local-compaction retry to ordinal 0 because `drain_to_completed` called the public `stream()` wrapper, which creates a fresh sequencer per call.
- The coordinator integrated the minimal correction before this report was finalized. `run_compact_task_inner_impl` now creates one sequencer before its retry loop (`core/src/compact.rs:275-289`), passes it through each `drain_to_completed` call (`:304-311`), and that helper calls `stream_with_diagnostic_attempts` with the shared reference (`:777-799`).
- The corrected path preserves concurrent compaction/history work and now matches sampling and remote-v2 retry lineage. No unresolved production finding remains in the requested follow-up scope.
- The active lifecycle tests should retain the discriminator: one retryable local-compaction failure followed by success must yield `requestKind: "compaction"` ordinals `[0, 1]`, with one failed and one completed pair.

## Incremental checks that pass

| Criterion | Source verdict |
|---|---|
| Warmup records only an actual send | PASS. WebSocket connect/426/auth handling finishes before the attempt is created (`core/src/client.rs:1867-1903`, attempt at `:1955-1973`). A 426 handshake therefore produces no fabricated WebSocket attempt. An actual `generate=false` send receives an explicit `warmup` attempt even if metadata is stale. |
| Warmup terminal lifecycle | PASS. `prewarm_websocket` drains through `Completed`, returns the propagated stream error, or reaches premature close (`core/src/client.rs:2072-2123`). The shared response wrapper records completed/error/close, and final-handle drop supplies cancellation with Stage R's existing at-most-once arbitration. |
| No duplicate handshake/fallback record | PASS. Handshake fallback happens before sequencer consumption, then the real HTTP request consumes ordinal 0. A sent WebSocket that fails consumes and finalizes its own ordinal; an outer retry/fallback sends a new HTTP request with the next ordinal. No code emits a synthetic fallback request or mutates the failed outcome. |
| Sampling outer retries and HTTP fallback | PASS. `run_sampling_request` creates one sequencer outside its retry loop (`core/src/session/turn.rs:1457-1463`) and passes it through every attempt (`:1487-1498`, `:2294-2340`). Both WebSocket and HTTP paths consume from that same sequencer (`core/src/client.rs:1955-1973`, `:1700-1708`), including 401 recovery. |
| Remote compaction v2 retries | PASS. One sequencer is created outside the v2 retry loop and reused on every stream call (`core/src/compact_remote_v2.rs:372-422`). `CodexResponsesRequestKind::Compaction` is supplied by the v2 request builder, so HTTP and WebSocket records retain the compaction category and completed response usage. |
| Legacy `/responses/compact` observation | PASS. Core fingerprints the finalized prepared `CompactionInput`, attaches one attempt, and finalizes success without fabricated response ID/usage or failure on any API/parse error (`core/src/client.rs:645-738`). The endpoint prepares once, observes the exact prepared body before auth, and clones those bytes for transport (`codex-api/src/endpoint/compact.rs:51-109`; `codex-api/src/endpoint/session.rs:122-157`). |
| Legacy compact-model fallback | PASS. The primary and actual fallback calls receive ordinals 0 and 1 respectively (`core/src/compact_remote.rs:215-253`); no record is created unless each call reaches the request-attempt boundary. |
| WebSocket lifecycle preservation | PASS. The follow-up does not add a connection reset or call `reset_client_session`. Existing transport fallback still owns the pre-existing reset. Per-send observation remains on the reused connection, so diagnostic attachment does not change incremental WebSocket state. |
| Instruction-prefix preservation | PASS. The diagnostic additions in `core/src/session/turn.rs` are limited to the sequencer import/local/parameter threading. Concurrent pending-input and compaction-prefix edits remain intact and are not coupled to diagnostics. |

## Deferred/unwired adjacent surface

`RequestKind::Memory` is mapped in `core/src/cache_diagnostics.rs:224-230`, but the actual detached memory Responses path constructs a fresh `ModelClient` without `.with_cache_diagnostics(...)` (`memories/write/src/runtime.rs:252-294`). Memory requests therefore produce no diagnostic record today. Memory was outside this follow-up's requested warmup/compaction/retry/fallback gate, so I did not make it a blocking finding; the builder report should describe the category as reserved until that separate client-construction path is wired.

## Limits and hygiene

- Codebase-memory metadata matched the relied-on existing core/API files; the new adapter and observer files were absent from the index and were read directly in full. Graph results remain best effort.
- `git diff --check` passed at review time. The intentionally dirty shared worktree was unchanged by this verification.
- About 18 GiB was free at the final check. I created no build artifacts or temporary source tree and removed no shared or unknown files.
