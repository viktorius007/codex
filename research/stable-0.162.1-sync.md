# Stable 0.162.1 local update

Installed and verified **0.162.1+local.11**. All 21,669 executed workspace tests passed, with 67 skipped and no failures or retries. The canonical installer activated `/Users/viktor/.local/lib/codex-local/0.162.1+local.11-e9fb3d6e2e28`; both PATH links resolve through its atomic `current` link. CLI version, code-mode host launch, ARM64 architecture and package-to-installed checksums passed.

Selected [stable rust-v0.162.1](https://github.com/openai/codex/releases/tag/rust-v0.162.1), published 2026-10-09, commit `092d3acd6bec3e3a14bdc7e7a2810ab628ab759d`. This follows the durable stable-release policy and avoids unreleased executor and fork changes in main. The clean `local/customizations` checkout contains the verified rebase; `main` advanced to the fetched upstream commit by a checked fast-forward. The actual checkout is `/Users/viktor/Developer/github/codex`; the session-supplied Projects path does not exist.

## Patch preservation

All 66 original commits replayed without skips. Three independent source audits found no fully equivalent upstream replacement for the scoped local production behaviors, so no local patch was removed or weakened. Retained behaviors include exec completion without polling, async attribution/admission, accepted usage boundaries and cold resume, active compaction catalogs, fresh-child shared prefixes, frozen tool specifications, private request fingerprints, MCP sampled authority and recovery, stable plugin locators, Goal accounting, internal Code Mode waits, malformed tool-search pairing, archive recovery and deterministic installation. The original inventory and final range comparison are preserved in the evidence archive.

Upstream migration required adapters for instruction provenance, world-state context, path/skill APIs, controller mailboxes and turn metadata. Dedicated instruction fingerprints still use keyed diagnostics without storing private plaintext. Generated fixtures were independently reviewed: 49 snapshots were accepted with permission, catalog and routing assertions retained. Decoded stable schema regeneration changes only RolloutLine to preserve ToolCatalogChanged; Python SDK generation produces no diff.

Executed causal failures exposed unloaded-recipient delivery and parent-eviction gaps. Typed controller deliveries now retain exact async payload/origin across unloading and consume each result once. Child task lifetimes pin the result parent through terminal delivery, cancellation and replacement; missing parents restore under current ancestor authority. The restoration future is boxed to keep turn admission within the original stack limits. Independent source reviews passed preservation of payload, origin, lock order, cancellation, restoration and ownership semantics.

Two functional consequences were surfaced during the work. Upstream numeric `fork_turns` now means full history rather than a bounded suffix; fresh-context delegation remains the repository default. Waiting parents remain resident while child turns run, so deeply nested active trees may reach the existing residency limit earlier. That limit was not raised, and the result-delivery protection was not weakened.

## Verification

| Check | Result |
| --- | --- |
| Final complete workspace suite at `1e4adde9ea` | 21,669 passed; 67 skipped; no failures or retries; 1,607.875 seconds |
| Corrected causal regression replay | 24 passed, including all prior stack overflows at original 8 MiB default / 4 MiB Guardian stack |
| Remaining broker/routing/exec replay | 11 passed, including all seven shell permission variants |
| Focused async preservation | 18 passed |
| Scoped normal Clippy and full formatting | Passed; one unchanged upstream let-and-return warning recorded |
| Full workspace binaries | Passed |
| Live Luna Code Mode smoke | Exit 0; exact background marker/exit confirmed; separate arithmetic 437; automatic completion reported without polling |
| Protocol / SDK / Bazel lock refresh | Passed; decoded schema change bounded; SDK unchanged |
| Installed package | Version, host launch, ARM64, links and checksums verified |

The first full run passed 21,655 tests and failed 14. Failure evidence was retained and grouped: nine stack overflows, four unchanged upstream terminal fixtures inheriting NO_COLOR, and one expected context order conflicting with the local prefix patch. A root-attribution count also raced with an unrelated automatic result turn. Surgical storage/fixture corrections preserve every assertion and do not raise stack limits or general timeouts. The final frozen-source suite passed completely. Raw red/green logs, JUnit, rejected snapshots, review hashes and worker patches are archived.

Builds used pinned Rust 1.95, 32 Cargo jobs, development incremental compilation and the verified V8 150.4.0 artifacts. Canonical installation used only `just install-local-package` and its atomic package installer. Tested code is `1e4adde9ead9b2fe3224a0ffdc6508b85ee48788`; package source is `5ee08da61333de4ccd5f5967288309efc4858a97`, differing only by the validation report. Later commits record evidence and policy links only.

## Installation, publication and cleanup

[Installed provenance](/Users/viktor/.local/lib/codex-local/0.162.1+local.11-e9fb3d6e2e28/provenance.json) records source, validation, checksums, rollback, exact publication receipt and managed-daemon activation. The final managed-daemon switch is recorded in [activation output](/Users/viktor/.local/lib/codex-local/0.162.1+local.11-e9fb3d6e2e28/daemon-update.log) and [daemon versions](/Users/viktor/.local/lib/codex-local/0.162.1+local.11-e9fb3d6e2e28/daemon-version.json). Publication uses an explicit force-with-lease tied to the freshly observed fork tip, followed by exact remote-tip verification.

Rollback package `/Users/viktor/.local/lib/codex-local/0.160.1+local.10-c446343404e8` remains intact; both original hashes are unchanged. Backup ref `backup/local10-before-0.162.1` retains original source `78e6a94a1d1a815d2a659f302fedd9c91dfe6c90`. New launches use the upgraded package; an already-running process retains its loaded binary until restarted.

All run-owned worker worktrees, the 47 GiB build worktree, obsolete work branches and the unique stopped Bazel output base were removed. Shared caches, unknown artifacts, both installed releases and diagnostic evidence were retained. The cleanup check showed one checkout and approximately 87 GiB free. The codebase-memory index was refreshed to the integrated source; its recorded parser gaps remain explicit.

[Evidence archive](stable-0.162.1-evidence/validation-evidence.tar.gz) preserves the complete run evidence; [manifest](stable-0.162.1-evidence/manifest.json) records individual and archive SHA-256 checksums. No diagnostic evidence was discarded.
