PASS
COMMIT: b7f03f154f2739aa19c636804a32e15814c13a1f
COMMAND: `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/private/tmp/codex-harness-audit/residency/codex-rs/target just test -p codex-core --lib -E 'test(agent::control::residency::tests)'` — 3 tests run: 3 passed, 2595 skipped.
CHECK: `git show --check --format= b7f03f154f2739aa19c636804a32e15814c13a1f` — exited 0.

| Criterion | Evidence |
| --- | --- |
| Loaded worker stays counted during eviction and after cancellation | `residency_tests.rs:70-128` holds the active-turn lock, polls eviction to pending, confirms the runtime remains in the manager, and asserts `(false, 1)` for a concurrent reservation and resident count. The baseline `residency-red-final.log` failed with `(true, 0)` twice; this run passed. `residency.rs:134-164` retains the deque entry until manager removal. |
| No duplicate concurrent eviction | `residency.rs:95-111` serializes the only call to `try_unload_one_resident` through `eviction_gate` and rechecks capacity after acquiring it. The gate is released on return or cancellation. Source verified; no separate concurrent-evictor test was run. |
| Release accounting only after runtime removal | `residency.rs:137-164` removes stale manager entries or removes the resident after `shutdown_and_wait` and `remove_thread` complete. No await occurs between manager removal and accounting removal. |
| Protected worker and pending mailbox retained; free slots remain concurrent | `residency.rs:88-94,178-192,243-249` preserves fast-path slot reservation, skips the protected ID, and rejects a worker with pending mailbox items. Existing normal and interrupted eviction tests passed; no separate protected/mailbox fixture was run. |

FINDINGS: None in the verified diff.
SHAPE: The production diff adds one mutex field and changes existing methods; no new helper, public API, suppression, or duplicated policy. The new test uses an existing test module and a test-owned temporary home.
LIMITS: This was the allocated focused verification. The full core suite, lint, mutation sweep, and live worker memory measurement were not run. The codebase-memory index has no recorded file gaps for the reviewed paths, but Rust call edges are partial; the worktree source supplied the concurrency trace.
