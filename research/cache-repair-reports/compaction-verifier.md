PASS
Verified base SHA: `3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18`; target is its uncommitted three-file diff on `work/issue-37305`.
Commands/evidence: `git diff --check HEAD -- <three scoped files>` exited 0; diff is 115 insertions/8 deletions (20 insertions/8 deletions production, 95 test insertions).
Targeted red: `initial-regressions-red.log` ran `mid_turn_local_compaction_reuses_the_active_step_tool_catalog` twice and failed with compact `tools=[]`, `parallel_tool_calls=false` versus the sampling step's non-empty catalog and `true`.
Targeted green: `compact-parity-green.log` summary is `1 test run: 1 passed, 4187 skipped`; SHA-256 checks confirm its integration checkout contains byte-identical copies of all three reviewed files.
Broader evidence: `core-prerequisites-complete.log` summary is `4163 tests run: 4156 passed, 7 failed, 27 skipped`; the parity regression passed, and all seven named failures are outside the reviewed files and behavior.
Criteria → proof: active automatic step reuse → `core/src/session/turn.rs:1322-1328` and `core/src/compact.rs:120-150`; manual fresh-step semantics → `core/src/compact.rs:153-180`; request parity with compaction schema preserved → `core/src/compact.rs:287-301`; regression protection → `core/tests/suite/compact.rs:4258-4348` plus the red/green logs.
Findings: none.

Cleared: `core/src/compact.rs` — automatic local compaction carries the exact frozen `Arc<StepContext>` into request construction; manual local compaction captures one fresh step at its standalone boundary, matching both existing remote manual paths; tools, parallel calls, and cyber access now match the ordinary/remote request pattern while `output_schema: None` and strict compaction behavior remain explicit. No input, summary, history-replacement, hook, analytics, retry, protocol, or remote-path behavior was changed.

Cleared: `core/src/session/turn.rs` — the sole changed callsite clones the already-active sampling `StepContext`; mid-turn, pre-turn, and previous-model automatic paths all converge through this call without recapturing or synthesizing a tool catalog. Graph inbound traces found one direct caller for local `run_inline_auto_compact_task`, three callers for `run_auto_compact`, and one caller for standalone `run_compact_task`.

Cleared: `core/tests/suite/compact.rs` — the integration test exercises the public thread route with a unique visible dynamic tool, verifies that premise, identifies the second request as local compaction, then deep-compares the serialized `tools` array and `parallel_tool_calls`. The independent ordinary sampling request is the oracle; the dynamic-tool presence assertion prevents empty/empty vacuity. The recorded baseline failure and restored green satisfy the discriminator. The test protects customer-visible cache reuse, latency/cost, and correct interpretation of tool calls already present in compacted input.

Shape/size: no new production function, helper, type, trait, error variant, public API, dependency, or compatibility layer; production delta is 28 changed lines and total diff is well below the 800-line cap. The change reuses `StepContext`, `ToolRouter::model_visible_specs`, and the established remote-compaction `Prompt` field pattern. New comments state lifecycle rationale rather than restating code; `git diff --check` is clean.

Graph/source verification: `search_graph` located the local compaction functions, `run_auto_compact`, `capture_step_context`, and ordinary `build_prompt`; inbound `trace_path` results were verified against source. `check_index_coverage` returned `no_recorded_issue`/`metadata_match` for every reviewed and supporting file relied on. The graph is best-effort and indexed from the root checkout, so every scoped file was also read in full from `/private/tmp/codex-cache-compact-parity` and changed ranges were checked directly.

Fixes made: none; this was a read-only verification.

Limitations: no command was built or run by this verifier, as explicitly required. The supplied integration logs are accepted because their reviewed files are byte-identical to this worktree. The broader core run is not wholly green: seven unrelated failures remain, including the separately owned completion-cap case. No mutation sweep or commit gate was supplied; the target remains an uncommitted review diff.
