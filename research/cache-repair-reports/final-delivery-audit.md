# Cache repair delivery audit

Delivered: 2026-09-14, Australia/Sydney. Source commit: `4b0a1a1194d1393680c9dca1d75cb9b8ecb17de7`. Installed version: `0.154.0+local.2`.

This audit covers the user's advanced diagnostics and locally justified Pareto repairs, not every speculative proposal in the historical issue queue. The user authorized priority changes, full tests, installation, and cleanup. No provider improvement is claimed.

| Requirement | Current evidence | Verdict |
|---|---|---|
| Privacy-safe complete observed request identity, isolated from model history | Private HMAC archive crate; logical, exact prepared wire, and identity observations; API and lifecycle tests; independent records/runtime reviews; installed CLI sidecar smoke | Pass |
| No disk cap, age limit, rotation, or automatic evidence deletion | Append-only run writer; private persistent key; no retention-removal path; guide and independent review; all210 run evidence files preserved with hashes | Pass |
| Bounded individual records and collection memory | 256KiB record cap, 1024 retained inputs/tools and omitted-tail hashing; oversized-manifest and privacy tests; independent bounded-memory review | Pass |
| Actionable, honest comparison | Offline analyzer reports first observed component/list differences, missing evidence and ambiguity; normalized eligible-warm-pair rates;19 analyzer tests plus2 historical scanner tests passed | Pass |
| Direct root-cause repairs with regression protection | Frozen compaction tools, accepted-usage history boundary, pending-input thresholds, fresh-child shared-prefix order, stable plugin locators, sampled MCP authority and deterministic tool order, bounded completion/wake behavior; focused discriminators and independent reports preserved | Pass |
| Assert mechanisms rather than timing | Exact sampled turn completion replaces transient-status polling; both variants passed in final376-test gate; durable AGENTS.md instruction committed | Pass |
| Full tests and lint/format | Full suite17641/17641 passed,47 skipped,1 successful retry,1 leaky label. Retry synchronization corrected; final376/376 focused tests passed. Final scoped Clippy no warnings; formatter exit0 | Pass, qualifications retained |
| Clean source, rebase-friendly integration |14 mechanism-grouped source commits rebased and fast-forwarded onto local/customizations; exact source bytes unchanged by rebase; final clean-commit build exit0. Larger cohesive diagnostic/schema groups are documented in preserved staging plan | Pass |
| Trustworthy installed artifact and rollback | Installer verified clean commit, matching version and pair, system-only dynamic libraries, checksums and full-green log; previous pair preserved; installed localhost request/sidecar smoke and analyzer passed; installed and backup hashes rechecked | Pass |
| Time and provenance for future analysis | Installation cutover Unix1789332102165, Sydney2026-09-14T06:41:42.165+10:00; full hashes in installation-local2.json. Historical rollout baseline remains separate, not an immutable file-byte cohort or complete diagnostic pre-window | Pass as measurement infrastructure; provider improvement unmeasured |
| Durable coordination and cleanup | Run ledger, issue ledger, reviews, source commits, provenance,210 preserved evidence files and source archive. All task worktrees/branches/build caches removed; root worktree only,41GiB free; unknown artifacts preserved | Pass |

The full suite preceded final mechanical lint changes and causal fixture synchronization. Those changes were independently reviewed and followed by the376-test affected gate; final formatting was not used as a reason to rerun the entire suite. The complete-suite leaky label was on an in-memory TUI cursor test; its cause was not established, and the result is retained rather than represented as a diagnosed product leak.

The broader proposals for a structured retained compaction tail/checkpoint, later world-state/skill/prompt-hook projection, generic MCP tools/list_changed support, fork-history redesign, and backend affinity remain explicitly outside this direct repair delivery. They are retained in the research ledger, not silently marked implemented. Actual schema/content changes can legitimately change a prefix. Fingerprint equality cannot prove a backend fault, and differences cannot alone prove causality.

Permanent private evidence: `/Users/viktor/.codex/cache-repair-evidence/2026-09-14-local2/`. Raw logs retain their original temporary paths for provenance; their preserved files now reside in its `run/` subdirectory. The installed source commit remains fixed even as later documentation commits update the branch.
