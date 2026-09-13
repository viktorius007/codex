# Cache diagnostics guide

Cache diagnostics provide a private, local record of the request features that can affect prompt-cache reuse. They help locate changes between warm requests and measure provider-reported cache loss without storing the underlying prompts or responses.

## What is recorded

When collection opens successfully for a Codex session, the instrumented build writes an append-only run file under:

```text
~/.codex/cache-diagnostics/YYYY/MM/DD/run-<random-id>.jsonl
```

The archive contains paired request and outcome records. Request records are labeled as one of four kinds:

- `turn`: an ordinary model turn or one of its higher-level retries.
- `warmup`: a WebSocket `generate=false` request used to establish a cache prefix.
- `compaction`: a request that compacts prior conversation state.
- `memory`: an internal memory-generation Responses request.

The collector records keyed fingerprints and byte counts for fixed logical request components, retained input and tool entries, exact prepared wire bodies, and identity-bearing transport fields. It also records fixed transport categories such as endpoint, compression, connection reuse, and incremental WebSocket mode. The bodies and identity values themselves are not stored. Outcomes contain terminal status and provider token usage when available. Missing observations are recorded explicitly.

The files do not contain prompt text, model output, tool arguments or output, file contents, credentials, full headers, or raw transport identity values. Fingerprints use an installation-local secret stored as `~/.codex/cache-diagnostics/.fingerprint-key`; archive files and the key are created as private files. Treat the archive as sensitive local evidence even though its request contents are fingerprinted.

Individual JSONL records are limited to 256 KiB. Input and tool manifests retain up to 1,024 entries each, with counts and fingerprints for omitted entries; the exact prepared-body fingerprint still covers the complete observed body.

Evidence is preserved indefinitely. There is no age limit, disk cap, rotation, or automatic deletion. The analyzer creates a temporary SQLite index and deletes only that scratch data when the run ends; it opens the evidence files read-only.

## Run the analyzer

From the repository root, print a private text summary with:

```sh
uv run --python 3.12 python scripts/audit_cache_diagnostics.py \
  ~/.codex/cache-diagnostics
```

For structured output and up to 100 detailed incidents:

```sh
uv run --python 3.12 python scripts/audit_cache_diagnostics.py \
  ~/.codex/cache-diagnostics --json --limit 100
```

`--limit` bounds only the detailed incident list. File counts, comparison counts, rates, and token totals still cover the complete scan. Pass a dated directory or one `run-*.jsonl` file instead of the archive root for a narrower cohort.

Analyzer reports use fixed labels, field names, counts, timestamps, and token totals. They do not print source paths, run or attempt IDs, fingerprint values, stored error strings, or unknown JSON values.

The defaults consider requests at most 30 minutes apart. A comparison is eligible only when the earlier request reported at least 1,024 cached tokens. A cache drop requires at least 1,024 estimated lost cached tokens and a new cached count no greater than half the expected amount. Use `--help` to see the corresponding threshold flags.

## Compare before and after a fix

Record the fixed binary's installation time as Unix milliseconds, then split one diagnostic cohort at that exact time:

```sh
uv run --python 3.12 python scripts/audit_cache_diagnostics.py \
  ~/.codex/cache-diagnostics \
  --split-at-unix-ms <INSTALL_TIME_UNIX_MS> --json --limit 100
```

Replacing the installed executable does not upgrade already running processes. For a version comparison, start fresh sessions with the new binary and select their run files; a timestamp split alone cannot identify the executable version of an older process. Installation provenance records millisecond timestamps before and after the binary pointer switch. Use the end of that interval for the post-install boundary and exclude requests within the interval.

The analyzer keeps the two windows independent: a request before the split cannot become the predecessor of a request after it. Each window reports:

- `eligible_warm_comparisons`: the denominator of comparable warm request pairs.
- `cache_drops`: pairs that cross the configured loss thresholds.
- `cache_drop_rate`: drops divided by eligible comparisons.
- `expected_cached_tokens`: the sum of `min(previous cached input, current input)`.
- `observed_cached_tokens`: the provider-reported cached input on the later requests.
- `cached_token_retention_rate`: observed divided by expected cached tokens.

A rate is `null` when its denominator is zero. Compare rates and token retention for well-defined cohorts; raw incident totals mainly reflect how many requests each window contains. The windows include every eligible request kind and lineage type, while each detailed incident identifies its `request_kind` and `comparison_kind`.

## Interpret findings carefully

`request_difference_candidate` means the collector observed a changed logical, wire, or transport fingerprint and reports the first fixed field or retained list position it can localize. It does not prove that the changed field caused the provider's cache result.

`unexplained_observed_equality` means the recorded fields were equal. It does not prove a backend fault: unobserved provider state, eviction, routing, timing, or incomplete local evidence may still explain the result. Missing wire or transport evidence is marked partial rather than treated as equality proof. Provider token counts are accounting evidence and do not directly prove physical cache reuse.

A retry-lineage comparison shares a turn fingerprint and has a nonzero higher-level retry ordinal. A same-thread comparison shares a thread fingerprint. A related-thread comparison shares only a cache-affinity fingerprint; it can identify a plausible cache-sharing predecessor but cannot prove ancestry, cross-process order, or use of the same physical provider cache. Equal-time candidates from different run files are left order-ambiguous.

For HTTP, the collector observes one prepared endpoint request before the generic transport retry loop. Lower transport retries clone and share that prepared observation instead of producing separate diagnostic attempts. `retryOrdinal` describes the higher-level Codex attempt sequence, not every low-level HTTP send.

## Keep the historical baseline separate

The [historical rollout baseline](prompt-cache-prefix-stability.md#local-rollout-baseline) was produced from ordinary session rollouts before these complete request fingerprints existed. The original summary covered 3,762 rollout files and 109,522 usage samples. The supplementary [saved scan and provenance](cache-baselines/2026-09-14-pre-repair/provenance.json) covered 3,768 files and 109,658 usage samples, with 769 candidates and 36,497,026 estimated lost cached tokens. These are preserved scan results, not immutable copies of every source file. The archive was live, and no exact file-byte cohort was captured; completion time is the time completion was observed.

The new archive cannot reconstruct complete diagnostic records for those older requests. A before/after split is meaningful only when diagnostics captured enough requests on both sides; installing the collector and a fix together cannot create a diagnostic pre-fix window. Do not rescan the growing historical rollout tree and compare its raw totals with a new diagnostic window. Preserve the original baseline as its own cohort, and evaluate post-install behavior from sessions created by the instrumented binary using normalized rates and token retention.
