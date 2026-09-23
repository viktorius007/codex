STATUS: DONE  
COMMIT: `17c9b68c519f40c94dd3705445f27e76531831b4`  
CHANGED: four existing MCP cancellation methods and `codex-rs/core/tests/suite/mcp_optional_startup_grace.rs`  
BLOCKERS: none  
TARGET: retained `codex-rs/target` approximately 20 GiB; about 23 GiB free after the final build

# Idle MCP startup cancellation repair

An idle interrupt used to cancel an in-flight server startup while leaving the published MCP runtime clean. The next turn retained the cancelled connection and omitted its tools. The repair carries a Boolean result through the existing connection, connection-set, runtime, and session cancellation methods. The session marks the runtime dirty only when the interrupt newly cancels an active, incomplete startup; the next turn uses the existing refresh path. The connection-set loop uses non-short-circuit `|=` so every server is visited. Ready, dormant, and previously cancelled connections leave the runtime clean.

## Causal regression and provenance

The auditor's original regression was kept except for one coordinator-approved fixture alignment: `.with_model_info_override("gpt-5.4", |model| model.supports_search_tool = false)`. The adjacent optional startup tests use the same setting. Without it, the model defers MCP tools into tool search, while `namespace_child_tool` inspects directly advertised namespace tools; the first repair run therefore observed `(2, false)` despite a second initialize. The event sequence and `(2, true)` assertion were unchanged.

- Corrected test on baseline `34d1bd0a110879b6c8d0b7d517a22ceb367aba9b` production: **failed twice** at the causal assertion, observing `(1, false)` versus `(2, true)`; [baseline-corrected-red.log](baseline-corrected-red.log). The replay script saved and temporarily restored only the four owned production files from `HEAD`, then restored the repair with an exit trap. `git status` confirmed all four repair diffs afterward.
- Corrected test on the repair: **1 passed, 4,636 skipped** using `just test -p codex-core -E 'test(interrupted_idle_mcp_startup_reconnects_for_next_turn)'`; [green-corrected.log](codex-rs/green-corrected.log).
- Existing optional startup controls: **4 passed, 4,633 skipped** using `just test -p codex-core -E 'test(optional_mcp_startup_grace_controls_initial_turn_tool_catalog)'`; one test passed on automatic retry after its first attempt hit a five-second startup guard under build load; [control-green.log](codex-rs/control-green.log).
- Existing `codex-mcp` controls for ready clients, dormant cached tools, concurrent catalogs, and pending reuse: **4 passed, 240 skipped** using a four-test `just test -p codex-mcp -E ...` selector; [mcp-controls.log](codex-rs/mcp-controls.log).
- `just fmt` passed. It also reformatted three unrelated Python evidence scripts; those were clean before the run and were restored byte-for-byte from `HEAD`. `git diff --cached --check` passed before commit.

Build commands used the retained target with command-local `RUSTC_WRAPPER=`, `HOST_CC=clang`, `HOST_CXX=clang++`, `CARGO_BUILD_JOBS=32`, and `CARGO_INCREMENTAL=1`. Synthetic local MCP and model servers were used; no live provider was contacted. Scoped lint and full core/workspace suites were deferred by the coordinator's disk allocation. The target is retained for independent review. A temporary baseline-source backup was removed by its exit trap; generated logs and the auditor's uncommitted evidence remain in this worktree for coordinator handoff.
