# Duplicate terminal completion repair

Status: Production fix implemented; targeted verification passed. The full `codex-core` crate suite failed outside the changed area.

## Change

`codex-rs/core/src/unified_exec/process_manager.rs`: `write_stdin_inner` retains the `wake_on_exit` flag belonging to the same process entry as its locked process. On a successful terminal response (`process_id: None`), it clears that flag after the final awaited event send and while the interaction lock is still held. Errors and live responses leave the flag armed. The exit watcher remains unchanged.

## Evidence

- Baseline red: Coordinator replayed `just test -p codex-core -E 'test(terminal_write_stdin_does_not_)'`; all three tests failed before this production edit. Audit evidence is in `report.md`.
- Green: `CARGO_TARGET_DIR=/private/tmp/codex-patch-audit/async-target CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 UV_CACHE_DIR=/private/tmp/codex-patch-audit/async-uv-cache just test -p codex-core -E 'test(terminal_write_stdin_does_not_)'` — 3 passed, 4633 skipped.
- Green: `just test -p codex-core -E 'test(unified_exec) | test(background_exec_completion_starts_a_follow_up_turn_without_polling)'` — 173 passed, 4463 skipped. This needed execution outside the shell sandbox because mock servers bind loopback ports; the sandbox attempt failed at that environmental boundary.
- `just fmt` completed. Formatter changes in three unrelated Python evidence scripts were restored from HEAD; only the intended Rust production and pre-existing test edits remain.
- `git diff --check` passed.
- Full crate run: `just test -p codex-core` — 4,609 tests run: 4,335 passed, 274 failed, 27 skipped. The log is `/private/tmp/codex-patch-audit/async-core-tests.log`. This isolated Cargo target lacks first-party helper binaries such as `target/debug/codex` and `target/debug/test_stdio_server`, causing many integration failures. `app_tool_exposure::connector_omissions_preserve_direct_only_namespace` failed both in the full run and isolated rerun because its mock expected two requests and received zero. Several code-mode tests reached unrelated 30-second deadlines. The targeted unified-exec/completion run had passed all 173 tests in the same target and environment.

## Limits and handoff

The full crate suite is not green for the reasons above. No workspace suite or live provider test has run. The coordinator owns independent verification and integration. No commit was made.
