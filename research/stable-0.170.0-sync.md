# Stable 0.170.0 patch rebase (local.14)

## Scope and result

Completed only the patch rebase on `chore/rebase-rust-v170`, starting at `3431a3e7bb` (111 local commits above `rust-v0.165.0`). Ran `git rebase --onto rust-v0.170.0 rust-v0.165.0`. The selected official release/tag had already been fetched and verified by the coordinator. The root `local/customizations` branch and the previous delegate's worktree were not changed. No subagents, builds, tests, installation, publication, or live probes were used.

The code handoff commit is `c01ba77105`; the subsequent documentation commit records this report. Workspace version is `0.170.0+local.14`; Cargo.lock workspace packages, including newly introduced upstream packages, follow it. The original tip remains available through the root branch and `backup/local13-before-0.170.0`.

## Preservation review

Read root AGENTS.md, the previous 0.165.0 report, and the previous delegate's incomplete 0.170.0 report. Inspected the full 111-commit local inventory and compared the changed-path sets: 414 local paths, 3,755 upstream paths, 195 overlapping paths. All locally added files remain present. The final [range comparison](stable-0.170.0-evidence/range-diff.txt) preserves the commit-by-commit comparison, including structural matching differences introduced by composition.

Preserved behavior families:

- Private bounded cache diagnostics, prepared-request observation, transport attempt attribution, restored catalog digests, instruction fingerprints, and offline audit tooling.
- Accepted usage checkpoints, invalidation after history replacement/removal, replay restoration, fully accounted recomputation, forced-full context signals, reasoning accounting, incoming-input projection, retained-output exclusion, and active compaction tools.
- Stable plugin skill locators and symlink resolution; host plugin reads remain available without enabling unrelated skill tools; disabled entries stay excluded.
- Frozen collaboration specs and model catalogs, fresh-child prefix construction, inherited instructions/settings, and tool-search pairing.
- Bounded async-result delivery, origin attribution, atomic reservation/rollback, parent residency through child completion, unloaded-recipient retention, shutdown guards, and Goal admission/accounting.
- Background exec exit wakes and duplicate-completion suppression, internal empty Code Mode waits, terminal SSE failure classification, MCP startup recovery and sampled authority, archive crash recovery, SemVer metadata, deterministic package installation, and local policy/research.

No local behavior was deliberately dropped or weakened. Upstream overlap was composed rather than assumed equivalent without tests.

## Conflicts and composition

Routine conflicts were resolved per commit. For repeatedly reshaped files, merged the final local file using the old stable file as ancestor and the new stable file as upstream, resolved the resulting conflicts, and reused that composed result at later conflicts. This avoids repeatedly applying obsolete intermediate representations. As in the prior stable update, intermediate commits are not promised individually buildable; the final tree is the handoff.

Four historical commits became empty because their changes were absorbed: `81fb5e4390` (response diagnostics), `75726ebd5b` (formatting), `978ae88fab` (Guardian sampling budget), and `4bd1ec4ff9` (old release version/schema). Their intended source behavior remains in the final composition; release versioning now uses local.14. The replay produced 107 commits, followed by a lock-version alignment commit and a source adaptation commit.

Key resolutions:

- **Usage accounting:** retain upstream `last_server_reported_usage_item_count` and `last_model_output_item_count` alongside local `AcceptedTokenUsage`. Accepted server usage uses upstream's covered boundary (`max(last model output, sampled input count)`) rather than the entire current history, so newly queued input remains estimated. Local checkpoints carry the accepted count and reasoning mode across replay; replacing/removing history invalidates them. Checkpoint installation, forced-full accounting, recomputation and buffered-output boundary resets synchronize the relevant boundary. Total estimates use upstream audio-duration-aware item estimation and the local reasoning compatibility distinction. Upstream's per-usage reasoning flag replaces the removed session-global flag.
- **Task lifecycle:** preserve atomic local reservation and rollback while retaining upstream host admission, outcome handoff tokens, pending initial input, continuation sampling and finalizer identity checks. Parent residency guards survive through terminal delivery. Upstream helper admission, prompt cache TTL and direct-only namespaces are carried into synthetic turns.
- **Transport diagnostics:** retain observers around upstream's reusable response future and last-response sender, routing affinity and asynchronous authentication. Boundaries continue to observe the prepared pre-auth request rather than secret headers.
- **MCP:** adapt sampled binding checks to scoped `McpServerKey`; retain identity/config/contract validation, Apps account-selector normalization, and upstream waiting for an in-flight Apps refresh. Preparation reports a descriptive error when the advertised tool or authority is no longer available.
- **Skills:** compose stable host plugin locators with upstream rendering policy, description omission, read metrics and source tracing. Host package fallback reads through the captured host snapshot; cloud/executor routes retain their normal behavior.
- **Fixtures:** appended regression tests were retained beside new upstream cases. Upstream's rewritten child-capture helper and inheritance tests were retained where it replaced older fixture routing. Conflicting generated `.snap` files and the precomputed app-server schema use upstream baselines; the coordinator must regenerate and review them against the composed runtime. Existing nonconflicting local fixture changes remain. No snapshots were claimed validated.

## Cheap checks and coordinator handoff

`just fmt` completed successfully; it formats tracked Rust/Bazel source using the repository formatter. Rust formatting also parsed the composed sources. `git diff --check`, conflict-marker scans, manifest parsing, local-added-file presence, and workspace/lock version consistency were checked. These checks do not establish compilation or runtime correctness.

