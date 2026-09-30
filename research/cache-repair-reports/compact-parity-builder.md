STATUS: DONE
COMMIT: none
CHANGED: 2 production files (plus 1 preserved frozen test file)
OPEN: 1
BLOCKERS: none; coordinator-owned formatting/diff check and targeted green gate remain pending because this role was told not to build, and unified exec process creation is temporarily failing machine-wide with OS error 24

---

Implemented the minimal local-compaction request-parity fix in the assigned worktree and left it uncommitted for the coordinator's green gate.

Production changes:

- `codex-rs/core/src/session/turn.rs`: the local automatic-compaction branch passes the active `Arc<StepContext>` instead of reducing it to `Arc<TurnContext>`, preserving the exact frozen tool router used by adjacent sampling.
- `codex-rs/core/src/compact.rs`: the common local-compaction path carries `Arc<StepContext>` through request construction. The compact `Prompt` uses that step's `model_visible_specs`, keeps `parallel_tool_calls: true`, and carries `cyber_access_program`; compaction-only `output_schema: None` and `output_schema_strict: true` remain explicit.
- `codex-rs/core/src/compact.rs`: standalone/manual local compaction captures one fresh step context at its request boundary before invoking the same builder, matching established manual remote-compaction behavior.

Remote compaction, history replacement, summarization input/suffix behavior, tests, protocol types, configuration, and sandbox-environment handling were unchanged. The authorized frozen edit in `codex-rs/core/tests/suite/compact.rs` was preserved.

Automatic correctness: `session::turn::run_auto_compact` already owns the active sampling step and now clones that same `Arc<StepContext>` into `compact::run_inline_auto_compact_task`; no tool-plan recapture occurs. Manual correctness: `compact::run_compact_task`, like both remote manual entry points, calls `Session::capture_step_context` once at the standalone boundary. Request correctness: `run_compact_task_inner_impl` reads the frozen tool specs, parallel setting, and cyber program while retaining the same input/base instructions and explicit compaction output-schema behavior.

Graph reuse and coverage evidence:

- Project `Users-viktor-Projects-github-codex` was ready at indexed root SHA `fdca0be4f4dedc8bf299f66b569f2cb9f409e858`. The worktree is `3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18`; graph results were bounded orientation and every relied-on range was read directly in the assigned worktree before editing.
- Exact symbol search for local compaction, `StepContext`, `TurnContext`, and `Prompt` returned 12 relevant nodes and the two direct callers of local `run_compact_task_inner`.
- Qualified inbound depth-2 traces returned exactly four callers for `compact::run_compact_task_inner` (two direct entry points plus manual/auto orchestration transitively) and exactly four for `compact::run_inline_auto_compact_task` (one direct plus three turn paths transitively).
- Reuse search located ordinary `session::turn::build_prompt` and both remote compact builders. `build_prompt` cannot be used unchanged because it carries the ordinary turn's final output schema. The patch reuses the existing `StepContext`, `ToolRouter::model_visible_specs`, and remote-compaction field pattern. It adds no helper, trait, type, validator, error variant, or public API.
- `check_index_coverage` returned `no_recorded_issue` and `metadata_match` for both changed files plus `tasks/compact.rs`, both remote attempt files, `session/step_context.rs`, and the frozen test. This remains best-effort because the indexed SHA differs.

Commands and outcomes:

- Brief, frozen-test report, mechanism-scout section 1, applicable skills, governing source, and initial worktree state: read successfully. Initial state was branch `work/issue-37305` at `3af0d7dfbb`, with only the authorized frozen test modified and 48 GiB free.
- Production `apply_patch`: succeeded.
- Cargo/build/test commands: not run, per assignment; coordinator owns baseline red and post-fix green.
- Post-edit `git diff --check`, diff inspection, and `just fmt` could not start. Unified exec returned `CreateProcess ... Too many open files (os error 24)` on repeated attempts, including after a 60-second wait and with login disabled. No formatter/build/test process began.
- Cleanup: this role created no build target, formatter cache, or temporary repository artifact. Unknown/user files were untouched.

Remaining gate: coordinator should run formatting/diff validation and `just test -p codex-core -E 'test(mid_turn_local_compaction_reuses_the_active_step_tool_catalog)'` with its isolated target, 32 build jobs, and incremental compilation. The green gate should precede commit and independent verification of both automatic and manual paths.
