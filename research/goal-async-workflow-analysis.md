# Goal and asynchronous-tool workflow analysis

Assessment date: 20 September 2026, Australia/Brisbane.

## Scope and evidence boundary

This document preserves the coordinated assessment and subsequent discussion of how the proposed solutions can be layered. It assesses the harness as consumed by the model: useful results arriving when needed, no empty polling turns, no unwanted restarts, and no lost or duplicate process or agent completions.

The preferred outcome is a small set of correct private patches that remain straightforward to reapply after upstream updates. This is analysis, not an approved implementation specification or a claim that the proposed repairs have been tested.

| Field | Assessed state |
|---|---|
| Checkout | `43677b09a43f`, branch `local/customizations` |
| Installed binary | `codex-cli 0.155.1+local.4` |
| Relevant configuration | `[features.multi_agent_v2] wait_agent_enabled = false` |
| Investigation | Coordinator plus three fresh-context Sol source investigations |
| Verification | Direct source and existing-test inspection; independent review of delivery interleavings |
| Not performed | New live model probes, Rust builds, test execution, production or configuration changes |
| Graph coverage | Generation `2026-09-19T23:08:18Z`; cited paths checked, with no recorded file-specific issue |
| Graph limitation | Rust binding, expanded-call, and implementation relationships were partial; material conclusions used direct source fallback |

Primary research inputs:

- [Goal continuation usage incident](goal-continuation-usage-incident.md): historical usage measurements, circuit-breaker provenance, and Goal source map.
- [Wait-agent disablement evidence](wait-agent-disablement-evidence.md): polling measurements, local completion-patch provenance, configuration behavior, and prior live probes.
- [Cache repair delivery audit](cache-repair-reports/final-delivery-audit.md): delivered mechanisms and explicitly unfinished broader obligations.
- [Parent completion tests](cache-repair-reports/parent-completion-tests.md): historical test design and completion-delivery contracts; its execution-pending statements are historical, not a fresh test verdict.

The historical incident ran `0.154.0+local.3`; the assessed harness contains the newer empty-turn breaker. Historical counts do not demonstrate failure of that newer breaker.

## Overall conclusion

The harness has useful completion mechanisms, but no consistent agreement between tools, turn scheduling, and Goals about when the model should run again. The central flawed assumption is that an active Goal plus an idle thread necessarily means another inference is useful.

The missing distinction is between:

- **Runnable work:** the model has another useful action it can perform now.
- **Waiting work:** the model has handed work off, and its next useful action depends on a result or other external event.

The coordination gap explains unnecessary inference and exposes separate risks around result delivery. It does not establish that the whole harness requires replacement. Focused repairs to delivery, waiting, and automatic admission are the preferred starting point.

## Current control flow

An ordinary Goal continuation follows this sequence:

```text
Turn finishes
  -> active turn is cleared
  -> thread-idle lifecycle runs
  -> Goal remains active and is not deferred
  -> automatic turn admission succeeds
  -> another model request
```

`GoalRuntimeHandle::continue_if_idle` checks tool visibility, a persisted continuation-deferral marker, live thread availability, Goal status, and Core admission. It has no general delay, progress-sensitive backoff, or requirement for a changed external event.

Core already gives pending trigger-turn mailbox work priority over Goal admission and reserves the active-turn slot to prevent concurrent starts. Those are useful foundations to preserve.

Source: [Goal runtime](../codex-rs/ext/goal/src/runtime.rs), `continue_if_idle`, lines 418–516; [idle lifecycle](../codex-rs/core/src/tasks/lifecycle.rs), lines 43–68; [turn admission](../codex-rs/core/src/session/turn_input.rs), `start_if_idle`, lines 338–470. Line references throughout this document refer to the assessed checkout.

## What the three-empty-turn breaker does

The breaker counts a turn only when it was positively identified as an automatically admitted Goal continuation, emitted an empty non-commentary final-answer item, and recorded no other qualifying activity.

Non-empty assistant text, questions, non-empty reasoning summaries, tool activity, and other recognized activity reset the streak. A response that finishes without any final-answer item does not satisfy the empty-final condition and resets the streak at evaluation.

At the third qualifying automatic continuation:

1. Runtime accounts final progress and persists the Goal as `blocked`.
2. The objective and accumulated usage remain stored.
3. Active Goal accounting is cleared and a `ThreadGoalUpdated` event is emitted.
4. Goal-driven automatic continuation stops because the status is no longer `active`.

