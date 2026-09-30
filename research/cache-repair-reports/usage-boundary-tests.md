STATUS: DONE
COMMIT: none
CHANGED: 2
OPEN: 5
BLOCKERS: none; the coordinator intentionally owns compilation in the shared integration target
---

# Test-writer deliverable: trustworthy token-usage history boundary

## Role result

Added two test-only regressions in `/private/tmp/codex-cache-usage-boundary`:

- `codex-rs/core/src/context_manager/history_tests.rs:788` exercises live history after an accepted numeric usage record. A matching tool output, user input, a later assistant response without usage, and another user input must all remain after the accepted accounting boundary.
- `codex-rs/core/src/session/rollout_reconstruction_tests.rs:194` replays the same shape with the accepted `TokenUsageRecord` preserved in rollout order and asserts resumed accounting includes the entire later tail.

No production file, protocol field, test hook, sandbox setting, or global configuration was changed. The test diff is 129 added lines across two existing test files. It remains uncommitted because the current implementation is expected to fail these red-first tests, and the brief prohibits committing an intentionally red tree.

## Gap proof

Executed before editing:

1. `rg -n --glob '*test*.rs' 'usage-less|without usage|missing usage|TokenUsageRecord.*ResponseItem|ResponseItem.*TokenUsageRecord|last accepted|covered.*history|usage boundary' codex-rs/core`
   - Result: no matching test.
2. `rg -n --glob '*test*.rs' 'total_token_usage_includes_all_items_after_last_model_generated_item|get_total_token_usage\(' codex-rs/core/src/context_manager codex-rs/core/src/session codex-rs/core/tests`
   - Result: only `codex-rs/core/src/context_manager/history_tests.rs:764` directly asserts this total.
3. `rg -n 'record_initial_history_.*(token|usage)|get_total_token_usage\(' codex-rs/core/src/session/tests.rs codex-rs/core/src/session/rollout_reconstruction_tests.rs codex-rs/core/tests/suite/*.rs`
   - Nearest reconstruction test: `codex-rs/core/src/session/tests.rs:2809` seeds the latest `TokenUsageInfo`, but never places model output without usage after the persisted record and never asserts the resumed total.

Nearest-test difference: the existing history test appends only local user/tool items after the newest model item. It never appends a later model item without a numeric usage update, so the current implicit "last model item" scan still selects the right tail. The existing resume test asserts the numeric record is restored, but not which history cut that record covers.

## Target enumeration and elicitation record

- Structural target: `ContextManager::get_total_token_usage` selects a tail boundary before summing estimated items; `Session::record_initial_history` reconstructs response-item history and separately restores the latest token information/record.
- Obligation authority: `/private/tmp/codex-cache-run/usage-boundary-brief.md` requires accepted numeric usage to remain paired with the history it covers and requires reconstruction not to recreate the undercount.
- Promise: callers receive an estimate of the active context total. An unwritten caller assumption is that a response lacking usage cannot retroactively mark earlier uncounted history as counted.
- Actors and worst legal input: the provider may complete a response with `usage: None`; users and tool execution may append legal model-visible items between the last accepted usage and that response; resume may rebuild the same order from persisted rollout items.
- Invariant: the covered-history cut advances only with accepted numeric usage; all later model-visible items remain estimated until a newer numeric usage is accepted.
- Boundary value: exactly one later model-generated item with no usage. With no such item, the existing test passes; adding it exposes the wrong boundary.
- Never-obligation: a usage-less response must never cause intervening tool output or user input to disappear from context-size accounting.
- Implicit contracts: ordering is load-bearing both live and during replay; the persisted `TokenUsageRecord` position is the available independent evidence for reconstruction.
- Happy/sad parity: accepted usage plus only local tail items is already covered at `history_tests.rs:764`; these tests add the usage-less-response path live and after resume with exact totals.

## Oracle and discriminator

The oracle independently enumerates the four response items after the accepted boundary and sums their existing per-item estimates onto the fixed accepted total of 100. It tests boundary membership rather than duplicating the production boundary-selection algorithm. Every fixture item has non-empty, distinct content, so omitting any of the tool output, first user item, usage-less assistant output, or final user item changes the exact expected total.

The discriminator is the `usage_less_assistant` fixture inserted after the accepted record with no second `update_token_info` call in the live test and no later `TokenUsageRecord` in the reconstructed rollout. On the current source, the last-model scan is expected to begin after that assistant item and therefore omit the earlier tool output, user input, and assistant output. This expected failure has not been claimed as observed: the coordinator directed this role not to compile and will execute it in the shared target.

Exact narrow invocation from `codex-rs`, after the coordinator exports its absolute `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=32`, and `CARGO_INCREMENTAL=1`:

```sh
just test -p codex-core -E 'test(usage_less_response_does_not_move_last_accepted_token_usage_boundary) | test(resumed_history_restores_last_accepted_token_usage_boundary)'
```

Required execution sequence:

1. Run the command against this test-only diff and record both named failures. For each failure, the expected side must exceed the actual side because the actual side omitted at least one pre-assistant tail item.
2. Freeze these two test files for the fresh production builder.
3. Run the identical command after the production fix and record both named tests passing.
4. The later mutation lane should mutate the boundary selection; a survivor returns as a new test finding.

