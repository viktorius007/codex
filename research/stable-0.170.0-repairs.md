# Repairs for the local.14 test failures (local.15)

19 November 2026. Follows [the failure investigation](stable-0.170.0-failure-investigation.md), which traced the 17 failing tests to four causes.

## Outcome

16 of the 17 tests are fixed and installed as `0.170.0+local.15`. One test, `blocked_descriptor_path`, still fails. The owner chose to record it as a known limit instead of starting a design change.

| Cause | Tests fixed | Repair | Merge |
|---|---|---|---|
| The restricted-network filter answered `getpeername` with "permission denied". Bash took that to mean its input came over a network and read `.bashrc` during snapshot replay. | 3 shell-snapshot tests | `linux-sandbox/src/seccomp.rs` no longer blocks `getpeername`. The call only reads the peer address of an existing socket; every call that connects, sends, receives or creates sockets stays blocked. | `6ce3d26f82` |
| This host's AppArmor transition clears bubblewrap's parent-death signal, so killing the outer process group left the sandboxed workload running. | 3 network-approval tests, 1 command-exec disconnect test | `linux-sandbox/src/lifetime.rs`: the inner sandbox stage holds a pidfd for the original launcher and ends the sandbox's process group and supervisor when the launcher dies. Six new tests cover direct and mount-wrapper launches in all three PID modes. | `fad6335020` |
| Model-search discovery was reused only when the MCP binding was the same object in memory, and startup readiness published a new one. | 8 search tests, plus `inherit_connector_version` | `codex-mcp/src/model_search.rs` and `connection_manager.rs`: reuse now compares what the binding serves (clients, catalog revisions, tool contracts, permissions and every server connection). A first version missed the connections of servers still starting up; review found it and a second commit closed it. | `3653eddd06` |
| The sandbox repository-discovery test only passed when its temporary folder was under `/tmp`. | 1 sandbox test | The test grants its helper folders an explicit write root; every assertion is unchanged. | `e28307362c` |

## Remaining limit

`shell_snapshot_concurrent_replays_keep_independent_readers::blocked_descriptor_path` denies `/dev/fd`. Bubblewrap creates `/dev/fd` as a symlink. Agent probes found that the system bubblewrap 0.11.1 rejects every mask tried on it, so the sandbox does not start. Codex's bundled bubblewrap cannot set up user namespaces under this host's AppArmor rule. A fix needs a different way to build the sandbox filesystem. A later executor disconnect seen in this test was never diagnosed.

## Validation

The full gate ran once on the merged tree `3653eddd06`:

| Step | Result |
|---|---|
| `just fmt` | exit 0, no changes |
| `just fix -p codex-linux-sandbox -p codex-mcp -p codex-core` (`CARGO_INCREMENTAL=0`) | exit 0, no changes |
| `cargo build --workspace --bins` | exit 0 |
| `just test` | 24,791 run, 24,788 passed, 3 failed, 54 skipped |

The three failures were `blocked_descriptor_path` and two `codex-cli` doctor snapshots that saved the old version string. The snapshots were updated in `e15caebe8a`, and their test binary then passed 7 of 7.

Each repair also passed its own scoped format, lint and test checks and a static Sol `high` review with no remaining findings. The tests were run on Linux only; macOS and Windows were not built.

## Installation

Installed with `just install-local-package` from `e15caebe8a`. The [installation record](/home/agent/.local/lib/codex-local/0.170.0+local.15-c825778ecd6e/provenance.json) holds the binary checksums and validation results. It also names the rollback release, local.14. The branch was pushed to `origin/local/customizations`.

## Environment notes

- Another session replaced `~/.cargo/registry` with a real directory on `/home`, so the tracked `validation/env.sh` stops at its registry-symlink check. The run used a copy without that check; Codex resolves its dependencies offline against the new directory.
- Scoped lint with incremental compilation failed in several lanes because the compiler's incremental cache files disappeared mid-build. Setting `CARGO_INCREMENTAL=0` for that command fixed it; the cause, the compiler or kache, is unproven.
- `deliver_mailbox_communication_to_current_turn` (`core/src/session/input_queue.rs:313`) was already unused at the base commit.
