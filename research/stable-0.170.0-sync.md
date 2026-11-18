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
