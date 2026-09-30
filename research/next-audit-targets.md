# Next audit targets

Research date: 2026-09-23. Local baseline: `34d1bd0a11`, stable `rust-v0.156.1`, installed local.7. One fresh-context `gpt-6-sol` agent at `high` effort researched GitHub; the coordinator compared the results with the local audit and repair ledger. GitHub states below were checked at 12:27 UTC.

These are priorities for investigation, not proven local defects or assigned defect severities. No new regression tests or production changes were made. Open issue status does not establish that a report still reproduces in this release. Reproduction on the current checkout is the first gate; retain only significant defects backed by causal red tests.

## Recommended order

| Priority | Target | Why investigate | Proposed causal check | Local overlap |
| --- | --- | --- | --- | --- |
| 1 | Interrupted MCP startup recovery | Reported permanent loss of a session's tools after startup cancellation. Relevant to repeated interrupts and tool-heavy sessions. | Cancel a controlled startup during tool listing, restore server readiness, and assert subsequent tool availability and execution authority. | Sampled authority protects advertised calls; it does not by itself establish recovery from cancelled initialization. |
| 2 | Tool-call/result ordering before the next model request | Reported requests containing an unmatched tool call can wedge a thread. The cited report used a third-party Responses provider; relevance to the normal provider remains to be established. | Hold one result in a batched call behind an explicit barrier. Inspect actual outgoing request pairing after release and on cancellation. | The local terminal notification repair prevents duplicate completion delivery; batch request assembly is a different boundary. |
| 3 | Session replay correctness and bounded resource use | Reports describe huge repeated compaction records exhausting memory and rolled-back output reappearing after resume. Especially relevant to long-running sessions. | Use small synthetic histories with repeated checkpoints, rollback, and a known suffix. Assert reconstructed history exactly and measure retained/decoded data growth; do not rely on elapsed time or destroy source history. | Accepted usage tracking and compaction tool parity do not establish bounded history loading or all rollback combinations. |
| 4 | Subagent capacity release and eviction | Reports describe completed agents retaining capacity through queued messages, or spawn stalling during eviction. | Fill a small controlled capacity, complete a child, queue a message, then assert exact slot ownership, spawn result, and preserved message delivery. | Completion wake and duplicate suppression concern parent notifications, not residency accounting. |
| 5 | Terminal response parsing and retry classification | Reports describe another model request after a terminal response whose usage is incomplete or whose status is incomplete. Potential wasted requests and inconsistent usage. | Feed controlled terminal stream events, count exact requests, and assert retained outputs and usage according to the provider contract. | Local accepted-usage bookkeeping is downstream of parsing and retry classification. |
| 6 | Durable transcript preservation during compaction | A reported large transcript rewrite lost history. High impact but lower-confidence applicability: older bundled CLI, single incident, no deliberate reproduction. | On disposable history, exercise compaction/storage transitions and assert the original archive and required metadata survive. | Distinct from request-prefix preservation. Do not assume the desktop incident reproduces in this CLI. |

Start with priorities 1–2. Priorities 3–4 are the next bounded wave. Priorities 5–6 require an applicability check before spending substantial test/build effort. This is not a recommendation for a repository-wide audit.

## Current GitHub leads

All rows were OPEN at the check time. Update times support incremental refresh without rereading unchanged research.

