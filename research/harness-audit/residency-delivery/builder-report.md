STATUS: DONE
COMMIT: b7f03f154f2739aa19c636804a32e15814c13a1f
CHANGED: codex-rs/core/src/agent/control/residency.rs; codex-rs/core/src/agent/control/residency_tests.rs
TESTS: 4 focused codex-core library tests passed
LIMITS: Full codex-core suite and lint were not run under the coordinator's allocated test scope; independent verification remains.

# Residency capacity repair

Eviction now keeps the selected worker in the resident deque through every awaited eligibility, materialization, and shutdown step. A canceled eviction therefore leaves the still-loaded worker counted. The resident entry is removed only after the manager confirms the thread is absent or its runtime has been removed. Selection rotates the deque to retain least-recently-used order. An async eviction gate prevents concurrent evictors from shutting down the same worker; capacity is checked again after acquiring that gate, so a freed slot is reserved without unloading another worker. Normal free-slot reservations remain concurrent, and the state mutex is never held across an await.

The auditor's unchanged `residency_keeps_loaded_owner_counted_during_and_after_cancelled_eviction` regression was red twice on the baseline, observing `(true, 0)` instead of `(false, 1)` after a known loaded worker's eviction future was canceled. With this commit it passed alongside the existing serial eviction and interrupted-worker tests. The exact command was:

```sh
RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/private/tmp/codex-harness-audit/residency/codex-rs/target just test -p codex-core --lib -E 'test(residency) | test(interrupted_v2_agent_is_lost_after_residency_eviction)'
```

The result was `4 tests run: 4 passed, 2594 skipped`, recorded in `residency-green.log`. `just fmt` passed with a worktree-local `UV_CACHE_DIR`, and `git diff --cached --check` passed before the commit. The first compile caught a borrow error in the gate's free-slot path; that was corrected before the passing run. No queue-only mail or persistence behavior was changed.

At handoff, the isolated Cargo target is retained for the independent verifier. It used 8.8 GiB at the last check, with 26 GiB free on the shared disk. The temporary `uv` cache was 16 KiB and can be removed after this report. The broad core suite, Clippy, and cross-platform checks were not run, and the test does not measure actual worker memory use or provider cost.
