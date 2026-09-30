STATUS: DONE
COMMIT: none
CHANGED: 2
OPEN: 3
BLOCKERS: Compilation, green runs, and fixture discriminators are intentionally pending the coordinator's single integration build.
---

Scope

- Test-only edits in `/private/tmp/codex-cache-parent-completion`:
  - `codex-rs/core/tests/suite/subagent_notifications.rs`
  - `codex-rs/core/src/session_prefix_tests.rs`
- No production source, V1 behavior, dependency, configuration, or build artifact changed.
- Graph project `Users-viktor-Projects-github-codex`, indexed at `3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18`. `check_index_coverage` reported `no_recorded_issue` with matching metadata for all 12 source/test files used; this remains a best-effort signal.

Gap proof

- Executed suite searches:
  - `rg -n -i 'completion.{0,40}(follow.?up|wake|idle)|(?:follow.?up|wake|idle).{0,40}completion' codex-rs/core/tests codex-rs/core/src --glob '*test*.rs'`
  - `rg -n 'MultiAgentV2|collaboration' codex-rs/core/tests/suite/subagent_notifications.rs codex-rs/core/tests/suite/pending_input.rs codex-rs/core/src/agent/control_tests.rs`
  - `rg -n 'wait_agent|CollabWaiting' codex-rs/core/tests/suite codex-rs/core/src --glob '*test*.rs'`
  - `rg -n -i '(sub.?agent|child).{0,40}(completion|terminal)|(completion|terminal).{0,40}(sub.?agent|child)' codex-rs/core/tests codex-rs/core/src --glob '*test*.rs'`
  - `rg -n '1000|truncate|truncat|MAX_.*TOKEN|token.*cap|completion.*cap' codex-rs/core/tests codex-rs/core/src --glob '*test*.rs'`
- Idle-parent nearest tests: `pending_input.rs:503` wakes from synthetic queue-only mail, while the prior `subagent_notifications.rs:2311` test required a fresh user turn and `wait_agent`. Neither drove a real V2 terminal child into an already-idle parent or counted automatic requests.
- Active-wait nearest test: prior `subagent_notifications.rs:2311` drove a real child through `wait_agent` and checked delivered content, but did not prove that the completion produced only one parent delivery/continuation and no automatic extra request.
- Long-success nearest test: `session_prefix_tests.rs:10` bounds only `AgentStatus::Errored`; `AgentStatus::Completed(Some(...))` had no length or explicit-truncation coverage.

Changes and oracles

1. `multi_agent_v2_terminal_child_wakes_idle_parent_once` starts a real V2 child, delays its terminal response until the spawning parent turn is idle, waits for the automatic parent turn, deep-compares the sole `agent_message` completion envelope, and asserts exactly four `/responses` requests (parent spawn, child, parent continuation, one idle wake). A queue-only implementation times out before the second parent `TurnComplete`; duplicate scheduling raises the request count above four.
2. `plaintext_multi_agent_v2_completion_during_wait_is_delivered_once` strengthens the existing completed/error and legacy/paginated cases. It now asserts one matched completion delivery and exactly five `/responses` requests. An implementation that both satisfies `wait_agent` and leaves an automatic wake pending produces a sixth request.
3. `successful_completion_message_is_bounded_and_marks_truncation` uses a test-owned 1,000-token contract, a long successful payload, preserved head/tail sentinels, and the explicit `tokens truncated` marker. Leaving successful output unbounded fails the token assertion; silent clipping fails the marker; envelope or tail loss fails the sentinel assertions.

Value and vacuity self-check

- Regression: catches queue-without-wake, wake-after-wait duplication, and unbounded or silently clipped successful completion payloads.
- Audience/cost: protects operators and users from parents that stall until manual input, duplicated model turns/content, and completion fragments large enough to disrupt context/cache behavior.
- The idle and active-wait fixtures exercise both timing states through the real thread manager, child session, completion watcher, mailbox, and parent request path. Only remote model I/O is stubbed; the response delay preserves the concurrency boundary that creates the risk.
- Exact arrays, exact request counts, preserved sentinels, an independent numeric limit, and an explicit marker leave no broad success-only oracle. Self-check: KEEP pending the coordinator's execution and mutation lane.

Exact coordinator commands (from `codex-rs`)

```sh
just test -p codex-core -E 'test(successful_completion_message_is_bounded_and_marks_truncation)'
just test -p codex-core -E 'test(multi_agent_v2_terminal_child_wakes_idle_parent_once)'
just test -p codex-core -E 'test(~plaintext_multi_agent_v2_completion_during_wait_is_delivered_once)'
```

Combined single-build selection if preferred:

```sh
just test -p codex-core -E 'test(successful_completion_message_is_bounded_and_marks_truncation) | test(multi_agent_v2_terminal_child_wakes_idle_parent_once) | test(~plaintext_multi_agent_v2_completion_during_wait_is_delivered_once)'
```

Discriminator instructions (execution pending by role constraint)

- Idle wake: after the integrated green run, temporarily change the idle test's child response delay from `Duration::from_secs(1)` to `Duration::from_millis(0)`. The completion no longer occurs at the idle boundary; the unchanged idle-wake oracle must fail. Restore one second and rerun green.
- Active wait: temporarily change the existing child response delay from one second to zero. The completion arrives before the parent enters the blocking wait, so the fixture no longer reaches the named active-wait state and the unchanged request/delivery oracle must fail. Restore one second and rerun green.
- Successful bound: temporarily change `.repeat(2_000)` to `.repeat(2)`. The unchanged explicit-truncation assertion must fail because a short success is not truncated. Restore 2,000 and rerun green.
- Mutation targets for the later sweep: the terminal-completion wake choice, the wait-consumption clearing branch, and the successful-status truncation policy/limit.

Executed non-build checks

- `UV_CACHE_DIR=/private/tmp/codex-parent-completion-uv-cache just fmt` — exit 0. The initial plain `just fmt` reached Rust formatting but failed when `uv` could not write its sandboxed home cache; rerun with a writable private cache succeeded.
- `git diff --check` — exit 0 before the transient shared-runner descriptor exhaustion.
- Cargo/test execution: pending by explicit coordinator ownership.