| Issue | Kind | Last updated (UTC) | Mechanism / disposition |
| --- | --- | --- | --- |
| [#46399](https://github.com/openai/codex/issues/46399) | Bug report | 2026-09-18T19:12:22Z | Cancelled MCP startup remains unavailable. Priority 1. |
| [#44604](https://github.com/openai/codex/issues/44604) | Bug report | 2026-09-17T11:46:43Z | Batched tool output missing at send boundary. Priority 2. |
| [#46193](https://github.com/openai/codex/issues/46193) | Bug report | 2026-09-17T11:53:18Z | Related call/output location report. Group with priority 2. |
| [#30932](https://github.com/openai/codex/issues/30932) | Bug report | 2026-09-22T00:13:48Z | Large rollout resume memory growth. Priority 3. |
| [#32851](https://github.com/openai/codex/issues/32851) | Bug report | 2026-07-13T18:32:16Z | Rollback plus compaction replay correctness. Priority 3, separate assertion. |
| [#33777](https://github.com/openai/codex/issues/33777) | Bug report | 2026-07-22T02:14:53Z | Terminal-resident eviction stalls spawn. Priority 4. |
| [#32353](https://github.com/openai/codex/issues/32353) | Bug report | 2026-09-23T07:36:21Z | Queue-only mail pins a completed resident. Priority 4. |
| [#37141](https://github.com/openai/codex/issues/37141) | Bug report | 2026-08-06T12:55:45Z | Completed-response usage parse error causes retry. Priority 5. |
| [#38831](https://github.com/openai/codex/issues/38831) | Bug report | 2026-08-17T06:15:42Z | Incomplete response triggers retry/fallback. Priority 5, separate contract check. |
| [#44363](https://github.com/openai/codex/issues/44363) | Bug report | 2026-09-23T02:17:35Z | Reported transcript rewrite/history loss. Priority 6. |
| [#36721](https://github.com/openai/codex/issues/36721) | Feature request | 2026-08-07T15:17:12Z | Structured checkpoint/lossless tail. Design proposal, not an additional proven bug. |
| [#37121](https://github.com/openai/codex/issues/37121) | Bug report | 2026-08-05T16:59:43Z | Tool-state loss after truncation and compaction; overlaps already-recorded continuity research. Defer broader semantic-summary work. |

## Deduplication and exclusions

- Do not reopen the exact locally repaired mechanisms merely because their GitHub issues remain open: [#37305](https://github.com/openai/codex/issues/37305) compaction tool parity and [#37299](https://github.com/openai/codex/issues/37299) idle-parent completion wake.
- Stable skill locators, frozen model/role descriptions, initial shared-prefix ordering, request diagnostics, disabled-plugin reader availability, and terminal stdin duplicate completion have already received targeted review or fixes. No fresh blanket review is proposed.
- The historical retiring-turn completion-loss theory is not carried forward: the previous async audit found session-level queues and restoration after cancelled starts.
- Generic MCP catalog refresh/list-change work in the earlier ledger remains distinct from the specific startup-cancellation lead. The `067cf7cdf9` catalog patch records catalog-change diagnostics; it does not itself implement catalog refresh.
- Provider cache affinity and retention remain reproduction-gated. A provider cache miss alone does not prove a local prefix defect.
- The three pre-existing core validation failures are documented in the previous audit. They do not prove an unauthorized read or a new production regression; resolve their test/policy expectations separately if restoring the broader test gate is selected.

## Local evidence and limits

Coordinator source scout: `codex-rs/rollout/src/recorder.rs:1069` reads records sequentially and retains decoded items in a `Vec`. This supports investigating retained-history scaling; it does not prove a reachable memory failure. Graph generation `2026-09-23T12:16:54Z` reported matching file metadata and no recorded parse gap for this file, with partial Rust semantic coverage. The exact source was read directly. No complete call-chain or exhaustive coverage claim is made.

Deduplication sources: [previous audit](custom-patch-audit.md), [issue disposition](cache-repair-issue-disposition.md), [prefix review](custom-patch-audit/prefix-review.md), [catalog review](custom-patch-audit/catalog-review.md), [validation diagnosis](custom-patch-audit/validation-diagnosis.md), and local commit descriptions.

Research coverage: GitHub open-issue searches for `cancellation retry tool call`, `resume session corrupt compaction`, `subagent reconnect completion`, `MCP reconnect tools list changed`, `performance memory subagent`, and `tool output missing retry`, all scoped to `repo:openai/codex is:issue is:open`; metadata checked for 22 candidates, selected new bodies/comments read. Web searches additionally covered compaction/resume tool loss and cancellation/retry. Search was bounded, not exhaustive. Firecrawl's documented developer command was unavailable and its search failed at the proxy; GitHub API and web supplied the results.

Cleanup: no new worktrees, build artifacts, or temporary research files were created. Only this research report and its root index entry were added; installed binaries are unchanged.