```json
{
  "id": "wt-usage-boundary-001",
  "kind": "finding",
  "category": "gap",
  "status": "fixed-in-test-diff",
  "candidate": "accepted numeric usage + later tool/user/model/user tail + usage-less response -> every item after the accepted cut remains estimated live and after resume",
  "gap_evidence": {
    "greps": [
      "rg -n --glob '*test*.rs' 'usage-less|without usage|missing usage|TokenUsageRecord.*ResponseItem|ResponseItem.*TokenUsageRecord|last accepted|covered.*history|usage boundary' codex-rs/core",
      "rg -n --glob '*test*.rs' 'total_token_usage_includes_all_items_after_last_model_generated_item|get_total_token_usage\\(' codex-rs/core/src/context_manager codex-rs/core/src/session codex-rs/core/tests"
    ],
    "nearest_test": "codex-rs/core/src/context_manager/history_tests.rs:764; codex-rs/core/src/session/tests.rs:2809",
    "difference": "neither test places a later model-generated item without usage after an accepted record and asserts exact tail accounting"
  },
  "strategy": "private Rust module tests; the live ContextManager seam observes the exact total, and the Session resume seam is required because rollout ordering creates the reconstruction risk; no boundary-creating dependency is mocked",
  "changes": [
    {
      "path": "codex-rs/core/src/context_manager/history_tests.rs",
      "lines": "788",
      "point": "live usage-less-response boundary regression"
    },
    {
      "path": "codex-rs/core/src/session/rollout_reconstruction_tests.rs",
      "lines": "194",
      "point": "rollout-order reconstruction regression"
    }
  ],
  "summary": "A usage-less model response can no longer be treated as proof that earlier tail items were included in the last accepted numeric usage.",
  "evidence": [
    {
      "path": "codex-rs/core/src/context_manager/history_tests.rs",
      "lines": "825",
      "point": "exact live total oracle"
    },
    {
      "path": "codex-rs/core/src/session/rollout_reconstruction_tests.rs",
      "lines": "263",
      "point": "exact resumed total oracle"
    }
  ],
  "proof": {
    "oracle_reasoning": "the test-owned post-boundary item list determines membership independently; the existing estimator supplies only each selected item's token cost",
    "discriminator": "insert one assistant response with no numeric usage after the accepted record; execution pending coordinator-owned shared target",
    "validation": "just fmt passed; narrow red and post-fix green test runs pending coordinator"
  },
  "verdict": "self-check: REDO(executed discriminator and green validation pending coordinator target); KEEP pending those runs and the later mutation sweep"
}
```

Value bar: the small-edit regression is replacing the explicit accepted cut with a search for the latest model item, or advancing the cut when usage is absent. The protected audience is customers with long tool-using threads and operators diagnosing compaction. Shipping the breakage understates active context, delays compaction, and can cause an oversized request or misleading token telemetry.

Vacuity checks: the discriminator changes the named boundary property; each selected item has distinct positive cost; the assertion is exact rather than a contains/non-empty proxy; the usage-less premise is directly constructed; and the expected item membership comes from the acceptance requirement rather than the production scan. The provider boundary is represented by real rollout records and response items; only network/file I/O is absent, which does not create this ordering risk.

## Actual command outcomes

- `git diff --check` — exit 0.
- `just fmt` — first attempt reached Python formatting but failed because the default uv cache at `/Users/viktor/.cache/uv` is outside the writable sandbox.
- `UV_CACHE_DIR=/private/tmp/codex-cache-run/uv-cache just fmt` — exit 0.
- Narrow tests — not run by instruction; compilation is pending the coordinator's dedicated shared integration target.

## Graph reuse and coverage

Project `Users-viktor-Projects-github-codex` was ready. `search_graph` located `ContextManager::get_total_token_usage`, its nearest existing history test, `Session::last_token_usage_record_from_rollout`, and the rollout reconstruction seam. An inbound depth-1 trace found the production wrapper `SessionState::get_total_token_usage`. Direct assigned-worktree reads verified the source and assertions used for the test design.

`check_index_coverage` reported `metadata_match` and `no_recorded_issue` for all six relied-on paths: `context_manager/history.rs`, `context_manager/history_tests.rs`, `session/mod.rs`, `session/rollout_reconstruction.rs`, `session/rollout_reconstruction_tests.rs`, and `state/session.rs`. This is best-effort graph evidence; the direct reads are the source of truth.

## Remaining gaps

1. Coordinator-owned red run and post-fix green run are pending.
2. A fresh builder must implement the production boundary without changing these tests.
3. Replacement/compaction and rollback need focused lifecycle assertions once the chosen boundary representation makes their reset/rebase contract concrete.
4. A subsequent `core/tests/suite/compact.rs` integration test should place the trustworthy total just below the auto-compaction threshold, add an uncounted tool/user tail followed by a usage-less response, and assert that the next turn compacts rather than sending an oversized sampling request.
5. Diff-scoped mutation evidence remains for the repository mutation lane after implementation.
