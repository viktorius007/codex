PASS: the integrated `BodyAfterPrefix` fallback and both fixture corrections exactly match the requested resolution, and all four affected `BodyAfterPrefix` integration tests pass.

## Integrated production accounting

`/private/tmp/codex-cache-integration/codex-rs/core/src/session/context_window.rs` now computes:

```rust
let stored_context_tokens = sess.get_total_token_usage().await;
let active_context_tokens = stored_context_tokens.saturating_add(pending_tokens);
// ...
let baseline = window.prefill_input_tokens.unwrap_or(active_context_tokens);
```

This is the required correction. Before the first server-observed prefill, the complete projected request is the baseline, so pending input does not become fictitious post-prefix growth. After a prefill exists, scoped growth remains `(stored + pending) - prefill`, which counts pending input exactly once. The independent full-context-window check still uses `active_context_tokens`, so oversized requests still trigger the hard cap.

## Integrated fixtures

Both requested one-literal changes are present in `core/tests/suite/compact.rs`:

- `auto_compact_body_after_prefix_counts_growth_after_compaction` uses `model_auto_compact_token_limit = Some(80)`.
- `auto_compact_body_after_prefix_still_caps_at_context_window` gives its first response `input_tokens = 10` and `output_tokens = 5`.

`body_after_prefix_model_switch_budget_compacts_with_next_model` retains its original fixture, as required; its three expected requests remain previous-model sampling, next-model compaction, and next-model continuation.

## Executed evidence

`/private/tmp/codex-cache-run/prefix-and-lifecycle-green.log` records PASS for all four relevant tests:

- `auto_compact_body_after_prefix_counts_growth_after_compaction`
- `auto_compact_body_after_prefix_ignores_starting_window_prefix`
- `auto_compact_body_after_prefix_still_caps_at_context_window`
- `body_after_prefix_model_switch_budget_compacts_with_next_model`

The supplied run executed 15 selected tests: 14 passed and one unrelated `prompt_cache_key::fresh_subagent_reuses_root_cache_prefix_until_its_role_boundary` test failed. That separate failure does not weaken the four direct discriminators reviewed here and is owned outside this verification assignment.

## Verdict scope

The earlier three failures were correctly divided into one production fallback defect and two stale threshold fixtures. The integrated correction preserves `Total` scope, established-prefix accounting, fallback-buffer behavior, the hard context-window policy, model selection, and pending-input/history mutation order. No remaining `BodyAfterPrefix` accounting finding blocks this pending-input slice.

This was a read-only source review. I made no production/test edits and ran no Cargo command; only this report and its pointer report were updated. The cleanup check found about 22 GiB free, and this verifier created no build artifacts.
