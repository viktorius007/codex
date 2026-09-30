PASS
Verified base: `d5f8ef066a86255fc1e84c1b2a9759c222aa9f4d` plus the uncommitted two-file diff in this worktree; no commit or commit gate exists yet.
Command: `just test -p codex-core -E 'test(terminal_write_stdin_does_not_) | test(background_exec_completion_starts_a_follow_up_turn_without_polling)'` with the isolated Cargo target — 4 passed, 4632 skipped.
Command: `git diff --check` — exit 0.
Baseline discriminator: `/private/tmp/codex-patch-audit/async-red-replay.log` — 3 tests failed twice before the production edit; the cross-component failure contained one `exec.completion` response item rather than the expected empty queue.

| Criterion | Proof |
| --- | --- |
| Terminal `write_stdin` consumes exit without a second model-visible completion | `mod_tests.rs:787-920` drives the real process, manager, watcher, and session queue; the red replay captured the duplicate and the independent green run passed. `process_manager.rs:1127-1129` disarms under the interaction lock, which the watcher takes at `async_watcher.rs:201-215`. |
| Exited entry and already-removed entry both disarm | `mod_tests.rs:688-784` varies `remove_before_exit`; both baseline cases failed and both current cases passed. The flag is cloned before either removal at `process_manager.rs:886-898`. |
| Unattended exit still wakes the agent | Existing `background_exec_completion_starts_a_follow_up_turn_without_polling` passed; `process_manager.rs:717-724` and `1134-1158` arm live sessions, and `async_watcher.rs:211-269` injects on exit. |
| Live, error, and cancelled writes preserve wakeup | `process_manager.rs:1127-1129` clears only on successful terminal `process_id: None`. Live responses retain `Some` at `1070-1075`; error returns occur before the clear at `906-1059` and `1076-1092`; dropped/cancelled futures also leave it armed. The shared lock serializes the successful clear with the watcher flag read. |

Findings: none in the scoped diff. `process_manager.rs` adds only a clone and one conditional store; no new function, type, API, comment, or indirection. The test fixture owns its process and queue; its empty-queue oracle cannot read its expected value from production. The watcher event and mutex are causal barriers; timeouts are hung guards. The new test comment explains why acquiring the mutex proves watcher injection has finished.

Limits: This is a review of an uncommitted patch, not a merge verdict. The 173 related tests reported green in `fix-report.md`, but I independently reran only the four tests above. The full `codex-core` run reported 274 failures, including missing helper binaries, and the coordinator is resolving that integration environment. The fixture intentionally orders the terminal writer ahead of the watcher; it does not establish behavior when the watcher acquires the lock first. That race predates this edit and has no executed failing probe in this audit. No provider test, mutation sweep, or commit gate ran. Codebase-memory coverage recorded no issue for the changed files, but Rust call relationships are semantically partial; the paths above were checked in source.
