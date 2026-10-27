# Forked-child startup failures on sync/0.165.0

Fix commit: `fec9a0d4d2` "Stop async-result wakes after shutdown; match full-fork child by its task" (branch `sync/0.165.0`).

## Finding: the premise was wrong

The forked child does start. Temporary instrumentation (now removed) showed `maybe_start_turn_for_pending_work_with_sub_id` reaching `started=true` for the child, and the mock server received the child's requests. The two tests fail for two separate reasons. Neither reason is the interrupted status or residency in child startup.

## 1. prompt_cache_key::api_key_subagent_inherits_prompt_cache_key_policy (both cases)

Causal chain:
1. `core/src/agent/control/spawn.rs:1173`: `preserve_fork_prefix` is on by default (same as upstream, `config/mod.rs:1413`). `spawn.rs:1277` therefore keeps every parent ResponseItem, including the parent's in-flight `spawn_agent` call (`call_id: "spawn-worker"`). Normalization pairs that call with an `"aborted"` output.
2. The test's child mock matches `body_contains(CHILD_TASK) && !body_contains(SPAWN_CALL_ID)` (`core/tests/suite/prompt_cache_key.rs:141`). The child's body always contains `spawn-worker`, so this mock never matches the child. The root's follow-up mock is single-use and already consumed, so the child gets 404s and retries until it errors.
3. Upstream still passes because upstream `base_mock()` (`git show rust-v0.165.0:codex-rs/core/tests/common/responses.rs:1133`) adds the recording `ResponseMock` matcher before the custom matcher. Every request the mock evaluates is recorded, matched or not. On upstream the child is "seen" but never served.
4. Local commit `1f0c136254` ("Correlate fixture responses and completion with admitted turns") changed `mount_sse_once_match` (`core/tests/common/responses.rs:1149`) to record only requests that match. That exposes the matcher defect.

Fix (fixture only; the assertions are unchanged): the child mock now matches `CHILD_TASK` together with the child's `Message Type: NEW_TASK` delivery. The child is now actually served.

## 2. guardian_retained_context::standalone_fork_retains_inherited_user_instructions::copied_worker_history

Causal chain:
1. Local design (b): a child result wakes the parent. `agent/control/completion.rs:115` sends with `trigger_turn = true`, `completion.rs:118` calls `deliver_async_result`, `agent/control.rs:325` calls `Session::enqueue_async_result`, and `session/inject.rs:16-26` calls `maybe_start_turn_for_pending_work_with_sub_id` (`tasks/mod.rs:555`). This calls the session directly and does not go through the submission loop.
2. Upstream only woke an idle session through the submission loop, which exits on shutdown. `shutdown_session_runtime` sets `state.shutting_down = true` (`session/handlers.rs:318`), but nothing on the local direct-wake path checked it. The only check, in `reserve_pending_work_turn`, was `is_interrupted()`, and an idle root is never marked interrupted.
3. In the test, the worker's first completion woke the root. That wake ran after `test.codex.shutdown_and_wait()` (`guardian_retained_context.rs:863`) and consumed the worker's `local-instruction` response (`:871`). The worker then got 404s, errored, and its error result woke the shut-down root again. That wake consumed the fork's `ask-root-action` response, so `record_answer` (`:987` -> `:161`) waited forever for `RequestUserInput`. This matches the recorded request order. The Paginated case happens to avoid the collision because its later mocks use specific matchers.

Fix: `reserve_pending_work_turn` (`core/src/session/turn_input.rs:706`) also returns `None` while `shutting_down` is set. The check runs under the `active_turn` admission lock, which keeps the existing active_turn -> state lock order. The result stays queued rather than being dropped. A reservation made just before the flag is set is cancelled by shutdown's `abort_all_tasks`, and its rollback re-check sees the flag. Design constraints (a), (b) and (c) and the upstream interrupt and follow-up behaviour are unchanged.

New causal regression test: `session::turn_input::tests::async_result_does_not_start_a_turn_after_shutdown_begins` (`core/src/session/turn_input_tests.rs`). It sets `shutting_down`, delivers an async result, and asserts that no turn is reserved and the result is still queued. With the guard removed, both this test and the guardian Legacy case fail (`turn_input_tests.rs:1155` and `common/lib.rs:395`). With the guard, both pass.

## Verification

Command (from `codex-rs`, with the required env vars):
`just test -p codex-core -E 'test(/prompt_cache_key::api_key_subagent_inherits|guardian_retained_context::standalone_fork_retains_inherited_user_instructions|agent_message_wake::|tasks::admission_tests|unified_exec::background_exec_completion_starts_a_follow_up_turn_without_polling|agent_execution::|subagent_notifications::|agent_control_tests::/)'`

Summary line after the fix:
`     Summary [   4.550s] 79 tests run: 79 passed, 5378 skipped`

No tests still fail. In addition, `prompt_cache_key::` plus `guardian_retained_context::` was run three times with `--retries 0`: 30/30 passed each time.

Note: `just fmt` also reformatted four unrelated files that were already committed (`codex-mcp/src/mcp/mod.rs`, `core/src/tasks/mod.rs`, `core/src/tools/handlers/multi_agents_tests.rs`, `core/tests/suite/scenarios.rs`). I reverted those changes and did not commit them. The Bazel/Starlark formatting step failed in this environment. `just fix -p codex-core` was not run.
