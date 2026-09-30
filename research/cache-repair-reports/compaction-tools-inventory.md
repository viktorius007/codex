STATUS: DONE
COMMIT: none
CHANGED: 0 files
OPEN: 5
BLOCKERS: none; the diagnostic fingerprint will strengthen measurement but none of these fixes depends on it
---

# Compaction and tool-stability mechanism inventory

## Evidence boundary

This is a one-pass source audit of stages 3 and 5 at `ea55207054217cc00a541e385b5ffa8898c87b30`. It excludes prefix/plugin construction, fresh-context children, parent completion, and diagnostic design. The saved GitHub ledger was used only to select leads; every verdict below was re-established from the pinned worktree. No GitHub refresh was needed because the current source answered the implementation questions and no issue-specific detail was required.

The worktree is clean on `work/cache-mechanism-scout` at the requested commit. The `Users-viktor-Projects-github-codex` graph is ready at the same commit. I used graph search/path tracing for the relevant flows, then verified every relied-on source and test path directly in `/private/tmp/codex-cache-scout`. Final graph coverage verification reported `metadata_match` and no recorded coverage issue for all 27 relied-on paths. This is source evidence, not a claim about production cache-hit rates; the independent request-fingerprint diagnostic should measure those.

## Prioritized worklist

| Priority | Mechanism | Current consequence | Smallest landing unit |
|---|---|---|---|
| P1 | Local compaction sends an empty tool catalog | The compaction request has a different stable prefix from the surrounding turn | Carry the frozen step context into local compaction and build the same model-visible tool fields |
| P1 | Token usage has no explicit covered-history boundary | A usage-less model response can silently undercount history and delay compaction | Store the item boundary covered by the last trustworthy numeric usage |
| P1 | Pre-turn compaction excludes pending input from its decision | A near-full thread can make one oversized request and fail instead of compacting first | Estimate pending user/context items for the threshold without putting them in the compact prompt |
| P2 | Local compaction drops the complete operational tail | The summary becomes the sole record of recent tool state, increasing repeated work and later prompt divergence | Retain one bounded suffix of complete history groups beside the summary |
| P2 | MCP `tools/list_changed` is only logged | Tool capability can remain stale; a naive refresh would also invalidate an in-flight sampled call | Coalesce a pending refresh and consume it only at the next step-context capture boundary |

### 1. P1 — local compaction omits the advertised tool catalog

**Current evidence.** Ordinary sampling builds `Prompt.tools` from the already-frozen `StepContext` and sets `parallel_tool_calls: true` (`codex-rs/core/src/session/turn.rs:1386-1403`). Both remote compaction paths do the same (`core/src/compact_remote_request.rs:61-71`, `core/src/compact_remote_v2_attempt.rs:75-85`). Local compaction instead constructs `Prompt { input, base_instructions, ..Default::default() }` (`core/src/compact.rs:279-289`). `Prompt::default()` means an empty tool array, `parallel_tool_calls: false`, and no cyber-access program (`core/src/client_common.rs:41-51`). The local auto-compaction path receives a frozen `StepContext` earlier in the call chain but reduces it to a `TurnContext` before this builder.

That is a direct, deterministic prefix difference between a normal request and a local compaction request. It also deprives the summarizing model of the schemas that explain tool calls already present in the compacted input.

**Smallest direct fix.** Carry the same `Arc<StepContext>` through the local compaction entry point and set `tools`, `parallel_tool_calls`, and `cyber_access_program` exactly as remote compaction does. Preserve the intentional compaction setting `output_schema: None`. Manual `/compact` should capture one step context at its boundary and invoke the same builder. This is a narrow `codex-core` change with no protocol or public API change.

**Regression seam.** Extend `core/tests/suite/compact.rs`: configure at least one non-default/dynamic or MCP tool, capture a normal request and the immediately adjacent local compaction request through their `ResponseMock`s, and assert exact equality of the serialized `tools` arrays plus `parallel_tool_calls`. The existing request-shape snapshots primarily inspect input and would not catch this omission by themselves. The independent fingerprint should then show the same stable-prefix digest across that transition.

**Ownership/dependencies.** `codex-core`: `compact.rs`, the local auto/manual compact callers, and `StepContext`/`ToolRouter`. Land this as one production commit plus its focused integration test.

### 2. P1 — numeric usage is not tied to an explicit history boundary

