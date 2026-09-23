# Custom patch audit

Baseline: `d5f8ef066a`, compared with `rust-v0.156.1`.

Scope: significant cache invalidation and wasted model turns introduced or left exposed by the local customizations. A finding requires an executed compiling regression test that fails on unchanged production code and asserts the causal mechanism. The user authorized fixes after coordinator review.

## Patch purposes and audit result

| Patch group | Purpose | Result |
| --- | --- | --- |
| Request diagnostics and HTTP/WebSocket observers | Record request identity and continuation decisions without changing requests | No proven significant defect in source review |
| Startup ordering and model/role snapshots | Keep shared request prefixes and tool descriptions stable | No proven significant defect in source review |
| Compaction and restored usage accounting | Use the active tool catalog during compaction and count pending/restored context consistently | No proven significant defect in source review |
| MCP sampled authority and stable skill locators | Execute the tools actually advertised and avoid cache-revision paths in prompts | Disabled-only plugin availability defect proven and repaired; other reviewed mechanisms had no proven significant defect |
| Goal, exec, subagent, and code-mode completion | Deliver useful results without empty polling, lost results, or duplicate completions | Duplicate terminal exec completion proven and repaired; the broader active-turn Goal restriction was rejected as unsupported |
| Local build/version support | Make builds practical and distinguish local binaries from upstream | No request-path defect in reviewed changes; global compiler-cache validation issue recorded separately |

No P0 or P1 defect was proven. Both admitted findings are P2. Source review does not establish exhaustive correctness, and no live provider cache-hit improvement is claimed.

## Run ledger

| Slice | Agent | Worktree | State |
| --- | --- | --- | --- |
| Prefix, compaction, usage accounting, resume | prefix_audit (Sol high) | `/private/tmp/codex-patch-audit/prefix` | Source review complete; no proven defect; no new tests run |
| Goal and asynchronous result delivery | async_audit (Sol high) | `/private/tmp/codex-patch-audit/async` | One proven P2 duplicate-completion defect; repaired and independently verified; integrated as `bde0997006` |
| MCP tools and skill catalogs | catalog_audit (Sol high) | `/private/tmp/codex-patch-audit/catalog` | One proven P2 defect; fixed and independently verified; integrated as `a0cd462856` |
| Diagnostics, transport instrumentation, remaining production patches | diagnostics_audit (Sol high) | `/private/tmp/codex-patch-audit/diagnostics` | Source review complete; no proven defect; no new tests run |

Initial checkout clean. Free space approximately 75 GiB; pre-existing root target approximately 78 GiB. Child shell inherited open-file soft limit 4096. Independent worktree targets required; build allocation coordinated to preserve disk space. Graph generation `2026-09-23T07:40:01Z` has partial Rust semantic coverage; reviewers must supplement graph results with source reads.

## Findings and verification

### P2 — disabled plugin changes the tool catalog

Disabled-only and absent-plugin snapshots render identical empty skill prompts, but the local locator patch advertised `skills.read` for disabled plugin entries. The regression `disabled_plugin_skill_does_not_advertise_read_tool` compiled and failed on unchanged production; the coordinator replay also failed with an extra serialized tool namespace. This is unnecessary request-catalog churn, not a measured provider cache-hit loss.

The selected surgical fix requires both `entry.enabled` and `entry.is_plugin_package()` before advertising the reader. A fresh Sol builder preserved the regression and obtained 176/176 passing skills-extension tests; an independent Sol reviewer reran the regression and checked enabled, executor, and orchestrator availability. Root integration commit: `a0cd462856`.

### P2 — terminal stdin receipt leaves a duplicate completion armed

After an initial exec yields a live process, terminal `write_stdin` can return its exit result while leaving the shared watcher flag armed. The watcher subsequently queues another model-visible completion. The coordinator replayed all three regressions red on unchanged production: normal exit, an entry removed before exit, and the real watcher enqueueing the exact duplicate response item. The watcher event and shared interaction mutex establish causal ordering; timeout values only guard hung tests.

The selected repair retains the existing flag alongside the process handle and clears it immediately before returning a successful terminal response, after any awaited event handling and while still holding the interaction mutex. Live responses, errors, and unattended background exits retain their notification behavior. A fresh Sol builder obtained 3/3 green regressions and 173/173 related tests. A separate Sol reviewer independently ran the three cases plus the unattended-background completion test; all four passed.

The candidate claiming that all denied Goal results must be hidden from active turns was rejected: admission governs automatic turn starts, and a normal final response closes current-turn delivery. A red assertion against that broader premise is not evidence of a product defect.

## Combined validation

The isolated core run had 4,335 passes and 274 failures, including absent helper executables. It is not claimed as green. Final integration rebuilds workspace binaries before core and workspace tests.

Root formatting passed. Scoped lint first failed on missing incremental files while the global `kache` wrapper was active; rerunning with command-local `RUSTC_WRAPPER= HOST_CC=clang HOST_CXX=clang++` passed. This establishes a working validation setup, not a diagnosed cache defect. Global compiler configuration was not changed. Workspace version and lockfile are aligned at `0.156.1+local.7`; dependency versions are unchanged.

After helper binaries were rebuilt, the core run produced 4,599 passes, 10 failures and 27 skips. Focused low-concurrency reruns cleared the five credential and two MCP timing failures. Three failures persisted: a skill fixture consumes `TurnComplete` twice, a denied-read fixture receives an unexpected child approval, and a persistent-terminal fixture changes restrictions that the running terminal cannot adopt.

For a direct baseline comparison, the coordinator temporarily replaced both changed production files with their `d5f8ef066a` contents and ran exactly those three cases. All three failed again; the repaired files were automatically restored and compared byte-for-byte with the reviewed worktrees. These failures predate the repairs. No assertions, timeouts, denied-read policy, or unrelated production behavior were changed to obtain a green result.

The complete workspace attempt used four test threads, but linking test executables exhausted disk before execution. There is no complete workspace pass. The candidate CLI and host from the successful workspace-binary build were preserved; both launch/help checks pass, the CLI reports `0.156.1+local.7`, both binaries are arm64 and link only macOS system libraries. Source commit `1329fc249e` records the final Rust source tree; the binaries were built before that commit was recorded from the same source contents.

## Delivery and cleanup

The checked pair is staged at `/Users/viktor/.local/lib/codex-local/0.156.1+local.7-1329fc249e83-57ce616cb9f2`, with provenance and the build log. The installed release remains local.6 pending the user's choice between scoped delivery, retaining the candidate, or an additional workspace build. This choice was requested under the user's explicit Pareto rule after the full-suite build hit disk exhaustion.

Run-created isolated targets and compiler artifacts were removed using recorded creation-time boundaries, preserving pre-existing files, the installed release, and all audit evidence. Free disk recovered to approximately 73 GiB. Raw validation logs, baseline failure proof, reports, and exact cleanup manifests are preserved in the hash-verified validation archive.

All nine delegated agents completed. All four task worktrees and their branches were removed after checking source integration and evidence preservation. The temporary task directory was removed after verifying the archive and staged candidate hashes; the root checkout is the only remaining worktree. The pre-existing root build directory was retained except for files created by this run.