Ordinary conversation resume does not reactivate a blocked Goal. Reactivation requires an external Goal update explicitly setting `status: active`; the model's `update_goal` tool cannot resume it. The normal breaker path emits the status update, not a separate explanatory warning. A failure to block is logged as a tracing warning.

This is an emergency brake, not a waiting mechanism or general proof of lack of progress. It preserves the Goal but can leave autonomous pursuit blocked until resumed. Repeated polling or non-empty “still waiting” replies can avoid it.

Blocking a Goal also does not disable every automatic turn source: process and child-agent completion paths can still start a turn independently of Goal status.

Source: [Goal accounting](../codex-rs/ext/goal/src/accounting.rs), lines 163–230; [Goal runtime](../codex-rs/ext/goal/src/runtime.rs), lines 269–416; [Goal extension](../codex-rs/ext/goal/src/extension.rs), lines 305–328; [Goal API](../codex-rs/ext/goal/src/api.rs), line 144 onward. Existing coverage includes [accounting tests](../codex-rs/ext/goal/tests/accounting.rs) and [empty-response integration tests](../codex-rs/app-server/tests/suite/v2/thread_goal_empty_responses.rs); these were not rerun for this assessment.

## Tool collaboration from the consuming model's perspective

| Mechanism | Current behavior | Workflow consequence |
|---|---|---|
| Yielded `exec_command` process | Local watcher sends a completion fragment when the process exits | Completion-only polling should be unnecessary, subject to the delivery defects below |
| Child-agent terminal result | Queues trigger-turn mail and can wake the parent | Terminal completion does not require `wait_agent` |
| Child `send_message` | Queue-only delivery; does not normally wake an idle parent | A still-running child asking a question can stall until another wake source occurs |
| `followup_task` | Starts or steers a non-root target | Supports new work for an idle child, but cannot directly wake the root with a follow-up |
| Yielded `functions.exec` script cell | Retains terminal output for observation through `functions.wait` | Removing waits without a replacement leaves the model unaware of completion |
| `clock.sleep` | Keeps the tool call open until new input or its deadline | No inference during the sleep, but deadline expiry can still wake the model without useful news |
| Active Goal | Immediately attempts continuation at ordinary idle boundaries | Can defeat the model's attempt to yield until a background result arrives |

A code-mode script cell and a shell process launched inside it are separate resources with different lifecycle and completion ownership. Their appearance under the same `functions.exec` wrapper does not unify those semantics.

The historical incident already had the local process and terminal-child completion patches. Notifications existed while polling continued. Adding push notifications alone did not establish a correct waiting workflow. Disabling `wait_agent` removes one polling surface, while `list_agents`, empty `write_stdin` polls, and code-mode waits remain possible.

The default collaboration guidance also still names `wait_agent` in a shared list when the flag disables the tool. This is a small consistency repair, not a solution to the scheduling problem.

Source: [terminal-child forwarding](../codex-rs/core/src/session/mod.rs), lines 2253–2400; [message delivery](../codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs), lines 12–105; [collaboration guidance](../codex-rs/core/src/session/multi_agents.rs), lines 50–53 and 123 onward; [sleep handler](../codex-rs/core/src/tools/handlers/sleep.rs), lines 101–156.

## Concrete defects and remaining design gaps

### Completion can enter a retiring turn buffer

Two source investigations independently found this reachable interleaving:

1. `on_task_finished` takes the running task out of `active_turn`.
2. It drains that turn's pending input once.
3. It performs further asynchronous finalization while the old `ActiveTurn` remains installed with `task = None`.
4. A process exit calls `inject_or_start`.
5. That function sees `active_turn = Some(...)`, appends to the old pending-input buffer, and returns.
6. Finalization clears the old turn without another drain. The completion is discarded.

An arrival before the drain but after task removal can instead be recorded without causing the needed model follow-up. The post-turn scheduler only considers pending mailbox work; it does not recover this late response-item injection.

This is a source-established reachable race, not a live reproduction or a measurement of frequency. Child completion avoids this particular gap by using session-level mailbox storage.

The repair should preserve receipt outside a retiring turn buffer and atomically arrange consumption by the current or next turn. Merely detecting `task = None` and replacing the active slot is unsafe because finalization still owns that state.

Source: [turn finalization](../codex-rs/core/src/tasks/mod.rs), lines 618–632 and 844–865; [injection](../codex-rs/core/src/session/inject.rs), lines 20–45; [process watcher](../codex-rs/core/src/unified_exec/async_watcher.rs), lines 263–265.

