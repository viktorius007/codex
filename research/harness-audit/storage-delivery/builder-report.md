STATUS: DONE
COMMIT: d75e535d2af710d3a1d019df7a668f5c3a3ef25d
CHANGED: `codex-rs/thread-store/src/local/thread_rollout_resolver.rs`, `codex-rs/thread-store/src/local/archive_thread.rs`
BLOCKERS: none

# Archive crash recovery repair

An archive moves a rollout file before updating SQLite. If the process stops between those operations, the paginated resolver sees the selected active path missing and refuses to scan for a fallback. The existing red test reopened SQLite after exactly this rename and failed to load intact model context with `no rollout found for thread id` (coordinator replayed it twice on baseline `34d1bd0a11`; `red-test-output.txt` holds the auditor's earlier red run). Its fixture first proves the rollout can load, then verifies reopened SQLite still selects the missing active path and the archived file exists; the final assertion requires nonempty context for the same thread. The baseline failure is the executed discriminator for that oracle.

The resolver now derives one archived path from the SQLite-selected active basename only when that path is missing, its row remains unarchived, and the caller includes archived threads. It accepts the file only when its parsed rollout ID matches the selected path and its session metadata names the requested thread. It returns that file without changing SQLite. This retains the selected rollout after a revert instead of scanning older rollouts for the same thread. The production change is in `thread_rollout_resolver.rs:97-137`; the unchanged causal regression is in `archive_thread.rs:373-468`.

## Verification

| Command | Result |
| --- | --- |
| `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store archive_crash_after_rename_keeps_paginated_context_resumable` | 1 passed, 256 skipped (`codex-rs/storage-green-focused.log`). |
| `UV_CACHE_DIR=/private/tmp/codex-harness-audit/storage/.uv-format-cache RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ just fmt` | Passed (`codex-rs/storage-fmt.log`). Three unrelated Python formatter rewrites were restored. |
| `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just test -p codex-thread-store` | 257 passed, 0 skipped (`codex-rs/storage-green-suite.log`). |
| `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 just fix -p codex-thread-store` | Passed in 3m 36s without warnings or source edits (`codex-rs/storage-fix.log`). Initial sandbox run could not open Cargo's local locking listener; the approved rerun passed. |
| `git diff --cached --check` before commit | Passed. Exact two-file commit `d75e535d2a`. |

The graph search for archive-move recovery returned no existing helper; existing `existing_rollout_path`, `rollout_id_from_path`, and `read_session_meta_line` were reused. Graph index coverage showed no recorded gaps for the resolver, archive code, local helpers, and rollout library entry point; Rust call and implementation edges remain partial, so source was checked directly.

The retained worktree target measured 6.1 GiB after lint; 16 GiB was free on the shared disk. The crate suite was run after formatting, and lint made no source edits. The workspace suite was not run because this change touches only `codex-thread-store`. The symmetric unarchive interruption and archive retry behavior were outside this selected repair and have no executed regression here. A fresh verifier should review the committed diff and scope behavior before integration.
