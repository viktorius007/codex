# Cache repair recovery checkpoint

The run is incomplete. Do not install an unvalidated binary.

RECOVERED: reading final results of already-completed exec sessions through write_stdin released retained resources and immediately restored exec. This was cleanup after completion notification, not passive polling. Reap each completed background session once until the local completion lifecycle is corrected. Initial regression log then showed all three intended behavioral tests failed.

The shared tool harness developed persistent EMFILE (Too many open files) after the baseline core suite and initial regression run. All agents' exec and most apply_patch operations failed before executing. Terminal UI access is prohibited by the computer-use tool; Activity Monitor inspection timed out. No running Rust command was killed, no diagnostics deleted, and no binary installed.

Root branch local/customizations is at fdca0be4f4 (three durable research/policy commits beyond ea55207054). Main was fast-forwarded to origin/main 36f0dbe796 without rebasing local/customizations away from stable 0.154.0.

## Preserved work

- /private/tmp/codex-cache-compact-parity, work/issue-37305: test edit in core/tests/suite/compact.rs plus production edits in core/src/compact.rs and core/src/session/turn.rs. Builder report /private/tmp/codex-cache-run/compact-parity-builder.md. Uncommitted, green gate pending.
- /private/tmp/codex-cache-usage-boundary, work/issue-35935: two test edits in core/src/context_manager/history_tests.rs and core/src/session/rollout_reconstruction_tests.rs. No production edits. Report usage-boundary-tests.md. Builder design remains in conversation handoff.
- /private/tmp/codex-cache-parent-completion, work/issue-37299: tests edited in core/tests/suite/subagent_notifications.rs and core/src/session_prefix_tests.rs; formatted. Report could not be written. Tests: successful_completion_message_is_bounded_and_marks_truncation; multi_agent_v2_terminal_child_wakes_idle_parent_once; plaintext_multi_agent_v2_completion_during_wait_is_delivered_once. No production fix yet.
- /private/tmp/codex-cache-integration, work/cache-integration: coordinator-only target (~11 GiB); contains test-only patches for compaction parity and usage boundary. Both patches are also in /private/tmp/codex-cache-run. Never overwrite these edits unknowingly.
- Diagnostic design/interface reports exist under /private/tmp/codex-cache-run; initial design is already committed under research/cache-repair-reports. Revised design uses immutable per-run files plus offline comparison (no runtime pointers/LRU/locks, no automatic evidence deletion). diagnostic-records-tests.md contains a proposed test/interface handoff, NOT actual tests. Record, transport and analysis worktrees exist but no code landed there.
- Skill locator worktree exists, no edits. Source scout report is committed.

## Executed validation

- Baseline exec-completion preservation regression: PASS, 1 test, 4184 skipped; build 2m58s. Log /private/tmp/codex-cache-run/core-build-baseline.log.
- Baseline core suite: FAIL, 4161 run, 3941 passed (1 leaky), 220 failed, 24 skipped. Log core-tests-baseline.log. Missing test_stdio_server, codex, and codex-code-mode-host explain many failures; remaining failures require rerun after prerequisites. No cache production fixes were present during this run.
- Initial regression-only run exited100. Log initial-regressions-red.log. Its contents have NOT been read because EMFILE began; do not claim the expected behavioral reds were observed.
- Baseline scanner Python3.12 unit tests passed; log scanner-tests.log.
- Bazelisk installed successfully through Homebrew, required for eventual dependency lock refresh.
- Last measured free disk before harness failure: 50 GiB. No final cleanup check could execute after EMFILE.

## Resume next

1. Restore shell/file-tool execution without discarding work; diagnose the running harness descriptor limit/leak. All agents have checkpointed/ended; no continued tool retry loops needed.
2. Inspect initial-regressions-red.log and materialize missing agent reports from conversation if needed.
3. Build prerequisite binaries from the integration worktree with its own target: cargo build -p codex-rmcp-client --bin test_stdio_server; cargo build -p codex-code-mode-host --bin codex-code-mode-host; cargo build -p codex-cli --bin codex. Run just test -p codex-core again. Do not compile from a different worktree into this target.
4. Remaining baseline failures with no binary-locator evidence: hook_runtime::tests::hook_lifecycle_notifications_hide_builtin_and_async_runs_but_preserve_metrics; session::tests::extension_metrics_preserve_session_metadata_tags; session::tests::world_state_extension_metrics_follow_turn_model_switch; suite::client_websockets::responses_websocket_emits_websocket_telemetry_events; suite::client_websockets::responses_websocket_includes_timing_metrics_header_when_runtime_metrics_enabled; suite::exec::openpty_works_under_real_exec_seatbelt_path.
5. Resume diagnostics test-writer/builder stages and root-cause patches. Use Sol for load-bearing work, fresh context, isolated worktrees, coordinator-only heavy builds. User asleep; full tests/install authorized; adapt priorities within stages1-5.
6. Integrate small green commits, independent verification, full authorized tests, version/lock bump, paired binary build/install with rollback and exact provenance. Cleanup all agent-created worktrees/targets only after verified integration; preserve diagnostics and unknown/user data.

## Cleanup ownership

Coordinator owns all /private/tmp/codex-cache-* worktrees and /private/tmp/codex-cache-run. Preserve reports, patches, and uncommitted work. Also remove agent-created /private/tmp/codex-parent-completion-uv-cache when safe. Completed scout/readiness worktrees were already removed; design/policy/integration/fix worktrees remain. No evidence may be automatically deleted.

## Restarted continuation (current)

Inherited shell soft limit4096, hard unlimited; commands work. Last disk39GiB. Prerequisite build succeeded using checksum-verified Codex V8 archive+binding via existing scripts/codex_package/v8.py (default upstream download404). Env /private/tmp/codex-cache-run/v8-env.json; runner run-check.py applies this to all coordinator-only target commands. Matching codex, codex-code-mode-host, test_stdio_server built but NOT installed.

Integration has compaction fix and its regression: targeted PASS after baseline red. New parent tests baseline: success bound FAIL, idle wake FAIL (timeout), active wait3 cases PASS. Root final-envelope cap patch applied to integration; full core check running with only3 intentionally red usage/idle tests excluded. Log core-prerequisites-complete.log, exec8872. After each real background completion, consume final result once as resource cleanup, no polling.

Fresh active roles: records_builder_recovered owns new record crate+manifests; transport_tests_recovered owns API observer tests; usage_builder_recovered owns usage boundary production; wake_builder_recovered owns wake production (root cap+test edits frozen); locator_tests_recovered owns skill-locator tests. All in their previously assigned isolated worktrees, no agent Rust builds. Root fixed4 small pending record test issues before builder dispatch. Partial old reports remain historical; current role reports use matching recovered names in /private/tmp/codex-cache-run.
