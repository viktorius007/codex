# Stable 0.163.0 update (local.12)

## Result

- `local/customizations` was rebased from `rust-v0.162.1` onto `rust-v0.163.0`. The rollback ref is `backup/local11-before-0.163.0` at `4626b410c9`.
- It was published to `origin` with `--force-with-lease` bound to `4626b410c9`. After the push, the remote tip equalled the local tip.
- `codex-cli 0.163.0+local.12` was installed on 2026-10-22 at 12:23:44 UTC, at `~/.local/lib/codex-local/0.163.0+local.12-c522ed019cd9`, through `just install-local-package`.
  - This is the first local install on this Linux host (`previous=none`).
  - Before it, `~/.local/bin/codex` pointed at the npm `@openai/codex` package, which is still installed under `~/.local/lib/node_modules/@openai/codex`.
- The installed binaries are x86-64 ELF for `x86_64-unknown-linux-gnu`, built with the `dev-small` profile:
  - `codex`: sha256 `570ba1a13bbae8286f4c0b403ba6855b2be6afab76aecb10b43ad41114f97b09`
  - `codex-code-mode-host`: sha256 `7fc0f8fe5fb3815feb538b30f3e6bef22ad238ec7bd7e9c4596c3c24d51775e3`
- Launch checks passed: `codex --version`, `codex-code-mode-host --help`, and a one-turn `codex exec` smoke test that answered `OK`.

## Changes beyond the mechanical rebase

- **Interface adaptation:** upstream changed the shapes of `ResponseInterrupt`, `PreparedToolInfo`, `StartThreadOptions` and `from_spawned`, and added `encrypted_content`/`status` fields. Local patch code and tests were updated to match.
- **Cached Apps MCP bindings:** the comparison between a cached binding and the live tool now ignores the account selector. This applies to `link_id`, `requires_explicit_link_id` and the `link_id` argument. Without it, the local authority check (from commit 46b53de989) blocked upstream's link-scoped session approval prompt. The live account is still resolved and validated when the call runs.
- **Pre-turn compaction:** with `RetainClientToolOutputs` enabled, retained tool outputs are no longer counted in the pending-input estimate (from commit fa9d1e2b3b). Compaction keeps those outputs whole, so counting them forced a compaction that could not free space. That compaction then deadlocked behind an in-flight background compaction.
- **Scenario snapshots:** the snapshots for new upstream scenarios were accepted with the local patch output:
  - local message order
  - exec-completion guidance
  - stable plugin skill locators
  - one parent wake per worker result
- **Removed tests:** the zsh-fork approval tests that upstream deleted were removed.
- **Linux install support:**
  - The `install-local-package` recipe selects the host's rusty_v8 archive and passes `--target` for the host triple. The packaging script defaults Linux to musl, whose standard library is not installed here.
  - `install_local_package.sh` uses GNU `mv -T` when it is available, and falls back to BSD `mv -h`.

## Validation

- The full workspace suite was run once against the frozen tree, excluding `codex-voice-host`, which needs GStreamer, a library this host lacks. Result: 22,079 tests, 21,860 passed, 217 failed, and 2 timed out ([summary](stable-0.163.0-evidence/full-suite-summary.txt), [failures](stable-0.163.0-evidence/full-suite-failures.txt)).
- Every remaining failure is caused by the host, not by the local patches:
  - AppArmor `apparmor_restrict_unprivileged_userns=1` blocks user namespaces, so bubblewrap fails with `setting up uid map: Permission denied`. This covers sandbox, network-proxy, approval and linux-sandbox tests.
  - TLS fallback and local socket tests also fail on this host.
  - The 105 failures that the sandbox error does not explain were rerun on unmodified `rust-v0.163.0`, where 87 also fail ([baseline](stable-0.163.0-evidence/upstream-baseline.txt)). The other 18 now pass on the branch.
  - `guardian_retained_context::...::referenced_truncated_checkpoint` timed out under full-suite load. It passes when its group is run in isolation.
- `background_exec_completion_starts_a_follow_up_turn_without_polling` passes.
- **Not run:**
  - `just bazel-lock-update`: Bazel is not installed, and `MODULE.bazel.lock` contains no crate version.
  - Scoped `just fix`.
  - The Python SDK step of `just write-app-server-schema`, which failed on `datamodel_code_generator`. The Rust schema fixture was regenerated.

## Linux build environment

- `/scratch/agent/sysdeps/env.sh` sets the following. `/scratch` is wiped on boot, so it must be recreated after a restart.
  - `PKG_CONFIG_PATH`, pointing at an extracted `libcap-dev` package (from `apt-get download libcap-dev`, with the paths in its `.pc` file rewritten).
  - The Linux `RUSTY_V8_*` paths. The archives live in `~/.cache/rusty-v8-codex/`, from `openai/codex` release `rusty-v8-v150.4.0`, and their sha256 was verified.
- Tests that start workspace binaries look for them in Cargo's `build-dir`. Run `cargo build --workspace --exclude codex-voice-host --bins` before running suites.
- [Patch comparison](stable-0.163.0-evidence/range-diff.txt) compares the old and the rebased local patch series.
