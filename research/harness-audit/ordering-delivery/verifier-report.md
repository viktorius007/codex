PASS
COMMIT: e43de7a0fedcfa25283da39e6ac043fe32e74f7d
COMMANDS: `just test -p codex-core --lib -E 'test(malformed_tool_search_call_records_matching_search_output)'` with the allocated target, `RUSTC_WRAPPER=`, clang hosts, 32 jobs, incremental enabled: 1 passed, 2597 skipped; `git show --check HEAD`: exit 0.

| Criterion | Proof |
| --- | --- |
| Malformed client search with a call ID gets a matching typed output and no orphan function output | `core/src/stream_events_utils_tests.rs:312-349` builds a wrong-type `query`, checks exact ordered history; focused test passed. The original red replay in `malformed-tool-search-red.txt` failed on `function_call_output` with an empty ID. |
| Existing search failure convention is retained | `core/src/stream_events_utils.rs:393-402` records an empty completed client `ToolSearchOutput` with the original ID, matching `core/src/tools/parallel.rs:289-297`. |
| Other response failures retain their path | The non-search fallback at `core/src/stream_events_utils.rs:403-409` is unchanged by the commit; the router at `core/src/tools/router.rs:332-384` emits this pre-dispatch parse error only for client search calls with IDs. |

FINDINGS: None in the committed diff.
CHECKED: Both changed files in full, the router's parse branch, existing search failure output, history conversion, diff shape, and the exact red/green regression. No new helper, type, indirection, or comment was added; test uses a fixture-owned malformed argument and asserts the externally recorded pair. Codebase-memory coverage reported no recorded file gaps, but Rust call edges are semantically partial; direct source resolved the relevant path.
LIMITS: This is a scoped verification. The full core/workspace suites, lint, and mutation sweep were outside the build allocation. The changed handler already exceeded the 100-line size target before this surgical 12-line addition. The test checks type and ID, while status/execution/empty tools are verified from source and the existing failure convention. No live provider was used.
