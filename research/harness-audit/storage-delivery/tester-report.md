STATUS: DONE
COMMIT: 130df36be5caa03e9204ac681fba899502252fd6
CHANGED: codex-rs/thread-store/src/local/archive_thread.rs (test module only)
OPEN: 0
BLOCKERS: none

# Storage archive recovery regression tests

The existing crash fixture now has two paginated rollouts for one logical thread. The older rollout has its own message; SQLite selects a newer immutable rollout with a different rollout ID and a distinct message. The test moves the selected file to the archive, reopens SQLite, and compares the complete recovered model context with the context before the simulated crash. It also asserts the exact archived path, an empty active-only resolver result, and equality of all SQLite metadata before and after recovery (`archive_thread.rs:408-547`). The original rename, sync, reopen, and successful resume assertion remain.

A second fixture keeps an older archive while SQLite's selected active file is missing, then places a wrong-owner file at the exact selected archive name. Both include-archived lookups must return `None`; the SQLite row must remain unchanged (`archive_thread.rs:549-629`). Both fixtures use actual files and a temporary SQLite database.

## Oracle and discriminator

- Selected context: the fixture-owned `selected history` and `older history` messages distinguish the two rollouts before the crash. The recovered complete context must equal that pre-crash context, and the resolver path must be the selected file's archived counterpart. Replacing the selected archive's bytes with the older rollout's bytes after the rename made the equality assertion fail with `older history` versus `selected history` (1 failed, 257 skipped; `/private/tmp/codex-storage-selected-discriminator.log`). Restoring the fixture passed.
- Owner and selected identity: with only the older archived file present, the resolver returns `None`; with a matching filename but another thread's session metadata, it still returns `None`. Changing the wrong-owner fixture UUID to the requested thread UUID made the second assertion fail with `Some(ResolvedThreadRollout)` versus `None` (1 failed, 257 skipped; `/private/tmp/codex-storage-owner-discriminator.log`). Restoring the fixture passed.
- Audience and cost: a person resuming a thread after an interrupted archive could receive the wrong history, or lose access to the chosen history. SQLite rewriting during read could conceal the interrupted state. These assertions catch small changes that broaden fallback, skip ownership or scope checks, or update the row during recovery.

## Verification

- `UV_CACHE_DIR=/private/tmp/codex-harness-audit/storage/.uv-format-cache RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ just fmt` passed. Its unrelated Python formatting changes were restored to HEAD.
- `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store -E 'test(archive_crash_after_rename_keeps_paginated_context_resumable) | test(missing_selected_paginated_rollout_does_not_use_older_or_wrong_owner_archive)'` passed at the final source: 2 passed, 256 skipped (`/private/tmp/codex-storage-tests-green.log`).
- `git diff --cached --check` passed before the one-file commit. The worktree has only pre-existing uncommitted AGENTS.md and audit artifacts plus this report and its index entry. The current source has no uncommitted code change.

The full crate or workspace suite, lint, and mutation sweep were outside this focused allocation. The builder's prior full crate run was 257 passed at `d75e535d2a`, before these test changes. Independent verification should review this commit and rerun the focused tests at its SHA. No production file was changed by this commit.