**Current evidence.** A completed response without numeric usage returns without recording a usage record (`core/src/session/mod.rs:4377-4408`). Total-token calculation takes the last stored numeric total, finds the *most recent model-generated item*, and estimates only items after that inferred point (`core/src/context_manager/history.rs:662-693`). If a later response emits an assistant/reasoning/tool-call item but has no usage, that new item moves the inferred boundary even though the stored numeric total still describes an older response. User messages and tool outputs between the older sample and the new model item disappear from the estimate.

Current source is already correct for the simpler case the old ledger described too broadly: local user/tool items appended after a trustworthy latest model sample are estimated. The defect is specifically that a usage-less response can move an implicit boundary that it did not earn.

**Smallest direct fix.** Make `ContextManager` own the exact item cut/ordinal covered by the last accepted numeric usage. Update it only after that response's output has been recorded and its numeric usage accepted. Calculate estimated tokens after this explicit cut, rather than after the last model-generated item. Replacement, rollback, compaction, and recomputation must update/reset the boundary atomically. Resume should reconstruct it from the rollout order and `TokenUsageRecord`; guessing from the last model item would recreate the bug. A history version by itself is insufficient because it does not identify the covered item cut.

**Regression seam.** Add a history test with: accepted usage; appended user and assistant output; no usage update for that assistant response; another user item. The total must include every item after the accepted boundary. Add an integration case in `core/tests/suite/compact.rs` where that undercount would previously suppress threshold compaction. Add live/resume parity at the existing compaction-resume/session reconstruction seam so replay rebuilds the same boundary.

**Ownership/dependencies.** `codex-core` `ContextManager`, session token-recording order, compaction history replacement, and rollout reconstruction. The existing persisted token-usage record may be sufficient if its position is preserved; confirm during implementation before adding a wire field. Land before the incoming-input estimator so that estimator starts from trustworthy usage.

### 3. P1 — the pre-turn decision does not count pending input

**Current evidence.** `run_turn` invokes pre-sampling compaction before context updates and the new user message are recorded. The source contains an exact TODO to estimate pending context diffs/full reinjection plus user input (`core/src/session/turn.rs:163-185`). The decision therefore considers only stored history. Existing snapshots in `core/tests/suite/snapshots/` document that the incoming message is absent from the compact request and that a context-window error can follow. On `ContextWindowExceeded`, sampling sets total usage to full and returns the error (`session/turn.rs:1481-1487`); it does not compact and retry that turn.

The incoming user message should remain absent from the *compaction request* so it survives the replacement. It only needs to be included in the projected threshold calculation.

**Smallest direct fix.** Before the pre-sampling decision, compute a bounded model-visible token estimate for pending `TurnInput` and the pending initial-context/world-state update, without mutating history. Compare current trustworthy usage plus that projection with the compaction threshold. Keep the existing sequence: compact stored history first, then record/reinject the pending turn content. User input is the smallest useful first commit; context diffs/full reinjection complete the explicit TODO and can be a second tiny commit if needed.

**Regression seam.** In `core/tests/suite/compact.rs`, place stored history just below the threshold and make the incoming user message alone cross it. Assert request order: compaction first, then ordinary sampling containing the incoming user message; there must be no oversized sampling request. Reuse the existing incoming-shape fixture but assert both facts separately: counted for the decision and excluded from the compact prompt. Add one equivalent context-reinjection crossing case if that support lands separately.

**Ownership/dependencies.** `codex-core` session/turn decision code plus the existing response-item token estimator and initial-context/world-state rendering. No protocol change. This should follow mechanism 2.

### 4. P2 — local compaction discards the real operational tail

**Current evidence.** After the summarization response, local compaction extracts annotated user messages and rebuilds history from those plus one generated summary (`core/src/compact.rs:355-379`; builder around `compact.rs:683-760`). Assistant messages and completed tool call/output groups are discarded. The existing mid-turn compact snapshot demonstrates that a call/output pair can be present in the compaction request while the post-compaction history contains only user material plus summary.

This is an indirect cache-instability mechanism rather than a request-prefix defect. When the summary omits or distorts recent tool state, the agent repeats discovery or calls, which grows a different subsequent history. Remote v2 intentionally follows the provider compact endpoint's retained-message behavior and opaque compaction item, so it should not be changed as part of this local fix.

**Smallest direct fix.** For local compaction only, retain a small hard-capped suffix of complete history groups before the summary. Reuse the existing history-grouping logic so a tool call and its output are either both retained or both dropped. Preserve the existing global context bounds and 10K-per-item rule. Do not redesign remote compaction in this commit.

**Regression seam.** Drive local mid-turn compaction after a successful tool call/output. The next request must contain the newest complete group exactly once plus the summary. A focused history-builder test should prove truncation never splits a call/output group and obeys the cap.

