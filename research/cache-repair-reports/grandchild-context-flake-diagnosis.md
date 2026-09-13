# `grandchild_full_fork_preserves_context_baseline` flake diagnosis

## Verdict

A clear fixture race exists in the descendant completion synchronization. It is a missed transient **status**, not a lost event and not evidence that forked history is incorrect.

The two failure logs do not identify which of the fixture's two ten-second `timeout` blocks expired, so they cannot prove the call site by themselves. The code-level race nevertheless matches the observed signature exactly: each first attempt ends at about ten seconds with only `deadline has elapsed`, while the retry completes in about 0.4 seconds and reaches all context assertions.

## Evidence

- `final-full-workspace.log` lines 7728-7747: `paginated_full_history` fails at 10.279 seconds with `deadline has elapsed`, then passes at 0.402 seconds.
- `final-full-workspace-2.log` lines 7672-7694: `legacy_full_history` fails at 10.231 seconds with the same error, then passes at 0.389 seconds.
- The fixture records both descendant model requests, obtains each thread, then polls `thread.agent_status()` until it equals `AgentStatus::Completed(_)` (`subagent_notifications.rs` lines 1307-1340).
- Status is mutable current state. `agent_status_from_event` maps a successful `TurnComplete` to `Completed`, the next `TurnStarted` to `Running`, and a failed follow-up completion to `Errored` (`core/src/agent/status.rs`). Polling every 10 ms can therefore miss a short-lived successful completion.
- Multi-agent-v2 child completion now sends its parent an `InterAgentCommunication` with `trigger_turn: true` (`core/src/session/mod.rs` lines 2279-2285). The communication handler queues it and asks the pending-work scheduler to start an idle turn (`core/src/session/handlers.rs` lines 79-95).
- In this fixture the child spawns the grandchild. If the grandchild's completion reaches the child after the child's post-tool model request has already been assembled, the child can finish its original turn and immediately start an automatic completion-driven turn. The status poll can first observe `Running`. The catch-all `_parent_followups` mock has exactly two responses (`subagent_notifications.rs` lines 1267-1274), covering the ordinary root and child post-tool requests. An additional automatically started request may therefore fail and leave the child `Errored`; the current loop accepts neither state and runs until its ten-second deadline.
- If the grandchild completion is incorporated before the child's final response boundary, no extra child turn is needed and the child remains `Completed`; this accounts for the rapid retries. Full-history setup has extra rollout persistence/materialization work, which can shift this ordering under suite load. The alternating legacy/paginated failures give no evidence of a mode-specific content defect.
- The request polling is not inherently lossy: `ResponseMock::requests()` retains captured requests and the fixture filters them by exact `agent_name`. The initial root `submit_turn` also already consumes the concrete root turn completion. The mutable descendant status poll is the synchronization point that cannot observe history.

## Smallest evidence-backed correction

For each captured descendant request, extract `client_metadata.turn_id` along with `thread_id`. Replace the `agent_status() == Completed` polling block with `wait_for_event_match` for the exact original turn's `EventMsg::TurnComplete`, then assert that the event has no error:

```rust
let body = request.body_json();
let turn_id = body["client_metadata"]["turn_id"]
    .as_str()
    .expect("descendant turn id")
    .to_string();
let thread_id = ThreadId::from_string(
    body["client_metadata"]["thread_id"]
        .as_str()
        .expect("descendant thread id"),
)?;
let thread = test.thread_manager.get_thread(thread_id).await?;
let completed = wait_for_event_match(thread.as_ref(), |event| match event {
    EventMsg::TurnComplete(completed) if completed.turn_id == turn_id => {
        Some(completed.clone())
    }
    _ => None,
})
.await;
assert!(
    completed.error.is_none(),
    "descendant turn {turn_id} failed: {:?}",
    completed.error
);
```

`SessionIo::next_event()` receives from a queue, so the original `TurnComplete` remains observable even if a later `TurnStarted` has already changed current status. Matching `turn_id` prevents a later automatic turn from satisfying the oracle. Checking `error.is_none()` preserves the current test's requirement that the original descendant turn completed successfully.

Do not increase the timeout, accept `Running`/`Errored`, or add generic follow-up responses. Those changes either retain the race, weaken the success oracle, or make this fork-context fixture depend on completion-notification response ordering.

## Scope

Read-only diagnosis. No source files, Cargo files, or build artifacts were changed, and no tests or builds were run.
