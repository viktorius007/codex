STATUS: DONE
COMMIT: 34d1bd0a110879b6c8d0b7d517a22ceb367aba9b
CHANGED: scan_sessions.py, report.md, findings.json
OPEN: 0
BLOCKERS: none
PROVEN_DEFECTS: 0
REVIEWED: 0 source entry points; 38 stable session files and one bounded runtime-log query
RED_TESTS: 0 executed / 0 failed

# Sanitized local-session evidence

## Scope and method

This is runtime testimony to route the six harness audits, not a production source audit or a causal red-test proof. The source data under `~/.codex` was read only. No prompt, command text, tool arguments, tool output, token, configuration secret, or raw transcript was copied into this report. `python3 scan_sessions.py` reproduces the content-free counts and SHA-256 manifests below. It scans only session files in UTC date folders `2026/09/22` and `2026/09/23` whose last modification was at or before the local.7 installation time, `2026-09-23T12:15:38Z`. That selection excludes active or later-appended files. The script also queries only three relevant WARN/ERROR log targets from `2026-09-22T00:00:00Z` through that cutoff using SQLite read-only mode.

The fixed selection contained **38 files, 77,811,249 bytes, and 15,327 JSONL records**. Event timestamps span `2026-09-22T12:57:30.785Z` to `2026-09-23T10:37:00.954Z`. The sorted file-ID/size/content-hash manifest SHA-256 is `985b5008aea7543ccdb04b64ef4233d41908ba91d65a9f771c926177a5ba27fb`. There were 39 `session_meta` records: 18 reported `0.155.1+local.5`, 21 reported `0.156.1+local.6`. No selected record reported local.7. A session metadata version describes its recorded start; it does not prove the binary version of a later resumed process.

At a separate metadata-only check at `2026-09-23T12:47:50Z`, 16 new session files that began after local.7 installation (`12:24:55.848Z`–`12:40:24.445Z`) all still recorded `0.156.1+local.6`. They were not part of the fixed event sample. Installation time alone therefore cannot identify the version of the already-running harness processes.

## Event map and audit leads

| Seam | Observed bounded evidence | Disposition |
| --- | --- | --- |
| MCP startup cancellation | Four completed MCP calls failed, all on `codebase-memory-mcp`: one `index_status` error and three `trace_path` function-not-found errors. Six `rmcp::transport::worker` ERROR rows occur within four seconds at `2026-09-23T00:24:25Z`–`00:24:28Z`, in one process, with two Responses WebSocket errors nearby. No sampled turn ties these to cancelled MCP startup or permanent tool loss. | Historical lead for the MCP owner only; no proven recovery failure. |
| Tool-call/result ordering | All **1,792** persisted tool call IDs in the 38 files have exactly one matching persisted output item: zero missing calls, orphan outputs, or duplicate outputs. This is a persistence-level pairing check, not an outbound request-body check. | No incident found in this sample; the ordering owner still needs its controlled send-boundary test. |
| Replay and durable storage | Eight `compacted` records were found in four files; one 29,043,838-byte file had five. The records alone do not establish reconstructed-history correctness, memory growth, or loss of the original archive. | Large-history lead only; no observed failure. |
| Subagent residency | Ten persisted `spawn_agent` calls; 10 `SubAgentActivity.started`, 10 `completed`, and 59 `interacted` items. The records do not expose capacity ownership or blocked eviction. | No capacity claim from this sample. |
| Terminal response parsing and retry | Twelve `codex_core::responses_retry` WARN rows at `core/src/responses_retry.rs:156` (7) and `:158` (5), through `2026-09-23T12:15:30Z`. Sanitized body markers: `websocket` 8, `closed` 5, `timeout` 2; `usage` and `incomplete` 0. These counts do not establish whether retries were correct or wasteful. | Route source locations and times to the retry owner; no evidence for the incomplete-usage report. |
| Transcript preservation | No archive replacement or corruption outcome is represented by the sampled JSONL event types. | No applicability evidence; the storage owner needs a disposable-history probe. |

The log slice contains 20 rows total: 12 retry WARN, six RMCP transport ERROR, and two Responses WebSocket ERROR. Its ID/timestamp/target/source-line/body-hash manifest SHA-256 is `239a68e8c3db709bb2f3e4780348ba72ab77ac7e56638596bf096d740affba82`. This binds the summarized rows without preserving their private bodies.

## Dismissed apparent anomalies

- The only `turn_aborted` event (file ID hash `68c47d9a6531229e`, `2026-09-23T03:23:52.133Z`, local.5) occurred before any tool call in that turn. A subsequent turn began at `03:24:23.419Z` and completed at `03:24:33.372Z`. Its 48 task starts versus 47 task completions are explained by that abort, so it is not evidence of a lost completion or MCP startup failure.
- Three `tool_catalog_changed` events (`2026-09-23T00:58:38.765Z`, `03:46:53.689Z`, `07:46:11.696Z`) report changed/added entries and zero removed entries. No sampled event shows a tool disappearing permanently. Catalog change by itself is not a cache or authority defect.
- The four failed MCP items above have completed result records. Three are explicit function-not-found errors; none has a timeout or connection marker in its structured error field. They are not evidence of a stalled MCP call.

## Limits and next use

This was a deliberately narrow 22–23 September preinstall sample plus three log targets, not a search of the 4,354+ older session files, all SQLite targets, live provider traffic, or a local.7 runtime. Records omit the outbound request bodies and some runtime ownership state needed to settle the six source questions. The file filter is reproducible against unchanged local records; if a selected file is later appended or rewritten, its manifest hash will differ and the old count must not be reused. No Rust code, tests, binaries, or original diagnostic records were changed. `findings.json` is empty because no significant defect has an executed causal red test.
