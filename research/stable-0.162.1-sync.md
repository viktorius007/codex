# Stable 0.162.1 local update

Status: source port complete; compilation and verification pending. Installation has not occurred.

Selected stable `rust-v0.162.1`, commit `092d3acd6bec3e3a14bdc7e7a2810ab628ab759d`, published 2026-10-09. Official release: https://github.com/openai/codex/releases/tag/rust-v0.162.1 . This follows the repository stable-release policy and avoids unreleased changes in main, including executor replacement and additional fork/eviction changes.

Original checkout moved to `/Users/viktor/Developer/github/codex`; the session-supplied `/Users/viktor/Projects/github/codex` does not exist. Initial checkout was clean at `78e6a94a1d1a815d2a659f302fedd9c91dfe6c90`. Backup ref `backup/local10-before-0.162.1` preserves that source. The original installed rollback package is `/Users/viktor/.local/lib/codex-local/0.160.1+local.10-c446343404e8`; both binary checksums are in the evidence preflight record.

Three independent source-comparison audits found no fully equivalent upstream replacement for the scoped local production behaviors. Retained accepted usage boundaries/resume, compaction catalog parity, fresh-child prefix, frozen specifications, private diagnostics, MCP recovery and sampled authority, stable plugin locators, Goal output accounting, async completion/admission, internal Code Mode waits, malformed tool-search pairing, archive recovery, local metadata and deterministic packaging. These are source conclusions; passing runtime verification is still required.

All 66 commits replayed without skips. Source adapter/version tip is `1a7fa4f9574dd4cb0b32e42671e8eac1e0eb3660`. Manifest and workspace lock use `0.162.1+local.11`. Adapters compose upstream controller mailbox and turn attribution with local origin/reservation behavior, preserve current Guardian hooks, use the new PathUri/SkillReadContext interfaces, and use replacement WorldState context while retaining the originally admitted tool catalog. Upstream conflicting generated fixtures were retained as baselines; 35 artifacts require regeneration and inspection.

Upstream feature change surfaced to the user: numeric `fork_turns` values now select full history, rather than a bounded suffix (PR #51329). Repository policy already requires fresh-context agents. No local production patch was retired on that basis.

Build environment: pinned Rust 1.95.0, 32 Cargo jobs, development incremental compilation, isolated target, dev-small profile. Global kache is disabled only for these commands, following previous failed-cache evidence; global configuration remains unchanged. V8 stays 150.4.0; both cached artifact hashes passed published checksums. Starting free space was approximately 74 GiB; before compilation approximately 66 GiB.

Validation, final installation, managed-daemon version, publication and cleanup results will be appended as completed.
