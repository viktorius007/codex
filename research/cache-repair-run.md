# Cache repair run

Started: 2026-09-14 Australia/Sydney.
Goal: a trustworthy tested and installed local binary, private cache diagnostics, and direct evidence-driven cache repairs.

## Current delivery state

Candidate: `0.154.0+local.2` in `/private/tmp/codex-cache-integration`, branch `work/cache-integration`. Root coordination branch remains `local/customizations` at `dc8f07458aad1db2d7fe3940643b23732cba6876`. Production changes have not yet been committed or installed. The installed binary is still `0.154.0+local.1`.

The full workspace suite passed all17,641 tests (47 skipped; one passed retry and one leaky label), using `just test --cargo-profile dev-small --test-threads 4 --features codex-v8-poc/sandbox`. Its authoritative log is `/private/tmp/codex-cache-run/final-full-workspace-2.log`. Scoped Clippy completed successfully; root corrected the remaining size/argument warnings without suppressions. The retry exposed a test synchronization weakness: it now asserts the exact sampled turn's successful completion event instead of polling transient status. Focused verification of those final corrections is running in `final-mechanism-and-lint-regressions.log`.

The detailed chronology, corrections, and recovery instructions remain in [cache-repair-recovery.md](cache-repair-recovery.md). Historical issue discovery remains in [prompt-cache-prefix-stability.md](prompt-cache-prefix-stability.md); refresh it incrementally, never through blanket rediscovery.

## Authorization and boundaries

- Preserve diagnostic evidence indefinitely: no automatic deletion, disk quota, or age expiry. Individual records and memory are bounded.
- Full tests, experiments, and validated installation are authorized. Preserve the previous matching binary pair and record exact installation provenance.
- Choose direct Pareto fixes from local evidence; GitHub proposals are clues. Do not expand into speculative backend or broad retained-history redesign.
- Use fresh-context Sol agents for load-bearing work. The coordinator owns small corrections, integration, and cleanup.
- Fingerprint differences are observed changes, not proven causes; equality does not prove a backend fault.

## Implemented candidate and regression evidence

| Area | Implemented behavior | Verification evidence in `/private/tmp/codex-cache-run/` |
|---|---|---|
| Diagnostic archive | Installation-keyed fingerprints, complete logical/wire observation, private append-only run files, bounded records/manifests, paired outcomes and usage; no automatic retention deletion | `records-api-lifecycle-green.log`:191 passed; `records-verifier.md` |
| Runtime diagnostics | Ordinary requests, warmup, local/remote compaction, retries, reused WebSockets, and detached memory requests | `runtime-diagnostics-verifier-followup.md` and `runtime-diagnostics-memory-assessment.md`; lifecycle gates; memory regression in final318-test gate |
| Offline analysis | Bounded read-only parsing, first observed differences, truthful missing/ambiguous evidence, normalized warm-comparison rates, independent before/after windows | `final-python-scanners.log`:21 passed; `analysis-verifier-followup.md` |
| Usage accounting | Accepted usage stays paired with its covered history boundary; usage-less tails, replacement, and resume are accounted for | `usage-verifier.md`; focused history/reconstruction gates |
| Compaction | Local compaction reuses active sampling tools; pending turn input contributes to thresholds without entering the compacted history | `compaction-verifier.md`; `pending-projection-fullsuite-review.md` |
| Fresh-context startup | Shared startup context precedes role-specific instructions; private parent history stays out of child input | `fresh-prefix-verifier.md`; `local2-wake-fixtures-3.log`:17 passed |
| Plugin skills | Stable semantic locators resolve through the active snapshot with containment protection | `locator-verifier-followup.md`; full skills gate176 passed |
| Subagent completion | Bounded completion text wakes an idle parent; active waits avoid duplicate delivery | Focused wake/security/residency gates; final318-test gate |
| MCP authority | Deterministic full tool order; sampled ready calls retain exact authority; lazy cached calls verify connection/configuration/schema before using live execution metadata | `tool-stability-final-audit.md`; `lazy-presentation-green.log`:322 passed |
| Local version and fixtures | SemVer metadata preserved; host-sensitive tests retain their behavioral oracles | SemVer crate gates; `tui-final-gate.log`:4314 passed |

## Explicit carry-forward boundaries

The original research queue contains proposals beyond these direct repairs. A hard-capped retained raw tail plus structured compaction checkpoint is not implemented. Later world-state, skill, and prompt-hook changes are not included in pending-turn projection. These remain separate compaction-continuity work; do not describe the whole original Stage3 as complete.

Generic MCP `tools/list_changed` notifications remain unimplemented. This candidate protects sampled execution authority and stable ordering. Publishing a genuinely changed schema should change the prefix; notification support is separate capability freshness work. Forked-history redesign and provider-controlled cache affinity remain lower-priority research, not fixes claimed by this build.

No provider-side cache improvement has been measured. The historical rollout scan cannot supply a complete request-fingerprint pre-fix window. Record installation start/end to millisecond precision and select new-process diagnostic runs for post-install analysis; old running processes remain on their old binary.

## Remaining delivery gates

1. Complete focused verification of the final causal synchronization and lint corrections; the full workspace gate is green.
2. Run scoped `just fix` and repository `just fmt`; inspect changes. Bazel lock refresh has already succeeded with no generated diff.
3. Regenerate and review the14 prepared commit groups. The temporary-index check already proves their combined blobs exactly reconstruct all90 candidate files. Preparation script: `/private/tmp/codex-cache-run/prepare-commit-patches.py`. Integrate small coherent commits into `local/customizations`.
4. Rebuild the matching `codex` and `codex-code-mode-host` pair from the clean final commit. Run `/private/tmp/codex-cache-run/install-validated.py` with exact commit/version and the full-green test log. Preserve the previous pair and provenance.
5. Run the installed CLI against a local mock provider, verifying its roundtrip and actual private diagnostic records. Preserve that diagnostic evidence.
6. Save final reports, commit IDs, tests, binary hashes, installation timestamps, and cohort guidance. Remove only owned build/temp artifacts and finished worktrees/obsolete branches; preserve unknown files and diagnostic evidence. Check disk and Git state before closing.

## Environment

MacBook Pro M3 Max,16 cores,48GB RAM. Cargo uses32 jobs, incremental development compilation, and one isolated integration target; agents run no concurrent Cargo builds. Open-file limit is4096 with unlimited hard limit. The verified Codex V8 archive/bindings are supplied by `run-check.py`; the full-suite V8 feature aligns the test marker with the linked sandbox build.

CMake, GStreamer, and Bazelisk were installed as required test/build prerequisites. Python tools use UV Python3.12 with the task-local cache. Disk headroom reached approximately11GiB during the full build. Do not remove its active target; clean owned incremental/build artifacts after Cargo completes and after preserving the final binaries/evidence.
