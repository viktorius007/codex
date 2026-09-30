# Malformed tool-search response repair

STATUS: DONE
COMMIT: e43de7a0fe
CHANGED: `codex-rs/core/src/stream_events_utils.rs`, `codex-rs/core/src/stream_events_utils_tests.rs`
BLOCKERS: none

## Repair

When a client `tool_search` call has malformed arguments, `ToolRouter::build_tool_call` returns `RespondToModel`. The response handler now records a completed `ToolSearchOutput` with the original call ID and an empty tool list, matching `ToolCallRuntime::failure_response` for other search failures. The existing response for other `RespondToModel` errors is unchanged. No helper or public API was added.

The auditor's 41-line regression remains unchanged. Its baseline run compiled and failed with `tool_search_call/search-invalid` followed by `function_call_output/empty ID`. The expected observable history is the search call followed by `tool_search_output/search-invalid`. See `malformed-tool-search-red.txt` and `coordinator-red.log` for the baseline evidence.

## Gates

- Focused command: `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/private/tmp/codex-harness-audit/ordering/codex-rs/target just test -p codex-core -E 'test(malformed_tool_search_call_records_matching_search_output)'` — 1 run, 1 passed, 4,636 skipped.
- Affected command: same environment and `just test -p codex-core -E 'test(stream_events_utils::tests::)'` — 14 run, 14 passed, 4,623 skipped.
- `UV_CACHE_DIR=/private/tmp/codex-harness-audit/ordering/uv-cache just fmt` passed. The formatter touched three unrelated Python files, which were restored to their original contents.
- `git diff --cached --check` passed before commit. The commit contains only the production source and the unchanged 41-line regression.

The full core and workspace suites and scoped lint were not run; the coordinator allocated only focused tests. Free disk fell to about 12 GiB after test compilation. The functions edited were already over the coding standard's 100-line target; the requested single-site repair added 12 net lines without an extra helper or API. Independent verification is still required before integration.

The root checkout and other agents' worktrees were left untouched. The 11 GiB local `codex-rs/target` remains available for independent verification.
