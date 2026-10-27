# Stable 0.165.0 update (local.13)

## Target and starting point

- Previous base: `rust-v0.163.0`, local branch `local/customizations` at `e3d72a333c` (installed `0.163.0+local.12`).
- Target: `rust-v0.165.0`, published 2026-10-27 (release `0.165.0`, the latest stable; `0.164.0` was the intermediate stable).
- Upstream delta `rust-v0.163.0..rust-v0.165.0`: 400 commits, 2,234 files changed.
- Rollback ref for the old branch: `backup/local12-before-0.165.0` (`e3d72a333c`).
- Toolchain: upstream moved `rust-toolchain.toml` from 1.95.0 to 1.99.1 (installed here).
- Embedded V8: `v8` crate pinned from 150.4.0 to 152.2.0. The matching Linux sandbox archives were fetched from the `openai/codex` release `rusty-v8-v152.2.0`, verified against the published `.sha256`, and placed flat in `~/.cache/rusty-v8-codex/`; the 150.4.0 files moved to `~/.cache/rusty-v8-codex/v150.4.0/`.

## Most relevant upstream changes for the local patch set

Release notes: `0.164.0` (#53725 to #56049) and `0.165.0` (#56081 to #57337). The changes that touch the locally patched areas:

- **Agent-message wakes (#54961, #56330, #55644).** Upstream added the `WakeOnAgentMessages` feature: queued mail can wake an idle root turn, interrupts and Plan mode block such wakes, and pending injections now interrupt `wait_agent` and sleep. This overlaps the local bounded asynchronous-result delivery and the exec-completion wake. The local admission path was composed with upstream's feature-gated `pending_work_turn_configuration` and `should_wake_on_agent_messages` so both paths co-exist.
- **Inter-agent message lifecycle hooks (#55657).** `InterAgentCommunication` in queues and mailboxes became `PendingAgentMessage`, which carries a delivery handle. The local retained mailbox delivery type now wraps `PendingAgentMessage`, so hooks survive unloaded-recipient retention.
- **Prompt cache keys (#55587).** Callers can set `PromptCacheKey` independently of the session id. The local fresh-child prefix test was kept next to upstream's new parametrised tests.
- **Compaction (#53800, #53932, #55581, #55733).** Background compaction is wired into the turn lifecycle, failed compactions no longer leave partial summaries, and compaction payloads are boxed in `RolloutItem`. The local accepted-usage accounting and compaction-parity patches merged without semantic conflict.
- **Unified exec (#55719, #56011).** `write_stdin` now resolves a `TerminalSession` before the call, and terminal sessions stay distinct across process-id reuse. The local wake-on-exit flag is now read from the store entry under the same lock as stdin approval.
- **Skills (#55303, #55322, #55330, #55357, #55705, #55631).** The skills extension was restructured around step-scoped catalogs, source-aliased reads and installation metadata. Unmodified 0.165.0 still renders plugin-backed host skills with their cache-revision path (verified by running the local locator tests against the untouched tag: 2 of 3 fail), so the local stable-locator patch was re-ported onto the new design.
- **MCP (#56132, #56153, #54079, #56266, EMA state).** `McpConfig` gained runtime handles (`McpEmaCancellation`, `Arc<EmaState>`); the local sampled-authority comparison now compares those by identity.
- **Model catalogs and settings (#56163, #56284, #56313, #57337).** Reasoning-effort changes mid-conversation are on by default, catalogs are scoped by thread originator, and GPT-6.1 Terra is in the catalog.
- **Toolchain and runtime.** Rust 1.95.0 to 1.99.1; V8 150.4.0 to 152.2.0; `datamodel-code-generator` 0.64.0 for the Python SDK.
- **Fixed upstream, previously only local.** The app-server daemon test now sets `HOME` to its temporary home (local commit `6c04a55547` carried that; the test hunk was dropped, the installer script and recipe were kept).

## Rebase method

- `git rebase --onto rust-v0.165.0 rust-v0.163.0` of the 101-commit local series in the worktree `codex-wt/sync-0.165.0` (branch `sync/0.165.0`), with `rerere` on. 48 files conflicted in the net patch set.
- Simple conflicts were resolved per commit. For the core files that several later local commits reshape (`tasks/mod.rs`, `session/input_queue.rs`, `agent/api.rs`, `agent/control.rs`, `agent/control/{api,mailbox,residency,spawn}.rs`, `session/handlers.rs`, `lib.rs`), the resolution was a three-way merge of the final local file against the 0.163.0 and 0.165.0 versions, applied at the first conflicting commit; later commits that re-touch those files kept that composed state. Six commits therefore became empty and were dropped by the rebase: `08d4bf402e`, `0079d0c7b1`, `848cdfd618`, `904d5073cc`, `2c92629f70`, and the skills implementation `fe77d81f19`. [Patch comparison](stable-0.165.0-evidence/range-diff.txt).
- The skills implementation was re-ported as one new commit, "Use stable locators for host plugin skills", written against the 0.165 skills crate after the local locator tests were adapted and shown to fail on the untouched tag (2 of 3). On the port, all 216 `codex-skills-extension` tests pass and clippy is clean.
- One local resolution briefly committed two files with conflict markers (`session/mod.rs`, `tools/handlers/mcp.rs`); they were corrected five commits later in the series (`Restore catalog digest attribution on cold resume`). The intermediate commits are not individually buildable.
- Snapshot fixtures (`core/tests/suite/snapshots/*.snap`) and the precomputed app-server schema were kept at their upstream content during the rebase and regenerated afterwards.
- Post-rebase compile fixes, each its own commit: the request-observer test auth provider returns `Result`; `McpConfig` equality compares runtime EMA handles by identity; interface adaptations (`PendingAgentMessage` conversions, boxed `CompactedItem`, `ContentItem` fragment bodies, `ToolInvocation.environments`, `TerminalSession` wake flag).

## Validation

All runs on this Linux host (x86-64, Rust 1.99.1), from the frozen `sync/0.165.0` tree, with workspace binaries uplifted into Cargo's build directory first (tests resolve spawned binaries there; on this host Cargo's `build-dir` and `target-dir` differ, so `CARGO_TARGET_DIR` was pointed at the build directory for test runs).

- **Workspace type-check** (`cargo check --workspace --all-targets`, excluding `codex-voice-host`, which needs GStreamer): clean after the compile fixes listed above.
- **`codex-core`**: 5,447 tests; after the binary uplift and the fixes, every failure outside the groups below passes ([summaries](stable-0.165.0-evidence/core-suite-summaries.txt)). The required `background_exec_completion_starts_a_follow_up_turn_without_polling` passes. The investigator's targeted set (wake control, admission, agent execution, subagent notifications, prompt cache key, guardian retained context): 79 run, 79 passed ([report](stable-0.165.0-evidence/forked-child-startup.md)).
- **Other changed crates** (api, mcp, skills, app-server, app-server-protocol, config, utils-string, thread-store, goal, cache-diagnostics, cli, models-manager, protocol, rollout): 4,899 run, 4,895 passed ([summary](stable-0.165.0-evidence/other-crates-summary.txt)). The four failures were the stale schema fixture (regenerated), the sample snapshot (accepted: only tool-description hashes changed, from the local exec guidance), the known host failure `command_exec_process_ids_are_connection_scoped_and_disconnect_terminates_process`, and `host_roots::tests::repo_ancestry_without_project_marker_does_not_walk_parents`, which also fails on the untouched tag in the probe worktree.
- **Complete workspace suite** (`just test --workspace --exclude codex-voice-host`, once, before the last two fixture commits): 22,814 run, 22,787 passed, 26 failed, 1 timed out ([summary](stable-0.165.0-evidence/full-suite-summary.txt), [failures](stable-0.165.0-evidence/full-suite-failures.txt)). Of the 26, 17 are in the 0.163.0 host-failure baseline (TLS, TUI, sandbox, network approval). The other nine: the eviction scenario (fixed after the run: local emits one parent wake per worker result, so 12 requests and a refreshed snapshot), the skills root test above, and seven in crates the local patches do not touch (`exec-server` shell snapshots, `linux-sandbox`, `multi-env-copy-file`, `history-notes`); the shell-snapshot ones fail because bubblewrap now works on this host and the test's temp directory is read-only inside the sandbox.
- **Upstream baseline on the untouched tag** (probe worktree): `prompt_cache_key::api_key_subagent_inherits_prompt_cache_key_policy`, `compact::auto_compact_accounts_for_encrypted_reasoning` and the `agent_message_wake` tests all pass there ([log](stable-0.165.0-evidence/upstream-baseline-probe.log)); each was then traced to a local cause and fixed or documented.
- **Deliberate expectation changes to upstream tests** (both documented in the test): `auto_compact_accounts_for_encrypted_reasoning` expects compaction before turns 3 and 4 because the local pre-turn check projects the incoming prompt onto the reported usage; `api_key_subagent_inherits_prompt_cache_key_policy` identifies the forked child by its NEW_TASK delivery because the local test mock records only matched requests.
- **Behavioural fixes found by the upstream tests**: finishing turns keep their status on interrupt; with `WakeOnAgentMessages` off, an interrupt during a pending explicit follow-up no longer marks the session interrupted; async-result wakes stop after shutdown; MCP config equality ignores runtime EMA handles.
- **Formatting and lint**: `just fmt` applied Rust formatting; its Bazel step failed only because `dotslash` is not installed here. Scoped `just fix` over the changed crates is clean after annotating the one new lock-order await (`CARGO_INCREMENTAL=0` was needed to avoid a stale incremental-cache error). The app-server schema fixture was regenerated with `just write-app-server-schema`.
- **Not run**: `just bazel-lock-update` (Bazel is not installed; `MODULE.bazel.lock` carries no crate version), the Python SDK part of the schema generation, and macOS or Windows builds.

## Installation

- `just install-local-package` built the `dev-small` package for `x86_64-unknown-linux-gnu` with the 152.2.0 V8 archive and installed it on 2026-10-27 at 23:47:05 UTC to `~/.local/lib/codex-local/0.165.0+local.13-be77b674427c`; `codex-local/current`, `~/.local/bin/codex` and `~/.local/bin/codex-code-mode-host` point at it ([summary](stable-0.165.0-evidence/install-summary.txt)).
- Rollback release: `0.163.0+local.12-c522ed019cd9` (recorded as `previous=` in `~/.local/lib/codex-local/install-log.txt`).
- Checksums (sha256):
  - `codex`: `0365bde70e5b346a7ed8b4bb1ebdc00538938d99c85ceb51fee6845fce6ef265`
  - `codex-code-mode-host`: `e9d17e5f0d9378a7dfd403767040cb1cb05585c26b1e259d6aeced66fd5c89f3`
- Launch checks: `codex --version` prints `codex-cli 0.165.0+local.13`; `codex-code-mode-host --help` prints its usage; the one-turn `codex exec` smoke test is recorded in the publication section below.

## Publication and smoke test

- A one-turn `codex exec` with the installed binary (default model, ChatGPT account) answered `OK` ([log](stable-0.165.0-evidence/smoke-exec.log)).
- `local/customizations` was fast-forwarded to the rebased series and pushed to `origin` with `--force-with-lease=local/customizations:e3d72a333c`, the remote tip observed immediately before the push. The old tip stays available as `backup/local12-before-0.165.0`.
- Cleanup removed the `sync-0.165.0` and `probe-0.165.0` worktrees and their branches, the Cargo `target/` directories of those worktrees, and the temporary logs under `/tmp`. The 150.4.0 V8 archives were kept under `~/.cache/rusty-v8-codex/v150.4.0/` for the 0.163.0 rollback release.
