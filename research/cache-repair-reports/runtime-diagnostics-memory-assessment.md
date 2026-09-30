PASS: the detached phase-one memory sampler is now wired through the same fail-soft request-fingerprint builder as normal Sessions.

Target: bounded read-only assessment and post-integration re-verification in `/private/tmp/codex-cache-integration`; production-source digest `f8491e11952b1c12f12549a242a8129e2d52b47406fa3b66d7be839dd3b97aa2`.

## Exact formerly missing path

1. Phase one runs jobs concurrently and calls `sample` (`memories/write/src/phase1.rs:205-242`).
2. `sample` builds a genuine model prompt and calls `MemoryStartupContext::stream_stage_one_prompt` (`memories/write/src/phase1.rs:284-318`).
3. That method resolves the installation identity and builds detached metadata whose request kind is `memory`, but constructs a fresh `ModelClient` at `memories/write/src/runtime.rs:241-294`.
4. `ModelClient::new` initializes `cache_diagnostics: None` (`core/src/client.rs:470-491`), so each construction path must opt into the fail-soft builder.

The phase-two consolidation agent starts a normal `CodexThread` (`memories/write/src/runtime.rs:322-360`) and therefore already goes through Session initialization. The surface that required correction was specifically the direct phase-one Responses call.

## Deliverable decision

This belongs in the current diagnostic deliverable. The user's scope is request fingerprints for model requests, and this path sends model-visible instructions/input through the Responses API. Treating it as auxiliary would create a silent blind spot precisely when several memory jobs can run concurrently. The presence of `CodexResponsesRequestKind::Memory` and its cache-diagnostic mapping confirms that the data model intends to distinguish it.

## Integrated smallest seam

The root integrated the narrow existing `ModelClient` attachment point while keeping `CacheDiagnostics` private to core:

1. `ModelClient::with_cache_diagnostics(&Path)` is now a documented public builder that internally calls private, fail-soft `CacheDiagnostics::open` (`core/src/client.rs:494-499`). Its signature exposes only `std::path::Path`, not the collector or record implementation.
2. Normal Session construction passes `config.codex_home.as_path()` through the same seam (`core/src/session/session.rs:1483-1487`).
3. The detached memory client chains the builder before creating its `ModelClientSession` (`memories/write/src/runtime.rs:252-270`).
4. Detached metadata sets `CodexResponsesRequestKind::Memory` (`core/src/turn_metadata.rs:84-112`), and the diagnostic adapter exhaustively maps that variant to `RequestKind::Memory` (`core/src/cache_diagnostics.rs:224-230`).

This is preferable to exposing `CacheDiagnostics`, adding a `codex-cache-diagnostics` dependency to the memories crate, changing the large `ModelClient::new` signature, reusing the live Session model client with different auth/fallback semantics, or moving ownership into `ThreadManager`. It uses the effective `Config.codex_home` already in scope and preserves the accepted session-scoped collector model.

Each detached request currently creates its own `ModelClient`, so this seam opens one run file per phase-one request. Those runs still use the same installation key and remain fingerprint-comparable. Sharing one collector across concurrent memory jobs would require new ownership in `MemoryStartupContext` or `ThreadManager`; that is architecture expansion and is unnecessary for correctness here.

## Actual footprint and dependencies

- Production change is confined to `core/src/client.rs`, `core/src/session/session.rs`, and `memories/write/src/runtime.rs`.
- No new crate or third-party dependency is used; the public builder spells `std::path::Path` fully qualified.
- No request, auth, WebSocket, retry, prompt, or instruction construction changes are needed.
- No Cargo manifest or Bazel dependency update should be required.

## Focused regression evidence

The active test author is extending `memories_startup_phase1_uses_live_thread_service_tier_and_detached_metadata` at `memories/write/src/startup_tests.rs:590-689`. It already sends exactly one detached phase-one request with a temporary `codex_home` and asserts `request_kind: memory`.

After the response completes, read the diagnostic archive and assert one request/outcome pair with:

- `requestKind: "memory"` and `retryOrdinal: 0`;
- HTTP transport and one completed outcome;
- the outcome linked to the request attempt ID;
- no raw phase-one prompt, response ID, credential, or diagnostic field in the outbound model body.

The test can traverse the two-level archive directory using `std::fs::read_dir`, avoiding a new `walkdir` dev dependency. If the shared archive-reading helper is moved into an existing test-support crate instead, account for that larger dependency footprint explicitly.

## Verification limits

- I made no code edits and ran no Cargo command, build, test, or formatter. The root integrated the source correction, which I then re-read in place.
- Codebase-memory metadata matched the existing memory/core paths; the new cache adapter remained absent from the index and was read directly.
- `git diff --check` passed. About 16 GiB was free at the final check; no artifacts were created.
