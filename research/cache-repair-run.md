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
| Current mechanism inventory | mechanism_scout | /private/tmp/codex-cache-run/mechanism-scout.md | running |
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
