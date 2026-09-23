STATUS: Complete; one proven duplicate-completion defect, production unchanged.
COMMIT: d5f8ef066a (local/customizations, audit baseline).
CHANGED: Test-only edits in `codex-rs/core/src/unified_exec/mod_tests.rs`; no production edits or commits.
OPEN: Coordinator to choose and implement the fix, then rerun both red regressions. No other assigned-slice defect met the proof threshold.
BLOCKERS: None. One existing integration test needed loopback permission; it passed outside the sandbox.
FILES_REVIEWED: `core/src/agent/control.rs`, `core/src/context/exec_completion.rs`, `core/src/session/{input_queue,inject,mod,turn_input}.rs`, `core/src/state/turn.rs`, `core/src/tasks/{mod,lifecycle}.rs`, `core/src/unified_exec/{async_watcher,mod_tests,process,process_manager}.rs`, `core/src/tools/code_mode/{delegate,wait_handler}.rs`, `core/tests/suite/{unified_exec,subagent_notifications}.rs`, `ext/goal/src/{extension,runtime}.rs`, `extension-api/src/async_result_admission.rs`.
PROVEN_DEFECTS: 1 (duplicate model-visible ExecCompletion after terminal write_stdin receipt).

---

## Finding 1 — terminal `write_stdin` and watcher both report the same exit (P2)

**Expected:** Once `write_stdin` returns `process_id: None` with the exit result, the background watcher may emit its terminal UI event, but it must not enqueue another `ExecCompletion` for the agent. This follows the local customization’s exactly-once completion requirement and avoids an extra model turn or duplicate context.

**Production path:** An initial `exec_command` returns a live session and arms the shared `wake_on_exit` flag. `write_stdin_inner` takes the process interaction mutex, waits until the process exits, and returns terminal output. In both the stored `ProcessStatus::Exited` and removed-entry `ProcessStatus::Unknown` paths, it does not clear `wake_on_exit` (`process_manager.rs:898, 1070-1127`). The watcher later takes that same mutex, reads the still-true flag, constructs an `ExecCompletion`, emits `ExecCommandEnd`, then calls `Session::inject_or_start` (`async_watcher.rs:190-269`). Injection enqueues a `TurnInput::ResponseItem` in the session result queue (`session/inject.rs:29-42`). When idle and not interrupted, pending-work admission can start a new regular turn (`tasks/mod.rs:471-540`). An active turn can consume the duplicate in its next model request. The model-visible fragment is capped by `context/exec_completion.rs`.

**Red proof:** `terminal_write_stdin_does_not_leave_exec_completion_armed::{normal_exit,removed_before_exit}` in `core/src/unified_exec/mod_tests.rs:691` executed with `just test -p codex-core -E 'test(terminal_write_stdin_does_not_leave_exec_completion_armed)'`. Both cases failed twice at line 777: the flag remained true after terminal output. This covers both production status branches.

**Cross-component red proof:** `terminal_write_stdin_does_not_queue_duplicate_exec_completion` in the same file at line 787 drives a real `UnifiedExecProcess` from a controlled process driver, the real manager `write_stdin`, the real watcher, and the real Session input queue. It waits for the writer to enter its poll, signals process exit, verifies terminal `process_id: None`, receives the watcher’s `ExecCommandEnd`, and then acquires the same interaction mutex held through watcher injection. That mutex is the causal barrier; the 2-second timeouts only guard against a hung test. It marks the session interrupted to keep the result queued for exact inspection. Command: `just test -p codex-core -E 'test(terminal_write_stdin_does_not_queue_duplicate_exec_completion)'`. It failed twice with the complete actual `TurnInput` vector containing one user `ResponseItem`, `ContentItemKind("exec.completion")`, and `<exec-command-completed call-id="exec-call" ...>`; expected `[]`. This observes the exact item the next model input would drain. The test sits in the crate test module because the synchronization mutex and queue are private to Core; an external HTTP mock cannot make this ordering deterministic.

**Repair boundary:** Clear the shared completion-armed state while terminal `write_stdin` still owns the process interaction mutex, covering both exited and already-removed process entries. Preserve watcher delivery when the process exits without any terminal `write_stdin` receipt. Coordinator owns the production solution.

## Checked behavior and dismissal

- Existing `background_exec_completion_starts_a_follow_up_turn_without_polling` passed, confirming a live process still wakes the agent when no terminal poll consumed it.
- Existing `empty_timed_yields_stay_inside_one_wait_until_useful_output` passed, confirming the code-mode wait keeps empty timed yields inside one tool call.
- Existing `multi_agent_v2_terminal_child_wakes_idle_parent_once` passed outside the sandbox. Its first attempt failed before agent logic because wiremock could not bind a loopback port (`PermissionDenied`).
- I dismissed a candidate that a Goal denial should remove async results from an active turn. The `AsyncResultAdmissionContributor` contract governs whether an idle result may **start** a turn. During an authorized active turn, queue delivery is allowed; the candidate red test asserted a stronger, unsupported rule and was removed.
- Source tracing found session-level async-result queueing and restoration on cancelled starts in the current code, so the older retiring-turn loss scenario in historical research is not presented as a current defect.

## Verification limits and housekeeping

The duplicate test proves queue delivery of the exact model-visible response item, not a measured provider `/responses` count. No live provider test ran. Codebase-memory coverage checks found `metadata_match/no_recorded_issue` for primary files, but Rust call binding and implementation relationships were semantically partial; production paths above were verified directly in source. `just fmt` and `git diff --check` passed. The formatter also changed three unrelated evidence Python files; those formatter-only hunks were restored exactly from HEAD, leaving only the intended test file modified. The isolated Cargo target is 6.6 GiB; free disk was 71 GiB after the build. Target retained for coordinator verification and cleanup.
