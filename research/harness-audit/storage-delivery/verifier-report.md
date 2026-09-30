PASS
COMMIT: `130df36be5caa03e9204ac681fba899502252fd6` on `work/harness-test-storage` (test-only follow-up to production `d75e535d2a`).
COMMAND: `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store -E 'test(archive_crash_after_rename_keeps_paginated_context_resumable) | test(missing_selected_paginated_rollout_does_not_use_older_or_wrong_owner_archive)'` — 2 passed, 256 skipped at this SHA.
COMMAND: `git diff --check HEAD^ HEAD` — clean; `git status --short` showed no uncommitted production or test source.
GRAPH: Root project generation `2026-09-23T13:16:09Z` had no recorded gaps for resolver, archive, helpers, model-context, or unarchive files. Rust semantic edges remain partial; this follow-up's test-only diff was reviewed directly in the worktree.

| Criterion | Evidence at verified SHA | Verdict |
| --- | --- | --- |
| Recover intact SQLite-selected paginated context after rename before metadata commit | `archive_thread.rs:408-520`: a stale SQLite row and moved selected file survive store/database reopen; recovered complete items equal the pre-crash selected context. Independent focused run passed. | Proven |
| Choose exact selected archived counterpart, not earlier immutable history | `archive_thread.rs:413-433,467-470,518-531`: distinct older and selected messages plus a replacement rollout ID, exact recovered items, and exact archived path. `archive_thread.rs:555-601` rejects an older archive when the selected file is absent. | Proven |
| Reject a counterpart owned by another thread | `archive_thread.rs:603-620` puts a wrong-owner file at the selected archive name and requires `None`; tester's fixture discriminator changed its owner to the requested ID and observed the assertion fail in `/private/tmp/codex-storage-owner-discriminator.log`. | Proven |
| Respect active-only exclusion and leave SQLite unchanged on read | `archive_thread.rs:533-545` requires active-only resolution to return `None` and compares the full SQLite row after recovery with the stale row before it; `:621-628` repeats the row check after failed resolution. | Proven |

## Findings

None in the verified diff. The earlier missing-protection finding in `verifier-initial-report.md` is closed by this test-only commit. The fixture discriminator that substituted older bytes produced an exact `older history` versus `selected history` failure in `/private/tmp/codex-storage-selected-discriminator.log`; both restored fixtures passed in my independent run.

## Checks and limits

The helper in `archive_thread.rs:182-210` writes test-owned JSONL into temporary files and is used twice; no production API or behavior changed in this commit. The tests compare independent pre-crash and post-crash context plus the selected path, and the negative fixture checks the two unsafe fallbacks directly. The production resolver branch remains the narrow, read-only implementation reviewed at `d75e535d2a` (`thread_rollout_resolver.rs:109-137`).

The builder's prior full `codex-thread-store` suite (257 passed) and scoped lint were at `d75e535d2a`; the focused run here compiled the new tests at `130df36be5`. Per the coordinator's allocation, no full suite, lint, or mutation sweep was run at this tip. Disk had 26 GiB free, and the existing 6.1 GiB target was retained for coordinator cleanup. No verifier code edits were made.
