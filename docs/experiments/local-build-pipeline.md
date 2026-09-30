# Local multi-agent build pipeline experiment

Date: 2026-09-13

## Question

Find the simplest local Rust build configuration that shortens wall time on this Mac while keeping concurrent agent worktrees independent.

## Local constraints

- MacBook Pro with an M3 Max, 16 CPU cores, 48 GB memory, and limited free disk space.
- Every file-writing agent uses a separate Git worktree and branch.
- Every worktree keeps its own Cargo `target/` directory. Sharing one target directory would let Cargo's directory lock serialize independent agents.
- The shell initially allowed 256 open files per process.
- Measurements used `cargo check -p codex-cli --quiet` with empty target directories unless the row says it was an edit rebuild.

Timings are local observations, not portable guarantees. Rebenchmark after material hardware, toolchain, file-limit, or cache changes.

## Cargo jobs

| Configuration | Real time | User CPU time | System CPU time | Result |
| --- | ---: | ---: | ---: | --- |
| Cargo default, 16 jobs, incremental default | 94.12s | 351.61s | 65.95s | Passed |
| 32 jobs, incremental disabled | 83.80s | 332.91s | 64.63s | Passed |
| 64 jobs, incremental disabled | 1.33s | 3.88s | 1.86s | Failed before useful compilation |

Thirty-two jobs shortened the cold check by 10.32 seconds, about 11%. Sixty-four jobs exceeded the process's 256-open-file limit and produced `Too many open files (os error 24)`. A shell-local `ulimit -n 4096` succeeds, but a higher limit was not shown to improve build time.

The 256-file allowance is per process, not one machine-wide pool shared by all agents. Separate Cargo processes receive separate descriptor tables, so this failure does not show that ordinary multi-agent development is blocked. Do not change the machine-wide limit without a benchmark showing that more than 32 Cargo jobs improves total throughput.

## Incremental compilation

The controlled comparison kept both variants at 32 jobs and used separate empty targets.

| Configuration | Cold check | Target size after cold check | Check after the same comment-only edit |
| --- | ---: | ---: | ---: |
| Incremental enabled | 96.18s | 4.1 GiB | 2.00s |
| Incremental disabled | 83.83s | 1.2 GiB | 2.69s |

Incremental compilation made this small edit rebuild 0.69 seconds faster, about 26%, while adding about 2.9 GiB and 12.35 seconds to the cold build. The extra storage and cold-build cost are acceptable locally, so development incremental compilation remains enabled.

## `sccache` 0.17.0

The neighbouring `project-management` checkout configures `sccache` as Cargo's Rust compiler wrapper and points all worktrees at a cache under Git's common directory. That shares the store, but released `sccache` does not normalize Rust checkout paths.

| Run into a new empty target | Real time | Rust cache result |
| --- | ---: | --- |
| Cold cache | 96.60s | 1,074 misses |
| Cache populated by the equivalent other checkout | 95.27s | 1,074 additional misses, zero hits |

Only non-Rust compiler entries hit. Rust artifacts did not transfer between absolute worktree paths. The upstream fix remains open in [`sccache` PR #2794](https://github.com/mozilla/sccache/pull/2794), tracked by [issue #2652](https://github.com/mozilla/sccache/issues/2652). Re-evaluate after it ships in a stable release.

## `zccache` 1.13.22

The experiment followed the released worktree instructions:

- `RUSTC_WRAPPER` pointed at `zccache` 1.13.22.
- `ZCCACHE_PATH_REMAP=auto` was enabled.
- Both real Git worktrees used the same `ZCCACHE_CACHE_DIR` and distinct empty Cargo targets.
- The wrapper's automatically started daemon was running for the measured builds.

| Run | Real time | Cache result |
| --- | ---: | --- |
| Cold cache | 145.85s | Cache population |
| Equivalent source in a different absolute worktree | 82.36s | 854 hits, 1,636 misses cumulatively; 34.3% hit rate |
| Comparable check without a compiler cache | 83.80s | Not applicable |

Cross-worktree caching worked, but the warm check saved only 1.44 seconds, about 1.7%, and the cold population cost was 62.05 seconds. The README's roughly 1 ms figure describes delivery of one existing hit, not complete Cargo wall time; Cargo work, cache misses, build scripts, procedural macros, and non-cacheable outputs remain. The optional target-snapshot and `zccache warm` layers were outside this compiler-cache experiment.

## Adopted configuration

- Cargo build jobs: 32.
- Development incremental compilation: enabled.
- Cargo targets: isolated per worktree.
- Compiler cache: none until a controlled cold/warm cross-worktree benchmark shows a meaningful net wall-time benefit.
- Agent-created worktrees, targets, and caches: remove after use to control disk consumption.

The targeted validation after the initial build configuration change was `just test -p codex-cli`: 432 tests passed in 9.023 seconds. Formatting also passed. The incremental decision was then validated by the controlled edit-rebuild timings above.

## Follow-ups

These are deliberately not part of the adopted configuration until their measurements justify the additional work:

1. Benchmark two or more simultaneous builds in isolated worktrees. The completed measurements cover individual Cargo processes, not aggregate multi-agent throughput or oversubscription.
2. Watch [`sccache` PR #2794](https://github.com/mozilla/sccache/pull/2794). After the fix ships in a stable release, repeat the moved-worktree cold/warm test before adopting it.
3. Correct the neighbouring `project-management` cache guidance: its shared store is ready for a future path-normalizing `sccache`, but released `sccache` currently provides no Rust cross-worktree hits.
4. Benchmark `cargo build -p codex-cli --bin codex` if executable builds become the observed bottleneck. The completed experiment measured `cargo check`, which omits final code generation and linking.
5. Preserve aggressive cleanup of agent-created targets and worktrees. About 56 GiB was free after cleanup, and isolated full/test targets can consume substantially more space than the check targets measured here.