### Result receipt and permission to restart are conflated

Idle process injection can start a regular turn directly. Trigger-turn child mail uses another scheduler. Neither path applies the Goal continuation check, and both can start inference after a user interruption. A paused, blocked, or usage-limited Goal therefore does not by itself prevent completion-driven inference.

Receipt should preserve the result. Automatic admission should separately decide whether processing it now is authorized and useful. That decision must respect the scope of a stop and the ownership of the pending work. A blanket prohibition whenever any Goal is non-active would also be wrong: the thread can contain other authorized work.

No duplicate-start defect was found in mailbox reservation itself: concurrent schedulers serialize through the active-turn lock. This does not prove exactly-once behavior across every delivery and cancellation path.

Source: [injection](../codex-rs/core/src/session/inject.rs), lines 35–45; [mailbox receiver](../codex-rs/core/src/session/handlers.rs), lines 82–97; [pending-work scheduler](../codex-rs/core/src/tasks/mod.rs), lines 441–500; [interruption](../codex-rs/core/src/tasks/mod.rs), lines 509–585.

### Code-mode waiting exposes runtime delays to the model

A yielded code cell survives ordinary turn completion. Its runtime buffers terminal output until an observer claims it, but no equivalent terminal push currently tells the model to collect it. Interruption terminates active code cells; session shutdown cancels and joins them. This is not restart-durable work. Unified-exec processes, in contrast, are retained across turn interruption, although shutdown terminates them.

The smallest promising code-mode change is to keep one `functions.wait` invocation open and perform bounded runtime waits internally instead of returning empty timed yields to the model. It can reuse the existing result claim and remote protocol. Keeping the turn open also prevents Goal idle continuation during that wait.

The implementation would still need to handle:

- Real partial output: a yielded response can contain useful data, including information needed to interact with a running job.
- User steering and cancellation: the handler must remain responsive while observing the cell.
- Missing cells, host failure, shutdown, and termination: these must become bounded actionable outcomes, not internal retry loops.
- Bounded buffering and accurate total elapsed time for the single model-facing call.

This works only after an explicit wait has been established. If the model ends its turn after the initial cell yield, no open wait exists. Conversely, adding terminal push alone does not prevent Goal restarts before that terminal event arrives. A complete handoff needs an open wait or an explicit deferred-work state.

A future push path must share the runtime's terminal-result claim with `functions.wait`; an independent callback copying the same output could create duplicate consumption and wakeups.

Source: [wait handler](../codex-rs/core/src/tools/code_mode/wait_handler.rs), lines 100–170; [runtime service](../codex-rs/code-mode-runtime/src/service.rs), lines 121–155 and 198–209; [cell completion ownership](../codex-rs/code-mode-runtime/src/cell_actor/types.rs), lines 199–257; [session runtime](../codex-rs/code-mode-runtime/src/session_runtime/mod.rs), lines 40–53, 136–144, and 279–302; [code-mode interruption](../codex-rs/core/src/tools/code_mode/mod.rs), lines 146–160; [process retention](../codex-rs/core/src/unified_exec/process_manager.rs), lines 568–595.

### Bounded output must remain usable

The assessment itself encountered truncated child completion reports and required follow-up requests to recover missing conclusions. A hard cap protects context, but the result contract should provide a concise conclusion and a retrievable full result when needed.

Process completion currently caps the output field at 1,000 tokens but includes the command separately; that is not a hard cap on the entire fragment. Whole-envelope bounds should be reviewed when editing this path.

Source: [child completion formatting](../codex-rs/core/src/session_prefix.rs), lines 9–43; [process completion formatting](../codex-rs/core/src/context/exec_completion.rs), lines 15–28.

### Cache efficiency does not remove unnecessary inference

The incident's 98.5% cached-input rate coexisted with enormous unnecessary model traffic. Prefix stability repairs cannot remove redundant requests, repeated context additions, or subsequent compaction pressure. The records establish traffic and context growth; they do not quantify model attention degradation or reveal the provider's subscription billing formula.

## Solution ranking

Patch-size assessments below are qualitative estimates from the affected paths, not measured implementation diffs.

