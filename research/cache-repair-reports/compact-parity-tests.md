STATUS: DONE
COMMIT: none
CHANGED: 1 file
OPEN: 1
BLOCKERS: none; the coordinator reserved compilation and the targeted red/green run for its shared integration target

---

## Role result

Added one focused integration regression in `/private/tmp/codex-cache-compact-parity/codex-rs/core/tests/suite/compact.rs`. It drives a real mid-turn local auto-compaction through `CodexThread`, freezes a non-default dynamic tool in the thread catalog, captures the ordinary sampling request and the immediately following local compaction request, proves the fixture really advertised the dynamic tool, identifies the compaction request by its summarization input, and compares the serialized `tools` plus `parallel_tool_calls` components exactly.

No production file was changed. The intentionally red test tree was not committed, per the brief. The sole worktree change is the test file (95 inserted lines, including imports).

## Finding

```json
{
  "id": "wt-compact-tool-parity-001",
  "kind": "finding",
  "run": {
    "commit": "3af0d7dfbb5b0a2f67866b97b7bbbc4ddc1b4d18",
    "model": "sol",
    "mode": "test-writer",
    "slice": "local-compaction-request-tool-parity"
  },
  "category": "gap",
  "domain": "tests",
  "status": "open",
  "candidate": "A mid-turn local compaction created from an active sampling step must serialize the same advertised tools and parallel-tool-call setting as that sampling request.",
  "gap_evidence": {
    "greps": [
      "git grep -n -e 'parallel_tool_calls' -e 'DynamicToolSpec' -e 'dynamic_tools' HEAD -- codex-rs/core/tests/suite/compact.rs || true  # no output",
      "git grep -n -e 'mid_turn_continuation_compaction' -e 'auto_compact_runs_after_token_limit_hit' -e 'model-visible tools payload' -e 'parallel_tool_calls' HEAD -- codex-rs/core/tests/suite/compact.rs codex-rs/core/tests/suite/compact_remote.rs"
    ],
    "nearest_test": "codex-rs/core/tests/suite/compact.rs:4351 (baseline line 4256), snapshot_request_shape_mid_turn_continuation_compaction",
    "difference": "The nearest local test reaches the real mid-turn compaction route and checks compaction input/history shape, but has no non-default dynamic or MCP tool and never asserts tools or parallel_tool_calls. Remote tests assert those fields on a different implementation route. No baseline local compact test subsumes the new parity assertions."
  },
  "strategy": "Integration level. A unit test of Prompt construction would stub away the defect-producing seams: thread catalog capture, StepContext reuse, mid-turn local compaction routing, and outbound request serialization. The wiremock SSE server replaces only network I/O while preserving all of those seams.",
  "changes": [
    {
      "path": "codex-rs/core/tests/suite/compact.rs",
      "lines": "4258-4348",
      "point": "Added mid_turn_local_compaction_reuses_the_active_step_tool_catalog and the imports it needs."
    }
  ],
  "summary": "The regression catches local compaction dropping an active step's visible tool catalog or changing its parallel-tool-call setting.",
  "evidence": [
    {
      "path": "codex-rs/core/tests/suite/compact.rs",
      "lines": "4292-4309",
      "point": "The real thread is created with visible dynamic tool compaction_catalog_probe, so empty/default request parity cannot satisfy the fixture premise."
    },
    {
      "path": "codex-rs/core/tests/suite/compact.rs",
      "lines": "4311-4336",
      "point": "A 96-token completion in a 100-token window causes the tool-output continuation to enter local compaction; request count and the summarization prompt identify the route."
    },
    {
      "path": "codex-rs/core/tests/suite/compact.rs",
      "lines": "4337-4345",
      "point": "The oracle deep-compares the captured tools and parallel_tool_calls values from adjacent sampling and compaction requests."
    }
  ],
  "proof": {
    "oracle_reasoning": "The assertion pins the observable request contract directly. Its expected side is the independently captured ordinary sampling request from the same frozen step, not a compact helper or descriptor used to build the compact request. Exact tuple equality rejects a missing/reordered/schema-changed tools array and a changed parallel_tool_calls value. The separate dynamic-tool presence assertion prevents both requests being empty from passing vacuously. The three-request count and summarization-prompt assertion prove the compared second body is the real mid-turn local compaction request.",
    "discriminator": "Fixture: dynamic tool compaction_catalog_probe with defer_loading=false; model returns an unsupported function call plus total_tokens=96 in a 100-token context, causing the same turn to enter local compaction. On baseline 3af0d7dfbb, codex-rs/core/src/compact.rs:285-289 builds local Prompt with Default, while codex-rs/core/src/session/turn.rs:1393-1397 puts model_visible_specs and parallel_tool_calls=true in ordinary sampling. The targeted run is therefore expected to fail with compact tools=[]/parallel=false versus a sampling catalog containing compaction_catalog_probe/parallel=true. Execution is pending the coordinator-owned shared integration target; this role was explicitly told not to compile.",
    "validation": "Formatting passed with `env UV_CACHE_DIR=/private/tmp/codex-cache-compact-parity-uv-cache just fmt`; `git diff --check` passed. Targeted compilation/execution pending coordinator: from codex-rs run `just test -p codex-core -E 'test(mid_turn_local_compaction_reuses_the_active_step_tool_catalog)'` using the coordinator's isolated absolute CARGO_TARGET_DIR, CARGO_BUILD_JOBS=32, and CARGO_INCREMENTAL=1. Re-run the same command after the builder fix for green evidence."
  },
  "value_bar": {
    "regression": "Catches the small edit/current omission that leaves the local compact Prompt's tool fields at defaults instead of carrying the active frozen step catalog.",
    "audience": "Customers and operators relying on stable model requests across automatic compaction.",
    "failure_cost": "An unnecessary cache-prefix change and a summarizer that cannot interpret already-recorded tool calls from their advertised schemas, increasing latency/cost and risking repeated or confused tool work."
  },
  "vacuity_checks": {
    "wrong_property": "Pass: the failing equality directly names request-component parity.",
    "fixture_coincidence": "Pass: the unique visible dynamic tool is asserted present; the suite exercises many compaction limits, and this case's 96/100 boundary value is load-bearing for reaching mid-turn compaction.",
    "under_constrained": "Pass: exact JSON equality is required for both related fields; non-empty/success alone cannot pass.",
    "unreachable_premise": "Pass by construction pending execution: the response sequence returns a tool call and over-limit usage, and the test requires exactly sampling/compact/continuation requests plus the compact prompt in request two.",
    "self_referential_oracle": "Pass: the ordinary request is produced by the regular sampling builder and captured independently; the compact request follows a separate production builder and route."
  },
  "elicitation": {
    "authority": "/private/tmp/codex-cache-run/compact-parity-brief.md: local compaction retains every unchanged advertised tool-related request component from the active sampling step; regression seam requires exact tools plus parallel_tool_calls equality on a real route.",
    "actors": "A model response emits a tool call and legal usage data; a client-supplied visible dynamic tool makes the catalog non-default; the token threshold triggers compaction.",
    "invariants": "Within one frozen step, sampling and local compaction advertise the identical tool catalog and parallelism setting; intentional compact input/output-schema differences remain outside the assertion.",
    "boundary": "96 tokens is one token above the 95% mid-turn continuation threshold for a 100-token context.",
    "never": "Local compaction must never silently erase the frozen advertised catalog or flip parallel tool calls while summarizing that step's history.",
    "implicit_contracts": "Step atomicity and request ordering are explicit in the captured three-request sequence.",
    "happy_sad_parity": "No refusal/error contract is targeted. The happy route pins exact values and ordering; a wrong compact request is the discriminator."
  },
  "verdict": "self-check: REDO(executed discriminator and restored-green run are pending the coordinator's scheduled integration compilation; no test-code change is currently indicated)"
}
```

