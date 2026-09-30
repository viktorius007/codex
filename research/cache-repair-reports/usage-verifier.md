# Usage-boundary independent verification

PASS: the localized replay compatibility defect is corrected. Replay now preserves the last persisted UI usage snapshot while installing a separate, fully accounted active-context estimate for compaction. No valid production finding remains in the usage-boundary scope.

- Base: `3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18` on `work/issue-35935` (uncommitted worktree).
- Final reviewed usage-worktree diff SHA-256: `81e5ba26e52b0231870b5de9abd4890d2cded8d9dc7afcfaf4dd22b11cc794e2`.
- `git diff --check`: pass.
- The usage worktree and tested integration tree are byte-identical for `history.rs`, `history_tests.rs`, `rollout_reconstruction.rs`, `rollout_reconstruction_tests.rs`, and `state/session.rs`. The reviewed usage sections of `session/mod.rs` are identical; that file also contains separately owned agent-wakeup and prompt-prefix changes in integration. `session/tests.rs` differs only in rustfmt layout for two mechanical helper migrations; assertions and behavior are identical.
- Codebase-memory project `Users-viktor-Projects-github-codex` was ready. Coverage checks returned `metadata_match` / `no_recorded_issue`; direct worktree reads were authoritative because the graph indexes another checkout.

## Findings

None.

## Resolved full-suite regression

`record_initial_history_seeds_token_info_from_rollout` failed deterministically in the full `codex-core` run at `session/tests.rs:2890`. The rollout's last meaningful `TokenCount` contains cumulative usage 375, last usage 35, and context window 2,000. Before the correction, resume retained cumulative 375 but replaced last usage with the locally estimated active total 5,487 and replaced the context window with the current model's 258,400.

The existing oracle is correct. Before this patch, resume explicitly restored the last recorded `TokenCount` so UIs could show the persisted counters immediately. The new active estimate answers a separate question: how large the reconstructed model context is for compaction. `AcceptedTokenUsage` was introduced precisely to keep that active value separate from `TokenUsageInfo`.

The applied correction is the smallest maintainable separation:

1. In `apply_rollout_reconstruction`, estimate the fallback under the existing state lock and construct `AcceptedTokenUsage::fully_accounted(TokenUsage { total_tokens: estimated.max(0), ..Default::default() }, state.history.annotated_items().len())`.
2. Install the original replayed `token_info` and that resolved accepted value together with `set_token_info_and_accepted_usage`. Replay fallback no longer calls `set_recomputed_token_usage`.
3. Remove `set_token_info_and_accepted_usage`'s debug assertion requiring `TokenUsageInfo` whenever accepted usage exists. A rollout with no `TokenCount` still needs an internal active estimate while preserving the old UI state of `None`. Keep the boundary-length assertion.
4. Leave `set_recomputed_token_usage` unchanged for explicit live recomputation; existing tests correctly require that path to update the displayed last usage and current model window.

The existing UI test and the new legacy/post-compaction active-total tests form the exact combined oracle: `token_info() == Some(info2)` while `get_total_token_usage() == full prepared history + current base instructions`. All three pass after the correction.

## Why the production design is correct

