FAIL: "A criterion with neither is unmet" — the crash regression does not prove the selected-file, identity, exclusion, and read-only recovery guards.
COMMIT: `d75e535d2af710d3a1d019df7a668f5c3a3ef25d` (`work/harness-fix-storage`).
COMMAND: `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store -E 'test(archive_crash_after_rename_keeps_paginated_context_resumable) | test(revert_keeps_thread_id_and_hides_suffix_across_repeated_reverts) | test(read_thread_returns_archived_rollout_when_requested) | test(read_thread_sqlite_fallback_respects_include_archived) | test(resolves_archived_ancestors)'` — 5 passed, 252 skipped at this SHA.
COMMAND: `git diff --check d75e535d^ d75e535d` — clean; `git status --short` showed only pre-existing audit artifacts and `AGENTS.md`.
GRAPH: `index_status` ready, generation `2026-09-23T13:16:09Z`; `check_index_coverage` found no recorded file gaps for the resolver, archive, helpers, model-context, and unarchive files. Rust call semantics are partial; direct worktree source supplied the verdict.

| Criterion | Evidence at verified SHA | Verdict |
| --- | --- | --- |
| Resume intact selected paginated context after rename before SQLite commit | `archive_thread.rs:373-465` asserts the stale SQLite path, moved file, and nonempty context after reopening; independent focused run passed. Baseline causal red is in `red-test-output.txt` (failure at `archive_thread.rs:463`). | Proven |
| Only the archived counterpart of SQLite's selected rollout; reject a mismatched thread or rollout | `thread_rollout_resolver.rs:97-135` constructs the selected basename and checks IDs. The sole new fixture has one correctly named, correctly owned rollout (`archive_thread.rs:376-384,419-425`); no wrong or older competing file or mismatched metadata is exercised. | Unproven by test |
| Respect archived exclusion and preserve earlier immutable history after revert | `thread_rollout_resolver.rs:109-137,160-175` retains the paginated stop and scope guard; independent ordinary archive and revert tests passed, but the crash fixture calls only `include_archived: true` and has no reverted history (`archive_thread.rs:410-465`). | Unproven in crash state |
| Do not mutate SQLite during a read | The new resolver performs reads only (`thread_rollout_resolver.rs:89-147`), but the crash fixture checks stale metadata only before loading (`archive_thread.rs:447-463`), never after. | Source clear; no regression proof |

## Finding — blocks integration

- `codex-rs/thread-store/src/local/archive_thread.rs:373-465`: The recovery test's single-rollout, positive-only fixture cannot distinguish the specified narrow recovery from a broad archived scan, a missing identity or scope check, or a database rewrite during read. Add a test-owned competing older rollout after revert, a wrong-owner archived counterpart, an active-only lookup, and post-read SQLite equality checks against the original stale metadata. Assert the selected context or path, not merely a nonempty item list. These may be split into focused tests by behavior. No production defect was demonstrated at this SHA.

## Checks and limits

- Read the committed resolver and archive files completely, plus the directly affected model-context, read-thread, lineage, helper, revert, and rollout-path code. The implementation visibly derives one counterpart from the selected path, checks filename rollout ID and session thread ID, obeys scope, and makes no SQLite write in the added branch (`thread_rollout_resolver.rs:109-137`).
- The builder reported the full `codex-thread-store` suite at 257 passed and scoped lint green; those results were inspected in `codex-rs/storage-green-suite.log` and `codex-rs/storage-fix.log`, not rerun. No workspace suite or mutation sweep was run: this was a bounded independent review with 14 GiB disk free and the 6.1 GiB target retained for the coordinator.
- Shape: no new abstraction, trait, error variant, or public API; reused `existing_rollout_path`, `rollout_id_from_path`, and `read_session_meta_line`. No added comment merely repeats code. The resolver change is 37 production lines and the regression is 96 test lines. No fix was made under the read-only verifier assignment; handoff is to a test author, then a fresh verifier.