| Approach | Correctness against the desired workflow | Patch and rebase burden | Assessment |
|---|---|---|---|
| Reliable completion delivery plus explicit event-owned waiting using existing turn/tool machinery | High for the running-session workflow | Small to medium, separable patches | Best starting balance |
| Persistent keyed waits and restart recovery | Strongest broader lifecycle guarantees | Medium to large; more integration points | Add if restart survival is required |
| Suppress Goals whenever any background work exists | Cannot distinguish a dependency from a server or unrelated job | Small initially | Too blunt as the principal fix |
| Broaden the no-progress breaker | Useful protection, with false-positive risk for legitimate work | Small to medium | Secondary safeguard |
| Longer timeouts or backoff | Reduces waste but retains unnecessary wakeups | Small | Mitigation and fallback |
| Disable Goals or remove waiting tools indiscriminately | Removes capability or leaves results unobserved | Tiny | Does not satisfy the objective |

For explicit waits within a live session, holding the existing turn open avoids the need for a new persistent Goal status or a major public API redesign. Elapsed Goal active time continues accruing while a tool waits; avoiding inference does not imply stopping wall-time accounting. Persistence becomes relevant when intentionally ending the turn with outstanding work or surviving restart/disconnection.

## Combining the approaches without interference

The approaches are not mutually exclusive. They comprise core repairs, safeguards, and operational fallbacks. They can be layered if they share one delivery and wake contract.

Recommended initial layers:

1. **Reliable receipt:** retain a result until consumed, including across turn finalization.
2. **Correct waiting and admission:** an explicitly awaited event wakes the model when useful work becomes available; an unchanged timeout does not. Stop and pause decisions govern whether receipt may start inference.
3. **Circuit breaker:** if an unproductive continuation loop escapes ordinary waiting, stop it with a clear reason while preserving the Goal and pending results.

Persistent keyed waits can later extend the same state and ownership rules. They should not introduce a competing scheduler. Backoff belongs behind these layers for systems that cannot notify on change; where possible, polling stays inside the runtime rather than repeatedly involving the model.

Automatic suppression is safe only for explicitly awaited dependencies, not every background process. Removing a wait tool is safe only when a replacement covers its required behavior. Interactive `write_stdin`, intentional intermediate-output reads, and meaningful partial results remain legitimate.

| Interference risk | Required composition rule |
|---|---|
| Wait tool consumes a completion and a separate callback also wakes the model | Share one terminal-result consumption decision |
| Goal continuation and completion both try to start a turn | Join an existing turn or reserve one new turn; prioritize pending useful input |
| One completion clears another job's wait | Give each dependency a distinct identity |
| An unfinished job prevents processing a useful completed result | Distinguish “any result is useful” from “all results are required” |
| Pause/stop discards a late result | Separate receipt and retention from permission to infer |
| Breaker treats normal waiting as failure | Runtime-owned waiting should produce no empty model turns to count |
| Background server prevents all Goal continuation | Suppression must follow declared dependencies, not raw process existence |

## Recommended patch sequence and proof obligations

1. **Repair completion receipt.** Keep results outside retiring turn buffers and arrange exactly-once consumption by the current or next turn.
2. **Separate receipt from automatic restart.** Preserve late results after interruption; share an ownership-aware admission decision across completion paths.
3. **Provide consistent explicit waiting.** Keep waiting inside the runtime until a relevant result, requested useful output, user input, or genuine failure occurs. Reuse existing code-mode wait operations initially where feasible.
4. **Retain and improve the breaker.** Address the missing-final case and explain why a Goal was blocked, without treating arbitrary tool activity as proof of progress.

Before trusting implementations, causal regression tests should cover:

- Completion injected after the final pending-input drain but before the active turn is cleared.
- Completion consumed by an active wait without an additional duplicate inference.
- Completion racing Goal automatic admission.
- Two simultaneous processes and a child agent, with no cleared unrelated dependency or lost result.
- User interruption or an applicable pause followed by a late result: receipt retained, automatic inference governed by the stop policy.
- Useful partial output and interactive input while a job remains running.
- Code-cell host failure, termination, missing-cell responses, and user steering during an internal wait loop.
- Three qualifying empty automatic turns, a response with no final item, and non-empty polling turns.
- Restart recovery only if durability is included in the chosen scope.

Assert exact causal events, retained results, and model-request counts. Timeouts are hung-test guards, not correctness oracles. Existing tests and historical live probes are useful starting evidence, but they do not replace new tests for these combined interleavings.

## Decision boundary

The recommendation is a layered local repair centered on one shared result-delivery and wake contract. Broader breakers and backoff support that contract; they do not replace it. A durable general scheduler is a possible later extension, not the starting requirement.

No solution in this document has been implemented or benchmarked as part of this assessment. Restart survival, the exact model-facing handoff interface, and partial-output policy remain design decisions for a subsequent implementation task.
