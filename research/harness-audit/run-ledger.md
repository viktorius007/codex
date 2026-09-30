# Harness audit run ledger

Baseline 34d1bd0a11. All investigators: fresh context, gpt-6-sol, high effort. User authorized six targeted audits and local-session evidence investigation. Previous shortlist changes in root AGENTS.md and research/next-audit-targets.md preserved.

| Agent | Slice | Worktree/report | State |
| --- | --- | --- | --- |
| harness_mcp | Interrupted MCP initialization/recovery | /private/tmp/codex-harness-audit/mcp/report.md | P2 repaired; independent PASS; integrated be9baf4da9 |
| harness_ordering | Tool batch result/request boundary | ordering-delivery/report.md | P2 repaired; independent PASS; integrated8efe0d1ebd |
| harness_replay | Session replay, rollback and retained-memory scaling | replay-review.md | Done; no proven defect; proposed resource oracle rejected |
| harness_residency | Subagent residency/capacity eviction | residency-review.md | P2 repaired; independent PASS; integrated c68e284bbc |
| harness_retry | Terminal response parse/retry classification | retry-review.md | P2 repaired; independent PASS; integrated ba8f64bca1 |
| harness_storage | Durable transcript write/compaction integrity | storage-review.md | P2 repaired; independent PASS; integrated bdf0e0845b and 851fe17cf1 |
| harness_sessions | Sanitized local session incident evidence | session-evidence.md | Done; no proven defect; scanner replay matched |

Initial resources: approximately 74 GiB free; inherited shell open-file limit4096. Reading proceeds concurrently; Cargo allocations require coordinator scheduling, isolated targets. No new live provider tests authorized for Sol. Production unchanged during investigation.

Build allocations: MCP/core and retry/API active first; storage/thread-store next; residency and ordering/core queued. Replay candidate requires resource-contract review before build. Idle coordinator did not wake on allocation messages; agents now end their turn on blocking requests so terminal completion wakes parent.

Measured targets before second core allocation: MCP11GiB, storage4.5GiB, free55GiB. Residency/core allocated; ordering remains queued. Retry/API reviewer target removed after evidence preservation; coordinator replay uses retry-fix worktree.

Retry quota classification admitted P2: coordinator regression replay compiled and failed twice with exact Stream instead of QuotaExceeded. Fresh Sol(high) builder harness_retry_fix owns retry-fix worktree, forwards only response.failed errors immediately, preserving other event behavior.

Retry repair independent PASS, integrated ba8f64bca1. Ordering repair independent PASS, integrated8efe0d1ebd. Targets3.9GiB and13GiB removed only after evidence preservation. Idle residency incremental cache6.7GiB removed; rebuilt as needed. Storage verifier requested focused guard tests, test author active; no production defect found in repair. MCP test fixture corrected after independent assessment to disable deferred tool-search presentation, corrected baseline replay failed; repair green pending.

Storage final independent PASS at130df36be5; production integratedbdf0e0845b and test protection851fe17cf1. Evidence archived, owned6.1GiB target removed. MCP corrected-fixture baseline red and repair green at17c9b68c5; independent verifier active.

Final two independent reviews PASS: MCP integrated be9baf4da9; residency integrated c68e284bbc. All seven audits complete, five P2 repairs integrated, two slices without proven defects. Final combined validation pending. MCP and residency owned targets removed after evidence preservation.

Delivery complete: local.8 installed 2026-09-24T00:19:49.306230+10:00 with local.7 rollback. Source 09a54a34c2. Formatting and scoped lint passed; workspace bins built; affected-crate tests 5305 passed, 3 known baseline failures, 27 skipped. Full workspace did not execute tests because compilation exhausted disk. No additional production fixes were introduced. All eight worktrees and 13 obsolete branches removed after evidence preservation.

Final cleanup: 55 GiB free after removing run-created incremental entries and 27.61 GiB of logical outputs from the failed workspace attempt. Combined logs and cleanup manifests archived and hash-verified before temporary originals were removed. Existing native build archives and diagnostic evidence retained.
