STATUS: DONE
COMMIT: 34d1bd0a11
CHANGED: test-only `codex-rs/thread-store/src/local/archive_thread.rs`; `report.md`, `findings.json`, `red-test.patch`, `red-test-output.txt`, `AGENTS.md` index
OPEN: 1 finding
BLOCKERS: none
PROVEN_DEFECTS: 1
REVIEWED: 6 write/persistence entry groups; 28 graph coverage-checked paths, with targeted source reads
RED_TESTS: 1 executed / 1 failed twice on baseline

# Durable transcript, compaction, migration, and archive audit

## Slice map and coverage

Baseline is `34d1bd0a11`. The codebase-memory graph was used for entry-point discovery and call traces, then exact worktree source was read. `check_index_coverage` reported no recorded file gaps for all 28 relied-on paths; Rust call/implementation semantics remain partial, so absence claims are based on direct source, not graph edges alone. This is a bounded risk-seam audit, not a full-file review of the large recorder, migration, or core session modules.

| Entry group | Actor and terminal outcomes | Source disposition |
| --- | --- | --- |
| Rollout recorder | New/resumed sessions queue items; persist, flush, shutdown, and discard write or retain pending items after I/O error. | `rollout/src/recorder.rs:857-1067,1768-2067`; inspected retry/error path and existing tests. No executed defect candidate. |
| Compression/materialization | Background cold-file worker and resumed writer exchange plain/compressed representations. | `rollout/src/compression.rs:101-189,809-1098`; verified temp sync/verification and source removal order in source. No executed defect candidate. |
| Legacy migration | Startup discovers rollouts; migration journals, stages, projects, publishes, and recovers. | `thread-store/src/local/rollout_migration.rs:375-834,998-1083`, `rollout_migration/{publish,startup}.rs`; journal exists before replacement and recovery runs for a pending paginated migration. No executed defect candidate. |
| Archive/unarchive | App-server and CLI trigger file moves plus SQLite metadata updates. Crash after rename is the candidate. | `thread-store/src/local/{archive_thread,unarchive_thread,helpers,thread_rollout_resolver,model_context}.rs`; complete archive/unarchive/resolver functions read. |
| Compaction checkpoint | Core replaces in-memory history and appends compacted/baseline items through local thread persistence. | `core/src/{compact,compact_remote_v2,session/mod}.rs:386-423,320-379,4000-4077,4413-4419`, `thread-store/src/{live_thread,local/live_writer}.rs`; no executed defect candidate. |
| Production request lifetime | App-server `thread/archive` awaits the local store; active requests survive ordinary client connection close. | `app-server/src/{message_processor,request_serialization,connection_rpc_gate}.rs`, `request_processors/thread_processor.rs:623-644,1687-1768`; only process crash/forced termination is asserted for the candidate. |

## Proven finding: interrupted archive breaks cold resume

`archive_thread_with_paths` renames the JSONL file before awaiting `mark_archived` (`archive_thread.rs:121-145`). The same order exists for unarchive (`unarchive_thread.rs:95-130`). If the process exits after rename and before the SQLite update, the durable SQLite row still points to the old path. `thread_rollout_resolver::resolve` returns `None` when a paginated row's path is missing (`thread_rollout_resolver.rs:96-118`), before trying the archived filesystem fallback. `load_latest_model_context` uses that resolver (`model_context.rs:37-51`), so a cold resume may fail despite intact JSONL bytes. The test fixture persists the exact rename-before-database state and reopens the database and store; it does not claim an ordinary client disconnect cancels the request.

The focused command was `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store archive_crash_after_rename_keeps_paginated_context_resumable`. It compiled and exited 100. Nextest ran one test and failed it twice in 0.578 seconds total: `InvalidRequest { message: "no rollout found for thread id 00000000-0000-0000-0000-0000000000d0" }` at the cold `load_latest_model_context` call. The earlier pre-rename call in the same test succeeded. After reopening SQLite, the test asserted the database still pointed to the missing active path and the archived JSONL existed. Raw output is in `red-test-output.txt`; the complete test-only patch is in `red-test.patch` and embedded in `findings.json`. The isolated target measured 4.6 GiB and was retained for coordinator replay.

The production sequence is directly reachable when the app-server process is killed during its awaited archive operation (`app-server/src/request_processors/thread_processor.rs:1756-1763`). Ordinary client connection close is not the claimed trigger: `connection_rpc_gate.rs:run` allows started handlers to finish and `shutdown` waits. The same risk is visible in the symmetric unarchive source, but only archive has an executed regression here. A minimal repair should recognize an exact moved counterpart of the SQLite-selected rollout path after a crash, verify that it belongs to the same thread and rollout, then reconcile the metadata or resolve through it. A broad scan could accidentally select an older immutable rollout after a revert.

## Dismissed candidates and limits

- Compression publishes a synced and decoded-verified temp before removing its plain source (`compression.rs:809-930`). A crash between publish and source removal may leave both representations, but the original remains. Reopen if a test shows wrong precedence or divergent bytes.
- Migration writes and syncs a `.pending` journal before staging (`rollout_migration.rs:622-647`, `publish.rs:225-248`) and startup scans pending IDs (`startup.rs:61-80`). A published paginated rollout with a journal invokes recovery (`rollout_migration.rs:483-518,998-1047`). Reopen with a concrete uncovered crash point or failed recovery fixture.
- A compaction checkpoint mutates in-memory history before appending to disk, and `persist_rollout_items` logs append errors without returning one (`core/src/session/mod.rs:4043-4073,4413-4419`). This is a source-level risk under persistent disk error; no deterministic disposable failure fixture was executed in this audit, so it is not a proven finding.
- Recorder `write_line` flushes each JSONL record and retry reopens the file (`rollout/src/recorder.rs:1796-1880,2049-2075`). A partial write or ambiguous flush may merit a fault-injection follow-up; this audit did not prove item loss or duplicate replay.

The archive crash fixture does not exercise a real power loss or the user's private sessions. It tests a durable on-disk state directly reachable from the shown source order. No production file was changed.
