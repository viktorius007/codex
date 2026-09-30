STATUS: DONE
COMMIT: 34d1bd0a110879b6c8d0b7d517a22ceb367aba9b
CHANGED: report.md; findings.json; AGENTS.md
OPEN: 0
BLOCKERS: none
PROVEN_DEFECTS: 0
REVIEWED: 5 entry points; 10 source/test files and relevant ranges
RED_TESTS: 0 executed / 0 failed

# Saved-session read and replay audit

## Slice map and coverage

Production entry points and terminal outcomes:

1. `LocalThreadStore::load_latest_model_context` routes paginated files through reverse lineage scanning and legacy files through full history loading (`codex-rs/thread-store/src/local/model_context.rs:37-79`). `ModelContextScan::push` accumulates items until a usable checkpoint and context baseline, or retains a full replay when it reaches the beginning (`codex-rs/rollout/src/model_context.rs:50-80,82-160`).
2. `RolloutRecorder::load_rollout_items` decodes every nonempty line into a `Vec`, skips malformed records with a parse-error count, and returns that vector (`codex-rs/rollout/src/recorder.rs:1069-1132`). `get_rollout_history` places it in `Arc<ResumedHistory>` (`:1134-1148`).
3. Core `Session::record_initial_history` passes the resumed vector to `apply_rollout_reconstruction` (`codex-rs/core/src/session/mod.rs:1564-1610,1677-1804`). `reconstruct_history_from_rollout` scans backward for the surviving replacement checkpoint and metadata, replays the selected suffix forward, applies rollback markers, and restores world state (`codex-rs/core/src/session/rollout_reconstruction.rs:76-520`).
4. `LocalAgentControl::load_agent_model_context` explicitly requests full stored history for legacy subagents and targeted model context for paginated ones (`codex-rs/core/src/agent/control/spawn.rs:153-177`).
5. `ContextManager::drop_last_n_user_turns` truncates reconstructed response history and retained context at instruction boundaries (`codex-rs/core/src/context_manager/history.rs:670-765`); `RetainedContext::restore` applies record and family size limits to checkpoint material (`codex-rs/history/src/retained_context.rs:141-172,408-462`).

Actors: local saved JSONL, its first session metadata, model and host rollout items, compaction checkpoints, rollback markers, file corruption, incomplete turns, and user/session resume or subagent loading. Terminal outcomes: reconstructed model history and context metadata; full-read fallback; read error; parsed-line skip. The reverse scan is only used for paginated histories. Local legacy history remains the default (`codex-rs/protocol/src/protocol.rs:779-785`, `codex-rs/thread-store/src/store.rs:96-98`).

The codebase-memory index generation was 2026-09-23T12:39:48Z. Exact-name graph search located reconstruction and rollout-load symbols; `trace_path` returned no Rust call edges and marked semantics partial. `get_code_snippet` confirmed both. `check_index_coverage` reported no recorded file gaps for the six core paths and six additional read/retention paths queried, but Rust binding/expanded-call/impl coverage is partial. Source reads established the call chain independently. This is a bounded slice review, not proof of absence outside those files.

## Legacy loading and resource limit

The local store's legacy `load_latest_model_context` branch returns the entire file even after a modern replacement checkpoint (`codex-rs/thread-store/src/local/model_context.rs:69-74`). Core legacy subagent loading also requests all history (`codex-rs/core/src/agent/control/spawn.rs:153-177`), and ordinary legacy resume reconstructs from an eagerly loaded `Arc<Vec<RolloutItem>>` (`codex-rs/rollout/src/recorder.rs:1069-1148`; `codex-rs/core/src/session/mod.rs:1564-1578`). Obsolete pre-checkpoint records therefore increase transient load memory and startup work with session age. This is a source-derived scaling characteristic, not a proven defect: the trait expressly permits full-history fallback (`codex-rs/thread-store/src/store.rs:177-188`), and no independent memory/time threshold or executed production-consumer measurement establishes a material failure. A proposed count test was removed because it would have asserted a new bounded-return contract rather than the current behavior's user consequence. A future targeted benchmark should measure peak allocation and end-to-end resume for two synthetic files with identical recovered state but different obsolete prefixes before ranking a repair.

## Dismissed and covered risks

- The reverse reconstruction does not blindly select the newest checkpoint: it skips turn segments covered by rollback markers before choosing a base (`rollout_reconstruction.rs:76-134,176-306`). Existing focused tests assert exact history and metadata for completed, incomplete, inter-agent, and oversized rollback cases (`rollout_reconstruction_tests.rs:756-1285`). A new serialized shape lacking those boundaries would reopen this question.
- Retained authorization and sender evidence are capped on live record and again at checkpoint restoration (`retained_context.rs:174-205,319-400,408-462`). The cap does not bound the original rollout file or the transient decoded checkpoint; those remain scaling limits. A checkpoint that bypasses the restore path would reopen this dismissal.
- Paginated reverse loading has a bounded cutoff only with replacement history, window number, and surviving context baseline. The source deliberately scans to the beginning for rollback markers or legacy checkpoint shapes (`model_context.rs:25-48,82-101`). This is a conservative fallback, not shown wrong by the reviewed fixtures. A production paginated file carrying those old shapes at large scale would reopen its performance impact.

## Commands and limits

- `just fmt` first failed because sandbox access to `~/.cache/uv` was denied. `UV_CACHE_DIR=/private/tmp/codex-harness-audit/replay/.uv-cache just fmt` passed. Its unrelated Python formatting changes were restored from HEAD; `git diff --check` passed.
- No Cargo command ran because the source review did not produce a viable causal-red regression. No live provider or private session data was used.
- No production or test source changed. `findings.json` is empty.
