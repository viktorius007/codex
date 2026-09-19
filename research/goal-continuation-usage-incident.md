# Goal continuation usage incident: 19 September 2026

## Purpose

This is a privacy-safe handoff for a future session investigating why active Codex Goals can consume large amounts of model usage through automatic continuation turns that do little or no work.

It combines:

- local rollout and Goal-database evidence from 19 September 2026 in Australia/Brisbane;
- the relevant public `openai/codex` issues and merged fixes as of 20 September 2026;
- the smallest current source path that schedules, accounts for, and stops Goal continuations; and
- discriminating tests and diagnostics for a source-level root-cause investigation.

No private prompt, repository content, Goal objective, tool output, credential, or full session transcript is reproduced here.

## Conclusion

The incident is confirmed.

Two local Goal threads created on 19 September Brisbane time generated **826 automatic Goal turns** and **1,876 token-bearing model-response records**. Those automatic turns processed **235,058,028 input tokens**, of which **231,428,352 (98.5%) were cached**, plus 411,579 output tokens. Both threads independently ended an automatic turn with `usage_limit_exceeded` at approximately **16:57 Brisbane time**. Failed provider attempts without a usage record are not included in the 1,876 count.

One thread generated **513 successful automatic turns with an empty final answer and no error** during the Brisbane day. Its longest uninterrupted run was **157 empty turns in 12 minutes 40 seconds**. These are not inferred from the blank UI: each turn has a Goal continuation input, a task start, a zero-length assistant final item, token usage, and a successful task completion in the rollout. A separate 514th turn had a zero-length `last_agent_message` but ended with `usage_limit_exceeded`; it contained commentary and a tool call, had no assistant final item, and is not counted as a successful empty-final turn.

Using a more conservative cross-session definition—exactly one model response, no tool call, and no more than 128 output tokens—the two threads contained **578 near-empty turns**. They processed **84,915,606 input tokens** (84,069,888 cached) and 10,213 output tokens. This threshold is an analysis aid, not a Codex product state; the literal empty-final count above comes directly from persisted events.