Per explicit scope, **no builds or tests ran**. Coordinator owns compilation, focused regression execution (especially accepted usage/replay, async admission, parent residency, plugin locators, MCP authority, compaction parity and `background_exec_completion_starts_a_follow_up_turn_without_polling`), fixture/schema regeneration, lint, workspace validation and any installation/publication. Bazel lock regeneration was not run; version-only lock changes and preserved local dependencies require the coordinator's normal lock validation. No unresolved intentional semantic tradeoff was identified; the source remains uncompiled and untested.

## Cleanup

No build artifacts were created. Temporary merge scripts, merge inputs and formatter logs under `/tmp/rebase170` were removed after preserving the range comparison. Unknown files, existing diagnostic evidence, other worktrees and all installed binaries were preserved. Home free space was about 1.3 GB and scratch about 423 GB at the final cleanup check.

## Implementation/build validation continuation

Validation started in the prepared `chore/rebase-rust-v170` worktree at `3de2ac0ef53a8cb9d299bc7a3143ba4036108502`, without editing the root checkout. Registry relocation was confirmed complete before Cargo: `/home/agent/.cargo/registry` is a symlink to `/scratch/agent/cache/cargo-registry`. Home free space increased from 1.2 to 5.8 GB; scratch retained 419 GB. Open-file soft limit is 524288.

The indexed validation environment places Cargo target/build outputs together on scratch (so spawned test helper binaries resolve consistently), uses 32 jobs, disables debug symbols, and overrides stale macOS V8 paths with cached Linux 152.2.0 assets. GStreamer 1.28.2 development dependencies were extracted into a private scratch sysroot without sudo; all three required pkg-config checks pass. The stale Ubuntu update index advertised unavailable plugins-base packages (404); the matching release-pocket packages download successfully. The indexed setup script reproduces this repair. Full compilation and runtime linking remain to be verified.

Early `cargo build --profile dev-small -p codex-cli -p codex-code-mode-host` is running; no passing build or test result is claimed yet. Installation and publication remain excluded from this continuation.

The first build failed in `codex-mcp`: local snapshot equality referenced removed `server_permission_profiles`. Adapted it to `server_permissions` and included the new `host_cwd`, `apps_early_resource_discovery`, and `discover_local_stdio` settings, retaining exclusion of transient EMA handles. The second early build is running. Cargo also canonically reordered two lockfile dependency entries without changing dependency versions.

The MCP permissions enum now derives value equality; its filesystem sandbox context already implements value equality. The third build advanced beyond MCP and exposed the upstream filesystem-access migration in stable host plugin reads. Adapted those reads to use current callback accessors for repository skills and the host accessor for host-owned scopes, following upstream's existing ordinary host read implementation. Stable package identity and canonical containment checks remain. The fourth early build is running; skills behavior regressions remain required before acceptance.

The fourth build compiled skills and exposed core interface mismatches. Adapted parent residency pin arguments, recorded MCP approval preparation to sampled binding/server/tool arguments, interrupted MCP startup invalidation to an explicit refresh reason, pending mailbox message type, and the missing Arc import. Removed an obsolete duplicate provisional-child guard: the earlier guard already owns the reserved child, membership and edge write, so retaining it through disarm preserves cancellation cleanup. Removed the unreachable duplicate TokenUsageRecord match arm while retaining the earlier checkpoint reconstruction arm. Fifth build is running; no test result is claimed.

The fifth build showed that the rollout warning cleanup had removed three required match arms in addition to the unreachable duplicate. Restored those three and narrowed the change to the single duplicate arm; also removed the unused spawn-source binding. Sixth build is running.

Sixth early build succeeded for both CLI and code-mode-host. CLI launch reports `codex-cli 0.170.0+local.14`; host `--help` succeeds. Started required `just fmt` followed by a workspace binary build using the same dev-small profile and unified scratch target/build directory, before regression testing. Candidate is not yet test-validated, installed or published.

`just fmt` completed successfully, including the DotSlash-backed formatter. Workspace binary build then reproduced missing ALSA pkg-config metadata (`alsa-sys` 0.4.0). Extracted `libasound2-dev` and `libasound2t64` into the same scratch sysroot; `pkg-config --modversion alsa` reports 1.2.15.3. Updated the dependency reproduction script and reran the original workspace binary build. Compile adaptations are committed separately for MCP (`973f28fd40`), skills (`306ad6c403`) and core (`3365e32f65`); behavior tests are still outstanding.

The second workspace binary build passed ALSA but `opusic-sys` 0.7.5 could not execute missing `cmake`. Supplied Ubuntu CMake 4.2.3, cmake-data, libarchive13t64, librhash1 and libjsoncpp26 in the private sysroot; `cmake --version` succeeds with the validation environment. Added its bin directory to PATH and packages to the reproduction script, then reran the original workspace build (third attempt).

Third workspace attempt exposed two setup failures: GIO's transitive pkg-config requirements needed libselinux-dev/libsepol-dev (now extracted and resolving), and cmake crate 0.1.57 emitted `CMAKE_C_COMPILER=/home/agent/.cargo/bin/kache` for the configured `HOST_CC=".../kache cc"`, dropping the compiler subcommand. Observed kache 2.1.1+patched.fbbb3b7e39ca reject compiler flags as CLI arguments; expected a compiler invocation. The preserved third build log is the reproducer. Set HOST_CC/HOST_CXX to the existing kache compiler shims, preserving cache use while supplying CMake a single compiler executable. Fourth workspace attempt is running; original operation must still succeed before claiming this repaired.

Fourth workspace binary build passed for the full workspace, including voice-host, confirming the dependency and CMake compiler-shim repairs. Started focused validation through `just test --cargo-profile dev-small` covering skills, MCP, cache diagnostics, rollout, thread store, and core exec-completion, cache, usage, child notification, parent residency, Goal, compaction and MCP startup filters. No exclusion was needed for GStreamer. Tests are still running; schema regeneration/review and scoped lint precede the final frozen workspace suite.
