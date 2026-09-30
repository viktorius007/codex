STATUS: DONE
COMMIT: 34d1bd0a110879b6c8d0b7d517a22ceb367aba9b
CHANGED: AGENTS.md; codex-rs/core/src/agent/control/residency_tests.rs (test only); report.md; findings.json; residency-regressions.patch; four raw test logs
OPEN: 1 proven defect
BLOCKERS: none
PROVEN_DEFECTS: 1
REVIEWED: 12 entry point families / 18 files at relevant source paths
RED_TESTS: 2 candidate tests executed and failed; 1 admitted after oracle review

# MultiAgentV2 residency and capacity audit

## Proven finding: a canceled eviction loses its capacity owner

With `max_concurrent_threads_per_session = 2`, the root has one loaded-worker slot (`config/mod.rs:1578-1590`). A completed child occupies it. When another spawn or reload requests a slot, `V2Residency::try_unload_one_resident` removes the child ID from `residents` before awaiting its manager lookup, idle check, rollout materialization, and shutdown (`agent/control/residency.rs:117-155, 168-180`). `try_reserve_pending_slot` counts only `residents.len() + pending_slots` (`residency.rs:105-115`). Another reservation can therefore claim the sole slot while the original worker is still loaded. If the first future is canceled, no drop guard restores the popped ID; later reservations continue to omit the loaded worker.

This is reachable from V2 spawn and reload (`agent/control/spawn.rs:590-614, 632-690`). A parent turn can cancel an in-flight tool task: `abort_all_tasks` takes its running task, and `handle_task_abort` eventually calls `task.handle.abort()` (`tasks/mod.rs:644-665, 1030-1062`). The user consequence is a resident worker count above the configured limit; cancellation can make the undercount persist until that worker is otherwise closed or touched. The test proves the local capacity invariant and cancellation mechanics; it does not measure live memory use or provider cost.

The exact red test is `residency_keeps_loaded_owner_counted_during_and_after_cancelled_eviction` in [the patch](residency-regressions.patch), embedded in [findings.json](findings.json). It creates one completed loaded child, verifies the manager lookup and terminal status, holds the active-turn lock, then directly polls the eviction future to `Pending`. This synchronization depends on the held lock, not on the resident count becoming wrong. While the original runtime remains loaded, another pending-slot reservation succeeds. After dropping the eviction future, the resident count is zero. The assertion requires `(false, 1)` and observed `(true, 0)` on unchanged production. [Final raw log](residency-red-final.log): one focused test compiled and failed twice at `residency_tests.rs:120`; 4,636 other tests were skipped. The earlier [race log](residency-red-cancellation.log) also failed, but its synchronization waited for the incorrect count, so the final test supersedes it. The original [broad run](residency-red.log) included the existing serial eviction test passing and four unrelated app-server residency tests failing because this sandbox forbids mock-server port binding.

The smallest repair direction is to keep an evicting worker represented in capacity accounting until its runtime is actually removed, with rollback on cancellation and shutdown failure. The coordinator owns the production choice and verification. A later fix should check both overlap and dropped-future paths without relying on elapsed time.

## Queue-only mail: observed limitation, not admitted

`send_message` selects `QueueOnly` (`tools/handlers/multi_agents_v2/send_message.rs:42-58`). The session handler accepts that mail but does not start a turn unless a durable sleep is present (`session/handlers.rs:72-116`); the idle scheduler likewise needs trigger-turn mail or durable sleep (`tasks/mod.rs:472-504`). `is_unloadable` intentionally rejects a worker with pending mailbox items (`agent/control/residency.rs:233-239`). With one worker slot, a completed worker holding queue-only mail blocks a new spawn until a follow-up or close. The [focused candidate log](residency-red-focused.log) confirms `AgentLimitReached { max_threads: 1 }` after the accepted mail was queued.

That queue is in memory (`session/input_queue.rs:84-103, 212-232`), and this audit found no durable handoff for it when evicting a worker. Requiring immediate slot availability could silently lose accepted mail. No independent contract established that the capacity tradeoff is wrong. The candidate test was removed from the admitted patch; it is not a finding or repair instruction. Revisit only with an explicit mailbox-preservation contract and a demonstrated delivery mechanism.

## Other source dispositions and limits

- Precommit residency slots release on drop (`residency.rs:28-46, 210-216`), and registry spawn reservations release their count and reserved path on drop (`agent/registry.rs:386-430`). This does not cover the popped eviction candidate, which has no guard.
- Explicit close calls `shutdown_live_agent`, which removes the runtime, residency entry, and registry identity after termination (`agent/control/legacy.rs:8-43`). `handle_thread_request_result` performs the same accounting cleanup for a request returning `InternalAgentDied` (`agent/control.rs:458-472`). These source paths do not prove every worker death is observed.
- An interrupted worker with an empty mailbox is eligible for eviction (`residency.rs:233-239`); the existing `interrupted_v2_agent_is_lost_after_residency_eviction` test passed in the broad run. Its loss-on-eviction behavior was not treated as a new defect.
- `AgentExecutionLimiter` counts running turns separately from loaded residency (`agent/control/execution.rs:17-109`, `tasks/mod.rs:352-436`). No separate running-turn over-capacity result was established in this slice.
- A persistence shutdown error is logged and the session still terminates (`session/handlers.rs:350-386`); this cross-slice storage lead was sent to the coordinator, not claimed as a proven residency defect.

The source inventory covered `agent/control/{residency,spawn,delivery,legacy,interrupt,inspection,execution}.rs`, `agent/{control,registry,status}.rs`, `tools/handlers/multi_agents_v2/{message_tool,send_message,followup_task}.rs`, `session/{handlers,input_queue,mod}.rs`, `tasks/mod.rs`, `thread_manager.rs`, and `codex_thread.rs`. Large session/task files were read at relevant entry points and terminal paths, not exhaustively cleared. The codebase-memory index at 2026-09-23T12:39:48Z reported no recorded file gaps for these paths, but Rust semantic call edges are partial; worktree source resolved the material claims. No private session records or live provider tests were needed.

`just fmt` passed after the final test edit. Its incidental Python formatting changes were restored byte-for-byte from HEAD. `git diff --check` and `python3 -m json.tool findings.json` passed. All builds used an isolated target with `RUSTC_WRAPPER=`, `HOST_CC=clang`, `HOST_CXX=clang++`, 32 jobs, and incremental compilation. The target is retained for the coordinator's repair handoff: 12 GiB at the final check, with 17 GiB free on the shared disk. The formatter cache was removed. Production source was not edited and no failing test was committed.
