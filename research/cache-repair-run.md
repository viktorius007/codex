# Cache repair run

Started: 2026-09-14 Australia/Sydney
Base: ea55207054 on local/customizations; version 0.154.0+local.1.
Goal: trustworthy tested and installed binary with privacy-safe diagnostics and evidence-driven cache mechanism repairs.

## Authorization and boundaries

- No automatic diagnostic deletion, disk cap, or age expiry; bounded records and memory.
- Full tests and validated binary installation authorized; preserve prior binary and installation provenance.
- Adapt priority from evidence within ledger stages 1–5. Beyond-Pareto work requires user approval.
- Fresh-context Sol agents for load-bearing work; Terra only simple located edits. Coordinator owns integration, corrections, cleanup.
- Fingerprint differences indicate observed client changes, not proven causes; equality does not prove backend fault.
- Existing issue research is durable; refresh incrementally.

## Progress

| Task | Agent | Report | State |
|---|---|---|---|
| Diagnostic architecture | diagnostics_design | /private/tmp/codex-cache-run/diagnostics-design.md | running |
| Compaction and tool inventory | mechanism_scout | /private/tmp/codex-cache-run/mechanism-scout.md | DONE: five mechanisms; report pending durable copy |
| Prefix and fresh-child inventory | prefix_scout | /private/tmp/codex-cache-run/prefix-scout.md | running |
| Build and installation readiness | build_readiness | /private/tmp/codex-cache-run/build-readiness.md | running |
| Policy and durable coordination record | root | this file | in progress |

## Next checkpoints

1. Select smallest diagnostic implementation and test seams from architecture evidence.
2. Delegate regression authoring, implementation, independent verification in fresh stages.
3. Triage stages 2–5 into non-overlapping mechanism fixes with direct regression witnesses.
4. Integrate small commits, run appropriate crate and full-suite checks, preservation regression, formatting/lint.
5. Bump local version, build and install validated binary; record exact timestamp/hash and rollback binary.
6. Preserve reports/evidence in research; remove completed worktrees and agent build artifacts.

## Environment

Initial clean checkout; 57 GiB free. Cargo jobs 32, isolated targets, development incremental enabled; no compiler cache. No active Git hooks (sample files only). Existing binary ~/.local/bin/codex is 298263304 bytes, dated Sep 13 21:15. Do not replace a running executable in place; use atomic replacement after validation.

## Validation and installation

Pending.

## Coordination notes

- User is asleep; resolve routine blockers autonomously.
- Policy integrated as d691087d0d. origin/main refreshed to 36f0dbe796 and main fast-forwarded; stable customization base unchanged.
- macOS system python3 is too old for scanner union annotations. Use UV_CACHE_DIR=/private/tmp/codex-cache-run/uv-cache uv run --python 3.12 python.
- Additional live pre-repair JSON scan started 2026-09-14T01:02:15+10:00; output research/cache-baselines/2026-09-14-pre-repair/scan.json. This is a new live cohort, not reconstruction of the original baseline.

## Fix dispatches

| Slice | Stage/agent | Worktree/branch | Report | State |
|---|---|---|---|---|
| Local compaction tool parity #37305 | test-writer compact_parity_tests | /private/tmp/codex-cache-compact-parity; work/issue-37305 | /private/tmp/codex-cache-run/compact-parity-tests.md | running |
| Numeric usage boundary #35935 | test-writer usage_boundary_tests | /private/tmp/codex-cache-usage-boundary; work/issue-35935 | /private/tmp/codex-cache-run/usage-boundary-tests.md | running |

Source inventory: direct local-compaction empty-tools mismatch; numeric usage boundary inferred from newest model item can advance without new usage; pending user/context not counted pre-turn; local operational tail dropped; MCP notifications ignored. Step catalog atomicity and deterministic producers already implemented, so avoid replacement machinery. Tail retention is indirect amplification and should remain lower priority than direct cache differences.

Baseline scanner tests: 2026-09-14, Python3.12 unittest targeted invocation exited 0; /private/tmp/codex-cache-run/scanner-tests.log. Baseline integration commit 3af0d7dfbb.

## Architecture checkpoint

Initial diagnostic design is in research/cache-repair-reports/diagnostics-design-initial.md. Sol designer is refining a smaller interface that moves cross-session comparisons to offline analysis, avoiding runtime pointer/LRU/locking machinery while preserving immutable redacted evidence, exact logical/wire observations, and truthful incompleteness flags.

Prefix report: stable semantic plugin locators require real resolver support; successful completion text lacks the existing 1000-token cap; V2 queued terminal completion does not trigger an idle parent. V1 duplicate delivery is lower priority than active V2. Fresh parent/child complete-prefix reuse is unproven, not a demonstrated routing defect; keep distinct thread IDs.

Additional test-writers running: skill_locator_tests (/private/tmp/codex-cache-skill-locators, work/issue-25609; report skill-locator-tests.md), parent_completion_tests (/private/tmp/codex-cache-parent-completion, work/issue-37299; report parent-completion-tests.md). They own tests only; no separate Rust builds.

Coordinator integration worktree: /private/tmp/codex-cache-integration, work/cache-integration. All heavy builds use only its isolated codex-rs/target with jobs32/incremental enabled. Baseline preservation test started, exec session44147, log /private/tmp/codex-cache-run/core-build-baseline.log. Initial just test --no-run rejected because nextest --no-fail-fast conflicts; corrected to the real exec-completion regression. Never poll exec completion.

Build readiness: preserve/install matching codex and codex-code-mode-host pair; full-suite footprint estimate 15–40 GiB, not measured. Shared target between agents prohibited; coordinator-only integration target reused across combined checks. Bazel absent; install if new diagnostic crate/dependencies require lock refresh. Version bump before final validation. Follow user test/fix/fmt ordering over contrary readiness-procedure suggestion.

## Latest continuation

See research/cache-repair-recovery.md, Restarted continuation (current), for active roles and validation state after the4096-limit restart. Initial speculative diagnostic reports are superseded by diagnostics-interface.md plus /private/tmp/codex-cache-run/diagnostics-brief.md refinements. No installation yet.