**Ownership/dependencies.** `codex-core` local `compact.rs`, with a small reuse/extraction from `compact_remote_history` if necessary. The diagnostic sidecar can correlate repeated actions and history amplification, but it cannot by itself prove summary loss caused them.

### 5. P2 — MCP tool-list changes are ignored, and refresh must respect step atomicity

**Current evidence.** `LoggingClientHandler::on_tool_list_changed` only logs (`codex-rs/rmcp-client/src/logging_client_handler.rs:82-88`). Generic live MCP clients therefore never refresh their catalog from this notification. The catalog machinery itself is already strong: `ClientToolCatalog` publishes serialized revisions and holds revision authority during calls (`codex-mcp/src/client_tool_catalog.rs:27-94`); runtime binding caching keys on stable catalog revisions (`codex-mcp/src/runtime.rs:360-412`); each `StepContext` freezes the exact binding/router used for both sampling and tool execution.

An immediate notification refresh would be unsafe. A model can sample a call from revision N, then have that prepared call rejected after a mid-step publication of revision N+1. Existing binding tests deliberately prove stale calls are rejected and publication waits for active revision use. The correct boundary is therefore the next step-context capture.

**Smallest direct fix.** Give each managed client a coalescing `refresh_pending` signal. The notification callback marks that client's signal only. During the next binding/step-context capture, consume the marker, fetch through the existing `ClientToolCatalog::refresh`, and publish before freezing the new binding. A fetch failure keeps the old catalog and leaves/re-arms pending retry state. Avoid global catalog mutation. The practical ownership bridge is a small callback/channel injected into `ElicitationClientService`/`LoggingClientHandler` when `codex-mcp` creates the managed client.

**Regression seam.** Use the existing in-process mutable-tools server and catalog/binding helpers. Mutate the server and emit `tools/list_changed`; prove (1) an already captured binding can finish a call from its advertised revision, (2) the next captured binding sees the new schema, (3) one burst of duplicate notifications produces one successful revision advance, and (4) a failed refresh retains the prior usable catalog and retries later. `codex-mcp/src/connection_manager_tests.rs`, `binding_tests.rs`, and the runtime binding-cache tests are the natural seams. The request fingerprint should change only when the new catalog is successfully published.

**Ownership/dependencies.** A callback API in `rmcp-client`, managed-client/catalog state in `codex-mcp`, and the binding capture boundary. Two commits are easier to review and cherry-pick: notification plumbing, then boundary refresh with behavioral tests. This fixes stale capability correctness; a real server-declared schema change is an intentional cache-key change.

## Cases closed without a production fix

- **Catalog atomicity within a step is already implemented.** `capture_binding_with_metadata` builds one binding from resolved snapshots, and `StepContext` owns the same binding/router used by sampling and execution. Revision checks prevent a stale schema from silently executing. No additional “freeze” mechanism is needed.
- **Optional MCP startup grace is controlled behavior.** A pending optional server may be omitted after the configured grace; required/explicitly mentioned servers wait, and a usable cache can stabilize startup. Tests in `core/tests/suite/mcp_optional_startup_grace.rs` cover the first-turn policy. Later readiness changes capability at a step boundary because unknown tools cannot safely be advertised early. Waiting longer is a configuration/product tradeoff, not a surgical cache fix.
- **Tool ordering has deterministic producers in the inspected paths.** MCP normalization sorts by stable raw identity (`codex-mcp/src/tools.rs:197`), namespace merging uses a `BTreeMap` and sorts contained tools by name (`core/src/tools/spec_plan.rs:900-948`), and core/extension registration follows source order. Dynamic tools intentionally preserve caller order; JSON-object key order and server-provided schema order are outside the client's control. A cross-process serialized request equality test would be worthwhile assurance, but the audit found no current client-side ordering mechanism to fix.
- **Remote-v2 retained-history behavior is provider-shaped.** It keeps bounded selected messages plus the server's opaque compaction output while filtering call groups. Without a remote-v2 reproduction showing client-caused divergence, changing this would be speculative and is outside the direct Pareto set.

## Landing order and diagnostic dependency

Land 1 first for the direct request-prefix win. Land 2 before 3 so projected incoming usage builds on a valid covered-history boundary. Items 4 and 5 are independent and can follow as separate cherry-pickable units. None is blocked on instrumentation: their regression seams inspect request JSON or state transitions directly. The independent bounded fingerprint should validate the field-level effects and distinguish intentional catalog/compaction transitions from unexplained drift; no retention cap or deletion of diagnostic data is proposed here.