## Commands and exact outcomes

- `codebase-memory-mcp cli search_graph ... --name-pattern '^run_inline_auto_compact_task$' --file-pattern 'codex-rs/core/src/compact.rs'`: one match, `compact.rs:119-149`, connected to `run_auto_compact` and `run_compact_task_inner`.
- `codebase-memory-mcp cli trace_path ...run_inline_auto_compact_task --direction inbound --depth 2 --include-tests true --include-evidence true`: four exact inbound production callers across two hops (`run_auto_compact`, `run_pre_sampling_compact`, `maybe_run_previous_model_inline_compact`, `run_turn`).
- `codebase-memory-mcp cli check_index_coverage` for `compact.rs`, `session/turn.rs`, local `compact.rs` tests, and `compact_remote.rs`: all `no_recorded_issue`, `metadata_match`, best-effort caveat. The graph project was reindexed against root HEAD `fdca0be4f4`, while this assigned worktree is `3af0d7dfbb`; consequently graph results were used only for route orientation and every relied-on source/test range was read directly in the assigned worktree.
- First `just fmt`: failed only because sandbox access to `/Users/viktor/.cache/uv` was denied. `env UV_CACHE_DIR=/private/tmp/codex-cache-compact-parity-uv-cache just fmt`: exit 0.
- `git diff --check`: exit 0.
- Cargo/test command: not run, by coordinator instruction. No local Cargo target was created.
- Cleanup: removed the agent-created 46 MiB `/private/tmp/codex-cache-compact-parity-uv-cache`; assigned worktree is 136 MiB; final observed disk free was 49 GiB. Unknown/user files were untouched.

## Remaining gap

The coordinator must record the expected baseline red and the post-fix green using the targeted command above. Until then the test is formatted and structurally inspected, but its discriminator is explicitly unproven by execution.