- `AcceptedTokenUsage` binds three facts into one value: the accepted numeric total, the exact history-item boundary it covers, and whether encrypted reasoning was already included (`context_manager/history.rs:101-130`). Callers cannot independently update these fields.
- A live server usage report captures `items.len()` at receipt. Subsequent tool output, user input, and usage-less assistant responses remain after that cut and are estimated locally (`history.rs:677-690,720-770`). A response without usage leaves the earlier accepted pair intact.
- Active-context accounting reads only the paired accepted value. UI-only `TokenUsageInfo` can remain available after invalidation without being added to a full-history estimate. This prevents the stale-total double count that affected reset, removal, compaction, rollback, and legacy replay.
- `remove_first_item`, `replace_annotated`, and `replace_compacted` invalidate the accepted pair whenever they destructively change history (`history.rs:517-568`). Replay rollback and replacement paths also clear it (`session/rollout_reconstruction.rs:427-435`).
- Modern replay creates the accepted pair exactly where an ordered `TokenUsageRecord` appears. A later `TokenCount` may refresh UI data but cannot move the accepted cut, because its rollout position does not prove which pending items the server counted (`rollout_reconstruction.rs:347-441`).
- Replay with no authoritative pair, including legacy `TokenCount`-only and post-compaction histories, estimates the exact media-prepared history plus current base instructions under one session-state lock (`session/mod.rs:1536-1614`). It constructs a fully accounted `AcceptedTokenUsage` at the exact installed-history length and installs that beside the unchanged replayed `TokenUsageInfo`.
- Live recomputation fetches base instructions first, then estimates the current locked history and installs its numeric total and boundary under that same lock (`session/mod.rs:4447-4462`). The earlier stale-clone/current-length race is closed.
- Server-reported usage alone enables the encrypted-reasoning compatibility estimate, limited to the accepted prefix. Locally recomputed and forced-full totals are marked fully accounted, so prefix reasoning is not counted twice (`history.rs:693-770`).
- `set_token_usage_full` preserves cumulative UI bookkeeping while independently setting active usage to the entire context window. Prior cumulative usage 120 followed by a forced window 128 therefore reports active usage 128, not the eight-token UI delta (`history.rs:361-373`).

The patch adds no protocol field, wire-format change, dependency, configuration, sandbox behavior, or model-visible context item. Its public surface remains crate-private and small. Production changes are surgical; most of the diff is regression coverage.

## Executed evidence

- `initial-regressions-red.log`: the two original boundary regressions earned red before the fix (live actual 149 vs expected 270; resume actual 148 vs expected 209).
- `usage-and-pending-green.log`: **110/110 passed**. This covers the corrected modern replay boundary, live usage-less tail behavior, the real compaction threshold, incoming pending input, and the relevant existing history/replay suite. It ran on the immediately preceding production revision; the only later production change was the isolated forced-full correction exercised below.
- `usage-extra-and-transport-green.log`: **10/10 passed**, of which eight are the final focused usage cases: forced-full accounting under both reasoning policies; encrypted reasoning for server-reported and fully-accounted sources; replacement invalidation; oldest-item removal invalidation; legacy `TokenCount`-only fallback; and post-compaction `TokenCount`-only fallback. The two remaining passes are unrelated request-observer checks selected by the shared run expression.
- The final focused run compiled the byte-identical usage production files reviewed here. Its only warnings are pre-existing/unrelated `private_interfaces` in cache diagnostics and an unused import in `openai_file_mcp`; the earlier dead setter warnings are gone.
- `integrated-core-api-records.log`: the full crate run exposed the deterministic UI compatibility regression described above.
- `prefix-and-lifecycle-green.log`: the corrected existing UI test and both legacy/post-compaction active-total cases pass. The combined run reports 14/15 because a separately owned prompt-prefix test remains red; that failure does not execute usage reconstruction or accounting.

The earlier 107/108 and usage-less failures do not identify production faults. The first test estimated unprepared replay messages; canonical preparation added three `content_item_kinds=unknown` annotations, accounting for the exact 60-token difference. The corrected fixture preannotates those messages. The second fixture used `ev_completed`, which emits an explicit zero-valued usage record and therefore legitimately advances the boundary; the corrected fixture omits usage and now exercises the intended usage-less path. Both corrected cases pass in the final evidence.

## Residual limits

- No deterministic concurrency regression was added for live recomputation. The reviewed lock scope directly enforces the required invariant, and the relevant state mutation has no await inside that critical section, so this adds no separate finding.
- The complete `codex-core` crate suite did not finish green solely for this final isolated revision because independently owned integration tests remain under correction. The final regression run directly executes the three paths changed by the last usage correction; wider branch validation remains the coordinator's integration responsibility.
- The usage worktree has no local `target/` or formatter cache. The shared volume has about 22 GiB free at re-review time. Diagnostic logs were preserved as required.

## Reviewed files

- `codex-rs/core/src/context_manager/history.rs`
- `codex-rs/core/src/context_manager/history_tests.rs`
- `codex-rs/core/src/context_manager/mod.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/rollout_reconstruction.rs`
- `codex-rs/core/src/session/rollout_reconstruction_tests.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/state/session.rs`

No build, Cargo command, source edit, commit, or subagent was performed by this verifier.
