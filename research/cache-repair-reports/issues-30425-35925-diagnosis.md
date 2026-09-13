# Issues 30425 and 35925: root-cause investigation

Assessed: 2026-09-14, Australia/Sydney. Current source: `b2da63e684` on `local/customizations`; installed source remains `4b0a1a1194` (`0.154.0+local.2`). Historical source inspected: `rust-v0.144.4`.

## Verdict

Neither issue has an established incident-level root cause. The investigation identifies request/history mechanisms and a concrete display limitation, but does not establish which mechanism caused the historical cache misses. Keep both issues unresolved in the disposition ledger.

| Reported symptom | Evidence-backed conclusion |
|---|---|
| #30425: successful repeated requests return zero cached tokens | Timeout is ruled out for the later successful requests. Exact request/identity equivalence and provider cache availability are not established. |
| #35925: 24 mid-loop misses | Rollout-visible append-only history is insufficient to establish complete request-prefix equivalence. Client mutation versus provider cache availability/routing remains unresolved. |
| #35925: six input-token decreases | Consistent with compaction; token counts alone do not prove client history shrinkage or identify a compaction event. Recognized Azure Responses providers normally used remote compaction v2 in the reported version. |
| #35925: counters obscure cached versus uncached usage | Confirmed current presentation limitation: the protocol carries cached input, but `/status` omits its amount and the configurable status line lacks a last-request cache-hit percentage. |

## Issue 30425

[Issue #30425](https://github.com/openai/codex/issues/30425) was open with `updated_at=2026-09-07T00:53:33Z` when checked. Two comments do not establish a cause or repair.

The reporter separately corrected a 180-second streamed-body timeout. It explains the timed-out request, not the later successful zero-cache responses. Reported replay rows 1056/1058/1059 each had 74,371 input tokens and zero cached tokens; other requests achieved substantial reuse. Caching was therefore not universally disabled in the report.

In current `codex-rs/core/src/client.rs:516-528`, an ordinary root session derives its prompt cache key from the session ID. The full request built at `client.rs:1002-1020` also includes instructions, input, tools, reasoning and other settings. Transport identities are separate at `client.rs:1326-1354`. A stable key does not establish equality of those other components. Replaying a dumped body also does not establish identical transport headers.

The issue does not specify the coverage of its local request fingerprint or provide the complete paired request and routing evidence needed to distinguish a changed prefix from provider cache availability. Provider routing, eligibility, eviction or unobserved state are possible explanations if the observed client request really was equivalent; none is individually proven.

Official documentation states that cache reuse depends on the rendered prefix and cache availability on the machine receiving the request; a cache key does not guarantee a hit. This explains why key stability cannot settle attribution, rather than establishing the cause of these incidents. See [OpenAI prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching).

## Issue 35925

[Issue #35925](https://github.com/openai/codex/issues/35925) was open with `updated_at=2026-09-07T22:45:08Z` when checked. The report describes Codex 0.144.4 with Azure.

### Historical compaction route

At tag `rust-v0.144.4`, `codex-rs/model-provider-info/src/lib.rs:414-416` recognizes Azure Responses providers as supporting remote compaction. `RemoteCompactionV2` is stable and enabled by default in `codex-rs/features/src/lib.rs`. The dispatch in `codex-rs/core/src/session/turn.rs:954-1019` selects remote v2, or legacy remote if v2 is disabled. The local path applies to unsupported providers. This route depends on the configured endpoint being recognized as Azure.

Historical `codex-rs/core/src/compact_remote_v2.rs:430-447` constructs replacement history from selected retained user/developer/system messages, truncates those to a budget, and appends provider-produced compaction output. Installing that replacement changes the history prefix and can reduce cache reuse after the first changed position. It does not necessarily eliminate reuse of unchanged initial instructions or tools.

This is an established code mechanism, not proof that it ran at each of the six reported input-token decreases. Those decreases were inferred from provider usage counts; the issue lacks paired serialized requests or corresponding compaction evidence. Do not label all six as proven compactions or silently attribute them to the local 20,000-token summary path.

### Ordinary requests and mid-loop misses

Historical `codex-rs/core/src/client.rs:840-925` copies prompt input and clears internal metadata for non-OpenAI providers. Azure Responses uses `store: true`, so `prepare_response_items_for_request` returns without removing IDs. These ordinary preparation steps do not remove reasoning or conversation items. Current preparation at `client.rs:1025-1034` likewise adjusts metadata/IDs rather than pruning conversation history.

Orphan-output normalization can remove unmatched tool outputs, but the report supplies no evidence tying that repair path to the six decreases. Reasoning/tool filtering in remote compaction processing is not evidence of routine pruning during ordinary requests.

The 24 mid-loop misses were classified from rollout events and usage counters. Those records cannot establish equality of complete serialized tools, instructions, settings and transport identity. No first differing request component has been identified for these incidents; no provider trace establishes a backend cause either.

### Display limitation

Current `codex-rs/protocol/src/protocol.rs:2216-2257` carries separate input/cached-input counts and both last and total usage. `codex-rs/tui/src/token_usage.rs:63-80` can format uncached and cached input separately. However, `/status` at `codex-rs/tui/src/status/card.rs:364-378` subtracts cached input and omits its amount. `codex-rs/tui/src/chatwidget/status_surfaces.rs:734-780` supplies cumulative status-line counts rather than a last-request hit percentage. The data exists; the requested display is incomplete. This conceals cache loss but does not cause it.

## Installed repairs and fresh local evidence

The local compaction tool-parity repair (`3ca29ec4b6`) removes a known mismatch in local compaction requests. It is not proof of a repair for the report's normal Azure remote-compaction path. Accounting repairs change compaction thresholds; diagnostics improve attribution. Neither establishes that these issues' mid-loop misses are fixed.

The coordinator ran:

```sh
python3 scripts/audit_cache_diagnostics.py /Users/viktor/.codex/cache-diagnostics --json --limit 8
```

Observed output: seven files, 46 records, 22 completed attempts, nine eligible warm comparisons, zero cache drops, zero malformed records, zero comparisons with missing evidence. The observed window was Unix milliseconds `1789333362666` through `1789333495235`. This small cohort contains no failure witness and cannot establish a repair or invalidate either historical report. No new provider requests or builds were launched for this investigation.

## Evidence required to close attribution

A failing warm-request pair needs correlated logical component/list fingerprints, prepared wire observation, selected transport identities, provider usage, timing, and compaction events. An input/tool-prefix difference can identify a client candidate; ordinary appended suffixes are expected changes and are not sufficient to explain a miss. A controlled reproduction is then needed to connect the candidate to cache loss. Equal observed client data narrows the investigation but still requires provider evidence to distinguish hidden prefix changes, eligibility, routing and cache availability.

## Investigation and cleanup record

Two fresh-context Sol investigations covered the two issues independently; the #35925 follow-up checked the historical provider route and ordinary input preparation. Codebase-memory coverage checks reported matching metadata and no recorded gaps for relied-on current code; historical tag source was read directly. These are best-effort coverage checks, not proof that the graph contains every relationship.

The coordinator read the existing audit/disposition/research, checked GitHub metadata, ran the offline diagnostic scan, and checked official cache semantics. Production source and installed binaries were unchanged. No build artifacts or temporary report files were created. The final cleanup check found the root worktree only and approximately 40 GiB available; existing diagnostic evidence was preserved.