The sessions ran **Codex CLI `0.154.0+local.3`**. That version predates [PR #44320](https://github.com/openai/codex/pull/44320), the three-empty-turn circuit breaker shipped in 0.155.0. The current checkout at `48780d7c6e3ccbc32d8cce8de03a746be8a1cb4d` is `0.155.1+local.4` and contains that fix. Therefore:

1. the historical empty-final chain has a known missing safeguard in the incident binary;
2. the underlying immediate continuation mechanism is still relevant in current code; and
3. repeated polling, tool activity, or non-empty no-progress replies remain outside the merged empty-turn breaker by design.

The account meter is shared across concurrent threads, so no honest analysis can assign every percentage point to one rollout. The automatic Goal traffic, identical limit failures, and meter transitions establish that the Goal loops materially consumed the available allowance; they do not establish the server's private billing formula.

## Time boundary and evidence set

“Yesterday Brisbane time” means the following half-open interval (start included, end excluded):

- local interval: `[2026-09-19 00:00:00, 2026-09-20 00:00:00)` AEST (`UTC+10`);
- UTC interval: `[2026-09-18T14:00:00Z, 2026-09-19T14:00:00Z)`.

The active Goal database was read from `~/.codex/goals_1.sqlite`. It contains two Goals created in that local-day interval:

| Thread | Goal created (Brisbane) | Status now | Goal-accounted tokens | Goal-accounted active time |
|---|---:|---|---:|---:|
| `01a0b77c-8c00-76c1-be80-26d140fc209c` | 13:04:59 | complete | 9,195,332 | 19,160 s |
| `01a0b786-a3f3-7120-b40d-33196c23e7ea` | 15:01:26 | complete | 5,174,066 | 11,853 s |

The Goal counter is a product accounting field. The larger rollout totals below are the sum of per-response `token_usage_record.usage` values and measure model traffic, including cached input. Keep these measures separate.

The session census read every rollout under `~/.codex/sessions` and `~/.codex/archived_sessions`, selecting records by their persisted UTC timestamps rather than filenames. Exactly two of 147 sessions contained an in-window internal Goal continuation marker. No private prompt or tool payload was needed to identify them.

## Local incident evidence

### Thread `01a0b77c-8c00-76c1-be80-26d140fc209c`

Rollout:

`~/.codex/sessions/2026/09/19/rollout-2026-09-19T12-26-27-01a0b77c-8c00-76c1-be80-26d140fc209c.jsonl`

Within the Brisbane-day boundary:

- automatic Goal turns: **248**;
- token-bearing model-response records within those turns: **994**;
- automatic turns with no tool call: **70**;
- automatic turns with exactly one tool call: **77**;
- conservative near-empty turns: **55**;
- input tokens: **119,246,122**;
- cached input tokens: **117,427,456**;
- output tokens: **256,246**;
- total input plus output: **119,502,368**;
- automatic continuation interval: **13:12:23–21:07:45 Brisbane**;
- recorded `usage_limit_exceeded`: **16:57:37 Brisbane**.

This thread mostly represents the broader form of the bug: automatic turns containing polling, orchestration calls, or short progress replies. It is not principally an empty-final chain, so PR #44320 would not necessarily stop it.

### Thread `01a0b786-a3f3-7120-b40d-33196c23e7ea`

Rollout:

`~/.codex/sessions/2026/09/19/rollout-2026-09-19T12-37-29-01a0b786-a3f3-7120-b40d-33196c23e7ea.jsonl`

Within the Brisbane-day boundary:

- automatic Goal turns: **578**;
- token-bearing model-response records within those turns: **882**;
- automatic turns with no tool call: **532**;
- automatic turns with exactly one tool call: **6**;
- successful automatic turns with a zero-length final answer: **513**;
- conservative near-empty turns: **523**;
- input tokens: **115,811,906**;
- cached input tokens: **114,000,896**;
- output tokens: **155,333**;
- total input plus output: **115,967,239**;
- traffic attached to empty-final turns: **74,287,995 input plus output tokens**;
- automatic continuation interval: **15:08:34–20:57:45 Brisbane**;
- longest uninterrupted empty-final chain: **157 turns**, from **20:39:56 through 20:52:36 Brisbane**;
- recorded `usage_limit_exceeded`: **16:57:33 Brisbane**.

A representative empty turn is `01a0b91b-2d6c-7ef2-a93f-6e9d9b61e2b8`. Its rollout sequence is:

1. `task_started`;
2. a 6,624-character Goal continuation input;
3. a reasoning item;
4. an assistant `final_answer` item whose `output_text` length is zero;
5. a `token_usage_record`; and
6. `task_complete` with a zero-length `last_agent_message` and no error.

The next Goal turn starts immediately because the Goal remains active.

### Shared usage-meter evidence

Both rollouts report the same account-level weekly meter. It moved from 87% used near the start of the sessions to 100%, and both threads received `usage_limit_exceeded` four seconds apart. After capacity reset, the meter again rose rapidly while both Goal sessions continued. Examples in UTC:

- 90%: `2026-09-19T03:37:12Z`;
- 95%: `2026-09-19T05:24:29Z`;
- 99%: `2026-09-19T06:35:53Z`;
- 100%: `2026-09-19T06:47:43Z` and `06:48:22Z`;
- both Goal turns fail on the limit: `06:57:33Z` and `06:57:37Z`;
- after reset, the second rollout reports 10% at `07:42:48Z`, 25% at `07:49:27Z`, and 75% at `08:48:28Z`.

Because other sessions were also active, these percentages corroborate the incident but are not suitable for per-thread attribution.

## Public issue and fix map

### Most directly relevant

- [#44909 — measured cost of false Goal continuations across 3,808 sessions](https://github.com/openai/codex/issues/44909) is the best mechanism report. It traces immediate idle continuation in 0.154.0, reports a 0.03-second median restart gap, and measures large full-context replay costs. A maintainer confirms that the new empty-final breaker does not cover repeated polling or non-empty responses.
- [#44735 — chat pause leaves Goal auto-continuing with rapid empty final answers](https://github.com/openai/codex/issues/44735) records ten completed empty turns in 44 seconds. The maintainer identifies the pause support and empty-turn breaker as newer than 0.154.0 and destined for 0.155.0.
- [#45026 — scheduled stop creates thousands of empty turns and hides transcript history](https://github.com/openai/codex/issues/45026) records 4,268 Goal continuations and 3,769 empty finals on 0.154.0, including continued activity around a usage-limit boundary.
- [#45864 — hundreds of empty Goal turns followed by blank history](https://github.com/openai/codex/issues/45864) reports 562 empty turns and 133.86M recorded tokens after the last meaningful turn.
- [#34248 — unbounded no-progress Goal loop](https://github.com/openai/codex/issues/34248) collects several variants, including thousands of duplicate turns and a 0.155.0 report that the new breaker can over-block.
- [#45974 — repeated model wakeups to poll deterministic jobs](https://github.com/openai/codex/issues/45974) describes the still-open polling form and argues for runtime-owned waiting followed by one completion wake.
- [#28144 — wait/wake support for Goals without spending tokens](https://github.com/openai/codex/issues/28144) requests a durable waiting state and event/timer wake.
- [#32188 — event-driven wake when a background exec session completes](https://github.com/openai/codex/issues/32188) is the main implementation discussion for letting an idle thread sleep until a tracked process exits.
- [#46085 — event-driven wake when a local process completes](https://github.com/openai/codex/issues/46085) describes the required process-owned wait semantics.

### Historical guardrails already merged

- [PR #23094](https://github.com/openai/codex/pull/23094), merged 18 May 2026, adds `blocked` and `usageLimited` Goal states and stops continuation after a usage-limit failure or repeated impasse. It addresses #22833, #22245, and #23067.
- [PR #41243](https://github.com/openai/codex/pull/41243), merged 28 August 2026, adds the stable `sleep_tool` feature and an `always_on` mode that registers `clock.sleep` independently of model catalog metadata.
- [PR #41454](https://github.com/openai/codex/pull/41454), merged 29 August 2026, blocks after three qualifying execution-host failure turns. Any successful tool resets its streak.
- [PR #44290](https://github.com/openai/codex/pull/44290), merged 9 September 2026, allows the model to persist `paused` after an explicit user request.
- [PR #44320](https://github.com/openai/codex/pull/44320), merged 9 September 2026 and released in [0.155.0](https://github.com/openai/codex/releases/tag/rust-v0.155.0), blocks after three consecutive automatically admitted turns with an empty final answer and no other activity.

The incident binary was based on `rust-v0.154.0`. That tag is not a descendant of PR #44320's commit `0735c519789d300097554425cc3d7cf3f2d718a1`. The empty chain therefore reproduces the exact pre-fix behavior described in #44735 and #45026; it is not evidence that the 0.155.0 breaker failed.

### What can be used without a custom implementation

Status as of 20 September 2026:

| Option | Upstream status | Usable without custom code? | What it actually covers |
|---|---|---|---|
| PR #44320 empty-turn breaker | Merged; present in installed `0.155.1+local.4` | Yes; already active | Stops the historical literal empty-final chain after three qualifying turns. Tool calls, non-empty commentary, and non-empty replies reset the streak. |
| PR #41243 `sleep_tool` `always_on` mode | Merged; supported by the installed build | Yes; configuration only | Gives every model a bounded sleep tool. It can reduce polling frequency, but the model still wakes after a sleep and it is not a zero-inference wait. |
| PR #23094 usage-limit and blocked states | Merged; already active | Yes; already active | Stops continuation after a recognized limit or impasse. It does not prevent the usage spent before the stop. |
| Issues #28144, #32188, #45974, and #46085 | Open proposals | No | Describe the full runtime-owned wait and exactly-once wake design, but no corresponding PR is merged into `openai/codex`. |

The best upstream-only mitigation is therefore to keep the current 0.155.1 build and enable the official sleep tool for all models:

```toml
[features.sleep_tool]
mode = "always_on"
```

At the time of this investigation, `~/.codex/config.toml` contained no explicit `sleep_tool` override, so the installed client used the default `model_driven` selection. This setting is a mitigation, not a complete repair: the maintainer response in [#44909](https://github.com/openai/codex/issues/44909#issuecomment-5647242265) explicitly confirms that the empty-turn breaker does not cover repeated polling or non-empty responses.

There are reference implementations outside the upstream repository, but none qualifies as a simple official fix:

- [vincenthcui/codex PR #1](https://github.com/vincenthcui/codex/pull/1) implements a timer-owned Goal continuation handoff in a personal fork. It was merged only into that fork and explicitly leaves the Desktop/automation host integration outside its scope.
- The comments on [#32188](https://github.com/openai/codex/issues/32188) link proof-of-concept branches, a tested personal-fork implementation, and unofficial downloadable binaries. They are useful design evidence, but adopting them would itself be a custom build and would assume responsibility for rebasing, validation, and security.
- `codex queue --thread ... --message ...` can be used by an external supervisor to wake a thread after work completes, but assembling cancellation, deduplication, output bounds, and failure handling around it is another custom solution.

Consequently, there is no upstream drop-in that fully fixes the broader incident shape. The literal-empty symptom is fixed, and sleep can make polling cheaper; eliminating model calls while deterministic work is still running still requires the event-driven runtime design described by the open issues.

### Other useful diagnostics supplied by users

- [#37304](https://github.com/openai/codex/issues/37304): 7,692 self-triggered turns, 7,682 empty strings, 47 compactions, and 1.041B recorded tokens after a stop request.
- [#37800](https://github.com/openai/codex/issues/37800): 2,806 Goal inputs followed by 2,806 aborts, suggesting a distinct queued-continuation or admission race.
- [#36503](https://github.com/openai/codex/issues/36503): a hook rejected every tool, including `update_goal(blocked)`, producing 2,514 repeated attempts; this motivates a runtime-owned fail-closed stop.
- [#40929](https://github.com/openai/codex/issues/40929): resumed Goal repeatedly followed stale pause context despite an updated objective.
- [#37299](https://github.com/openai/codex/issues/37299): 8,744 of 11,002 model-visible calls were wait-family calls and 83% of `wait_agent` calls timed out.
- [#38495](https://github.com/openai/codex/issues/38495): a non-Goal analogue where waiting on one long command became 90 model turns and 21.6M input tokens.

Public reports often use “tokens” for different counters. Preserve the distinction between provider response traffic, cached input, Goal-accounted usage, and account-plan percentage.

## Current source map

Source map verified at checkout HEAD `48780d7c6e3ccbc32d8cce8de03a746be8a1cb4d`.

```text
turn finishes
  codex-rs/core/src/tasks/mod.rs:819-865
    -> idle lifecycle, after active turn clears
       codex-rs/core/src/tasks/lifecycle.rs:43-68
         -> GoalExtension::on_thread_idle
            codex-rs/ext/goal/src/extension.rs:176-189
              -> GoalRuntimeHandle::continue_if_idle
                 codex-rs/ext/goal/src/runtime.rs:418-484
                   -> CodexThread::start_turn_if_idle
                      codex-rs/core/src/codex_thread.rs:334-352
                        -> automatic TurnInput admission
                           codex-rs/core/src/session/turn_input.rs:338-470
                             -> ordinary run_turn / model request / tool loop
                                codex-rs/core/src/session/turn.rs:424-705
```

The important current behaviors are:

- `continue_if_idle` admits another synthetic turn whenever the Goal is still `active`, the thread is idle, and no continuation deferral is present. There is no general delay or progress-sensitive backoff.
- a tool call sets `needs_follow_up`, so its output is followed by another model response within the same logical turn;
- token usage is recorded at response completion and then accumulated into Goal usage through `GoalExtension` and `GoalStore` (`ext/goal/src/extension.rs:419-515`, `ext/goal/src/runtime.rs:532-592`, `state/src/runtime/goals.rs:499-610`);
- usage-limit errors map to `usage_limited`, and non-active statuses are no longer eligible for continuation (`ext/goal/src/extension.rs:388-412`, `ext/goal/src/runtime.rs:269-391`);
- the current empty-response breaker only increments when the turn is known to be an automatic Goal continuation, has an empty final answer, and recorded no other activity (`ext/goal/src/accounting.rs:163-230`);
- tool calls, non-empty commentary, non-empty no-progress replies, Goal changes, and user turns reset that empty streak.

The current checkout also contains event-driven completion paths for yielded `exec_command` processes and terminal child-agent completion. Preserve and compare those mechanisms when investigating Goal wait ownership: a completed process or child should wake its parent exactly once without model polling. The analogous remaining problem is waits whose ownership is not represented by those paths, including repeated Goal polling and nonterminal child-status polling. Existing terminal-child coverage is in `codex-rs/core/tests/suite/subagent_notifications.rs` (`multi_agent_v2_terminal_child_wakes_idle_parent_once`).

## Root-cause questions

The historical empty-final loop has a straightforward proximate cause: the incident binary did not contain the three-empty-turn breaker. The deeper design question remains open because current Goals still restart immediately for many no-progress shapes.

Investigate these separately:

1. **Admission:** Why is another Goal turn necessary at this idle boundary? Record the prior turn's trigger, outcome classification, tool activity, Goal status/version, pending mailbox state, and deferral state at the decision point.
2. **Progress:** Which events represent substantive progress? Literal emptiness is too narrow. Distinguish productive tool work, a live runtime-owned wait, repeated status polling, repeated non-empty pause text, reasoning-only output, failed state transitions, and a response with no final item.
3. **Wait ownership:** Can the runtime represent each live external process or subagent as a keyed wait and derive Goal deferral from the set of live waits? A single thread-level boolean is insufficient when several waits coexist.
4. **Wake semantics:** Completion should enqueue one bounded event and admit one follow-up turn. Cancellation, duplicate completion, already-finished commands, restart recovery, and user steering need explicit behavior.
5. **Stop authority:** A model-issued `update_goal` call cannot be the only circuit breaker if hooks, tool routing, or execution-host failure can prevent that call. Determine which stops must be runtime-owned.
6. **Accounting:** Attribute every response to thread, turn, root turn, Goal, and trigger source. Report cached and uncached input, output, Goal units, and plan-meter changes separately.

## Discriminating tests

Existing coverage includes:

- empty finals block after three turns: `codex-rs/app-server/tests/suite/v2/thread_goal_empty_responses.rs`;
- empty-streak reset rules: `codex-rs/ext/goal/tests/accounting.rs`;
- usage-limit accounting and stop status: `codex-rs/ext/goal/tests/goal_extension_backend.rs`;
- idle lifecycle ordering: `codex-rs/core/src/session/tests.rs`;
- automatic turn admission: `codex-rs/core/tests/suite/turn_input_submission.rs`.

Highest-value additions for a future investigation:

1. Replay three literal empty finals against the exact 0.154 incident build and current build. The current build should make only three automatic requests and persist `blocked`.
2. Replay an empty response that emits `response.created` and `response.completed` with usage but no assistant final item. Current item-based accounting may classify this differently from a zero-length final.
3. Replay three semantically identical non-empty pause/status replies. Decide whether the runtime should block, back off, or require a persisted wait/pause state.
4. Run a live deterministic process under an active Goal. Assert that parent-model input while waiting is zero until the completion event, and that exactly one follow-up turn is admitted.
5. Repeat the wait test with two simultaneous processes and with a subagent. Completion of one must not clear deferral while another owned wait is live.
6. Inject `usage_limit_exceeded` during an automatic continuation through the mocked Responses transport and assert final accounting, `usage_limited`, and no subsequent model request.
7. Make the stop/update tool fail through the same hook or tool path that caused the impasse. Assert a bounded runtime-owned terminal state.

Tests should assert causal events and request counts. Timeouts are hung-test guards, not the correctness oracle.

## Suggested diagnostic record

For each continuation decision, persist a bounded record containing:

- thread ID, turn ID, root turn ID, and Goal ID;
- Goal status and monotonic Goal revision read before admission;
- idle cause and proposed continuation trigger;
- prior turn classification: final text length, item kinds, tool calls and outcomes, error kind, and token usage;
- pending user/mailbox input and continuation-deferral state;
- live runtime-owned wait handles, represented by type and opaque ID only;
- admission result: started, coalesced, deferred, rejected, or stopped; and
- the reason code for that result.

Keep user text and tool output out of this record. Bound both record size and in-memory retention, while preserving the existing disk diagnostic evidence policy.

## Reproduction and analysis notes

To re-check the incident without exposing content:

- classify an automatic Goal turn only when its turn records contain the internal Goal continuation input;
- join `token_usage_record` to automatic turns by `turn_id`;
- count tool calls from `response_item` records with `function_call` or `custom_tool_call`;
- determine empty success from `task_complete.error == null` and zero-length `last_agent_message`, then confirm the assistant final item has zero-length output text;
- use `event_msg.payload.rate_limits.primary.used_percent` only as a shared-account corroboration signal; and
- do not sum cumulative `token_count.info.total_token_usage` snapshots. Sum the per-response `token_usage_record.payload.usage` values instead.

The local Goals DB and rollouts are diagnostic evidence and must not be deleted or sanitized in place.
