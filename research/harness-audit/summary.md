# Harness audit and repair

Audit baseline: `34d1bd0a110879b6c8d0b7d517a22ceb367aba9b`, stable `rust-v0.156.1` plus local.7 patches. Seven bounded investigations used fresh-context Sol agents at high effort. Repair and independent verification roles also used Sol at high effort. No live provider tests were run.

## Proven defects

All five findings are P2: meaningful failures under specific error, cancellation, or crash conditions. No P0 or P1 defect was established. Each was admitted only after an executed mechanism test failed against baseline production; each repair passed that test and independent review.

| Area | Failure mechanism | Surgical repair | Integrated commits |
| --- | --- | --- | --- |
| Response retry | A transport error after `response.failed` overwrote the terminal quota classification with a retryable stream error. | Return the decoded terminal failure immediately. | `ba8f64bca1` |
| Tool-search result | Malformed client tool-search arguments produced a function output with an empty ID instead of the matching tool-search output. | Preserve the response type and call ID on the error path. | `8efe0d1ebd` |
| Transcript archive | A crash after moving transcript files but before SQLite update left the selected history path missing. | Resolve only the exact archived counterpart, checking selected rollout and thread identity, without changing database state. | `bdf0e0845b`, `851fe17cf1` |
| MCP startup | Idle interruption cancelled pending startup without arranging a refresh for the next turn. | Mark the existing refresh path dirty only when pending startup was newly cancelled. | `be9baf4da9` |
| Subagent capacity | Eviction removed a loaded worker from capacity accounting before awaited work; cancellation left it uncounted. | Keep the worker counted until removal and serialize eviction attempts. | `c68e284bbc` |

## Evidence and clean results

| Investigation | Evidence or disposition |
| --- | --- |
| Retry | Baseline returned `Stream` instead of `QuotaExceeded`; repaired API suite passed 194 tests and independent focused replay passed. |
| Ordering | Baseline recorded the wrong response object and ID; repaired focused regression and 14 related tests passed; independent replay passed. The reported early-next-request theory did not match the current awaited drain path. |
| Storage | Simulated the actual durable crash state, reopened storage, and demonstrated missing history. Repaired thread-store suite passed 257 tests. Additional tests protect selected history, owner identity, archive exclusion, and unchanged SQLite metadata; final independent review passed. |
| MCP | Corrected baseline returned `(1, false)` for initialize attempts and next-request tool availability; repaired result `(2, true)`. Independent regression and four MCP controls passed. The first test fixture omitted disabling deferred tool search; its corrected baseline was replayed before admission of the final repair. |
| Residency | Baseline admitted another worker and lost accounting `(true, 0)` while eviction was paused; repaired result `(false, 1)`. Independent residency suite passed all three tests. |
| Replay | No proven defect. A proposed history-size bound lacked a supported resource contract and was rejected. Legacy full-history scaling remains a possible performance investigation, not an established bug. |
| Local sessions | Read-only sanitized scan of 38 files found 1,792 call IDs and 1,792 outputs, without missing or duplicate persisted outputs. Coordinator replay matched the source-manifest hash. Recorded retry and MCP errors did not establish the five mechanisms occurring locally. |

## Limits

The local-session sample mostly predates local.7 installation. Session metadata inspected after installation still reported local.6, so replacing installed binaries did not restart the existing harness. The scan does not prove current provider cache reuse or exclude failures outside the sampled records. Source review and focused tests establish the repaired mechanisms; they do not establish a globally defect-free harness.

An incidental shutdown-error propagation lead remains unproven: a shutdown error may be followed by completion and runtime removal. No regression test established data loss, so this audit did not change that path.

## Delivery

Installed **0.156.1+local.8** at 2026-09-24T00:19:49.306230+10:00, from source commit `09a54a34c25f631734f5b7e7664a87676814e0e8`. local.7 is retained for rollback. Installed version, both help commands, resolved paths, architecture, system-library links, and SHA-256 checks passed. The running harness was not restarted.

Formatting and scoped lint passed. All workspace binaries built. Combined affected-crate tests: **5,305 passed (one flaky), three failed, 27 skipped**. The three failures are the same previously documented baseline failures in skill approval and terminal approval. All new causal regressions passed. Full-workspace compilation exhausted disk before its tests executed; no full-workspace pass is claimed. No live provider cache-hit measurement was run.

Eight audit worktrees and 13 obsolete branches were removed after evidence archival and patch-equivalence checks. Run-created incremental files and outputs from the failed workspace attempt were reclaimed. Native build archives were preserved during final cleanup because selective deletion of those archives had previously left stale Cargo build metadata.

Detailed role reports, raw evidence, cleanup manifests, and installation provenance are retained alongside this summary.
