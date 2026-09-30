STATUS: DONE
COMMIT: 34d1bd0a110879b6c8d0b7d517a22ceb367aba9b
CHANGED: codex-rs/core/src/stream_events_utils_tests.rs; report.md; findings.json; malformed-tool-search-test.patch; malformed-tool-search-red.txt; AGENTS.md
OPEN: 1 proven defect; 1 compatibility lead without an established oracle
BLOCKERS: none
PROVEN_DEFECTS: 1
REVIEWED: 7 entry points; 11 production files in the bounded request-ordering slice
RED_TESTS: 1 executed / 1 failed

# Tool-result ordering and next-request audit

The malformed-search test compiled and failed on unchanged production. The image probe lacked a justified contract and was removed before compilation. `findings.json` contains one proven finding and its full test patch.

## Slice map and disposition

| Entry point or boundary | Observed path and disposition |
| --- | --- |
| `run_turn` | Records accepted input, builds each sampling input with `clone_history().for_prompt`, and starts the next step only after the prior `run_sampling_request` returns (`session/turn.rs:405,532-554`). |
| `try_run_sampling_request` | Handles `OutputItemDone`, queues direct tool futures in `FuturesOrdered`, and drains them after the stream ends (`session/turn.rs:2636-2740,3099`). Cancellation is checked after draining, preventing another request (`session/turn.rs:3110-3112`). The reported early-follow-up race in #44604 is not established in this current path. |
| `handle_output_item_done` | Records each call before starting its handler. Ordinary successful results enter history during the ordered drain (`stream_events_utils.rs:300-349`; `session/turn.rs:2451-2477`). Malformed client `tool_search` arguments take the `RespondToModel` branch described below (`stream_events_utils.rs:387-411`). |
| `ToolCallRuntime` | Starts execution when each call is received; handles cancellation with a synthetic aborted output; maps ordinary failures to matching tool output types (`tools/parallel.rs:76-281,290-337`). Fatal failures have no output and reach `drain_in_flight`'s error path; history normalization can synthesize one on a later prompt. |
| History normalization | `for_prompt` invokes `ensure_call_outputs_present`, `remove_orphan_outputs`, and media stripping without reordering successful outputs (`context_manager/history.rs:520-534,874-887`; `context_manager/normalize.rs:21-211`). This closes the missing-output path for the ordinary history snapshot, while preserving developer messages between valid paired outputs. |
| Image preparation and recording | Each resized tool image output gains a developer notice immediately after that output (`image_preparation.rs:120-203`; `context/image_resize_notice.rs:38-48`). `record_prepared_conversation_items` appends the pair to history before returning (`session/mod.rs:3547-3600`). This is a compatibility lead pending a provider contract for the exact batch shape. |
| Request composition | `build_responses_request` copies prompt input and adds Responses Lite prefix items when applicable; the normal request path leaves existing relative order intact (`client.rs:987-1055`). No send-time batch validation was found in this bounded path. |

The graph index was checked for all files cited above and recorded no file gaps. Rust call and implementation edges are partial, so source reads were used for the actual ordering and error arms. The graph was a navigation aid, not proof of absence.

## Unproven compatibility lead: image notice splits a batch

With the opt-in `ImageResizeNotice` feature enabled, two resized `view_image` outputs are recorded as `call A, call B, output A, developer notice A, output B, developer notice B`. A proposed probe for adjacent outputs would only assert a desired order, so it was removed from the test diff. [DeepSeek's Responses guide](https://api-docs.deepseek.com/guides/responses_api/) says function calls are merged with the adjacent assistant message, without specifying this exact multi-output layout. DeepSeek project reports [#740](https://github.com/deepseek-ai/awesome-deepseek-integration/issues/740) and [#1588](https://github.com/deepseek-ai/DeepSeek-V3/issues/1588) show rejected developer-message interleaving between a call and its own output, a different shape. [#46193](https://github.com/openai/codex/issues/46193) describes the batch shape but remains reporter evidence. The feature defaults off (`codex-rs/features/src/lib.rs:1541-1544`). This lead needs an authoritative rule or an independent, allowed provider reproduction for the exact shape before finding classification.

## Proven finding: malformed tool-search call records wrong output

`ToolRouter::build_tool_call` returns `RespondToModel` when client `tool_search` arguments fail to deserialize (`tools/router.rs:349-362`). The `handle_output_item_done` error branch records a `FunctionCallOutput` with an empty call ID (`stream_events_utils.rs:387-411`), rather than a `ToolSearchOutput` paired to the original search call. On a later prompt, normalization synthesizes an empty tool-search output and treats the empty-ID function output as orphan (`context_manager/normalize.rs:21-211`). In debug builds the orphan path calls `error_or_panic`; in release it logs (`util.rs:81-87`). The test `malformed_tool_search_call_records_matching_search_output` asserts exact raw history after the error branch. Test patch: `codex-rs/core/src/stream_events_utils_tests.rs` in this worktree.

The executed assertion observed `[tool_search_call/search-invalid, function_call_output/empty ID]`; its independent expected pair was `[tool_search_call/search-invalid, tool_search_output/search-invalid]`. The fixture passes an integer `query` to the accepted `ToolSearchCall` response item, forcing the parser's `RespondToModel` arm. `ToolCallRuntime::failure_response` already uses a matching `ToolSearchOutput` for other search failures (`tools/parallel.rs:289-300`). The output mismatch is therefore causal and specific, rather than a generic failure assertion. User consequence is failed tool discovery with no parse error reaching the model; the debug normalization panic is inferred from the cited source and was not separately executed. Severity: moderate. The exact [test patch](malformed-tool-search-test.patch) and [raw red log](malformed-tool-search-red.txt) are preserved.

## Dismissed historical mechanism and remaining limits

The specific claim in #44604 that the next sampling request starts before the final batch result is appended does not match the current `drain_in_flight` await and next-step assembly order. Both call and output recording are awaited before `try_run_sampling_request` returns. This dismissal would reopen if another path starts the next request without this drain or records output outside it. I have not inferred behavior for response parsing/retry or persisted rollout reconstruction, which are separately owned slices. I have not scanned private session data.

## Validation and cleanup

- `UV_CACHE_DIR=/private/tmp/codex-harness-audit/ordering/uv-cache just fmt` passed in `codex-rs`; three unrelated Python files touched by its workspace formatter were restored to their unchanged baseline.
- `git diff --check` passed. Production files remain unchanged.
- Allocated command from `codex-rs`: `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++ CARGO_BUILD_JOBS=32 CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/private/tmp/codex-harness-audit/ordering/codex-rs/target just test -p codex-core -E 'test(malformed_tool_search_call_records_matching_search_output)'`.
- Runner verdict: 1 test run, 0 passed, 1 failed, 4,636 skipped; the runner retried the same failing assertion and exited 100. This was an assertion failure after successful compilation, not a compiler failure.
- Initial free disk: 77 GiB; 15 GiB after concurrent builds. Open-file soft limit: 4096. Isolated target measured 11 GiB and is retained exclusively for the coordinator's repair handoff.
- The Python formatter cache was removed; no other agent-created temporary files remain in this worktree. The two evidence files and target are deliberately preserved.

Minimal repair consideration: preserve the original search call ID and emit the tool-search output type in the parser-error arm, reusing the existing failure-output convention. The coordinator chooses any fix; production source is unchanged in this audit.
