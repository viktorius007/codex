STATUS: DONE
COMMIT: none
CHANGED: 0 files
OPEN: 3
BLOCKERS: none for the expected source-only cache repair; a dependency change would block on missing Bazel until Bazel/Bazelisk is installed
---

# Cache-repair build, test, and installation procedure

## Decision

Use one dedicated integration worktree and its own `codex-rs/target`. Keep that target for formatting/lint compilation, targeted tests, the authorized full suite, and final development-profile executable builds. Install both `codex` and `codex-code-mode-host` from the same commit into one immutable versioned directory. Make both public executable names traverse one `current` symlink and atomically switch that single symlink between matched directories. Do not use `just install`: in this repository that recipe only reports the Rust toolchain and runs `cargo fetch`; it does not install a Codex executable.

The development profile is intentional. It has incremental compilation enabled, is the profile naturally warmed by the targeted and full test runs, and matches the observed 298,263,304-byte locally installed `codex`. Building release binaries after the full test suite would compile a second profile and substantially increase time and disk without adding validation required for this local repair.

## Measured facts at 2026-09-14, before any build

- Readiness worktree: `/private/tmp/codex-cache-build-readiness`, branch `work/cache-build-readiness`, clean at `d691087d0d43d31b681774db2da3ab12990af103`.
- The worktree is 89 MiB and has no `target/` or `codex-rs/target/`. No `codex-rs/target` was found under `/private/tmp` or `/Users/viktor/Projects/github`.
- APFS free space is 57 GiB. Cargo registry data is 2.4 GiB, Cargo Git data is 306 MiB, and installed Rust toolchains are 10 GiB. These are shared prerequisites, not cleanup candidates.
- `codex-rs/.cargo/config.toml` sets 32 build jobs. `[profile.dev]` in `codex-rs/Cargo.toml` explicitly enables incremental compilation and uses limited debug data. No compiler-wrapper, shared-target, or V8 override environment variable is currently set.
- The repository pins Rust 1.95.0. That toolchain is installed and active inside `codex-rs`, with Cargo 1.95.0, rustfmt, Clippy, rust-src, and the arm64 macOS standard library. `just` 1.58.0, `cargo-nextest` 0.9.133, `cargo-insta` 1.47.2, Git, ripgrep, Python 3, Apple Clang/Xcode, Deno, curl, `jq`, `xcrun`, `libtool`, `install`, and `shasum` are available.
- Bazel and Bazelisk are absent. CMake, Ninja, and protoc are absent, but they are not needed for the normal prebuilt-V8 path or the Cargo/Nextest procedure below. They become relevant only if an unexpected build path requests them.
- `just test` runs `cargo nextest run --no-fail-fast` with the local Nextest profile and an 8 MiB Rust minimum stack. With no package selector it covers the workspace; the workspace includes `codex-v8-poc`, `codex-code-mode-runtime`, and `codex-code-mode-host`.
- The Rust `v8` crate source and crate archive for version 150.4.0 are cached, but no reusable `librusty_v8` static archive was found. Its normal build script downloads a prebuilt archive from the pinned rusty_v8 release into the selected target directory. The full suite therefore needs network access once per empty isolated target. It does not need a local GN/Ninja/source V8 build unless the prebuilt archive is unavailable or `V8_FROM_SOURCE` is set.
- `codex-rs/Cargo.toml` and `Cargo.lock` currently agree on `0.154.0+local.1`. A new cache-repair patch set must use the next local revision, ordinarily `0.154.0+local.2`, in both files before final validation.
- Current installation:
  - `/Users/viktor/.local/bin/codex`: arm64 Mach-O, 298,263,304 bytes, SHA-256 `d571e6e415b142a00fc944768213014db0664d21843c2ed342b0db5e7f62f2e6`, reports `codex-cli 0.154.0+local.1`.
  - `/Users/viktor/.local/bin/codex-code-mode-host`: arm64 Mach-O, 82,992,392 bytes, SHA-256 `de5adcf6283a9e701f08cf5478d8a4ef52951a7aeb3a3e1236c99d538d102ce8`.
  - Both link only macOS system frameworks and `/usr/lib` libraries; no Homebrew or worktree dylib must accompany them.
  - At inspection, two `codex` processes and four `codex-code-mode-host` processes had those exact installed inodes open. Replacing a pathname on macOS leaves already-running processes on their open inode. Newly started processes must get the new matched pair.
- `code_mode_host` is stable and enabled by default. The standalone resolver looks for `codex-code-mode-host` beside the resolved main executable. A complete local install therefore requires both binaries from the same commit. The system `rg` remains the fallback search executable; `/opt/homebrew/bin/rg` is installed. No bundled package resource is needed for this existing standalone arrangement.
- The codebase-memory index was at the root checkout commit and reported no gap for the source files used here. `Cargo.lock` was not graph-tracked and `codex-rs/.cargo/config.toml` was deliberately excluded; both were read directly. Operational state, installed binaries, dynamic links, process inodes, tool versions, and disk sizes were also measured directly.

## Disk envelope

Measured local evidence covers a cold `cargo check -p codex-cli`, not a full test build: the incremental check target was 4.1 GiB, versus 1.2 GiB without incremental compilation. The repository experiment explicitly warns that isolated full/test targets can be substantially larger.

The full-suite target size is therefore an estimate: reserve 15-40 GiB for workspace test executables, incremental objects, and V8, plus about 0.4 GiB for each retained installed `codex`/host pair. With 57 GiB free, one run is plausible but does not have enough evidence to be called risk-free. Use these gates:

1. Require at least 50 GiB free before starting an empty-target full run.
2. Recheck after lint/format compilation and targeted tests. Require at least 35 GiB free before starting the full suite.
3. Recheck after the full suite. The final development binary builds should be mostly incremental; stop before them if less than 8 GiB remains.
4. Never delete diagnostic logs. After successful installation and provenance capture, remove only the exact agent-created target and finished worktree. Preserve unknown targets and all installation backups.

Do not interrupt a Rust command merely because it is slow or appears to wait on a Cargo lock. The repository explicitly requires patience and forbids killing Rust work by PID.

## Version and dependency gates

Perform these before final lint/tests so every validated artifact carries the final version.

1. Increment the workspace version in `codex-rs/Cargo.toml` from `0.154.0+local.1` to the next revision for this patch set.
2. From `codex-rs`, regenerate only workspace package versions without refreshing dependencies:

   ```sh
   cargo metadata --offline --format-version 1 --no-deps >/dev/null
   cargo metadata --offline --locked --format-version 1 --no-deps >/dev/null
   ```

3. Review `git diff -- codex-rs/Cargo.toml codex-rs/Cargo.lock`. For a version-only bump, lockfile changes should be the mechanical workspace package-version replacements seen in the previous local version commit; crate sources, checksums, and third-party versions must not change.
4. If any Rust dependency was added, removed, or changed, run `just bazel-lock-update` from the repository root and include `MODULE.bazel.lock`. This path is conditionally blocked today because neither `bazel` nor `bazelisk` is installed. Install Bazel/Bazelisk before continuing rather than omitting the required lock update. A version-only metadata change does not constitute a dependency change.
5. Before tests, require a clean lock relation with `cargo metadata --offline --locked --format-version 1 --no-deps >/dev/null` and inspect `git diff --check`.

## Execution order

The coordinator should substitute the final integration worktree, final commit, expected version, changed packages, and durable evidence directory. Do not use the readiness worktree after the implementation is integrated elsewhere.

```sh
export CACHE_REPAIR_WT=/absolute/path/to/final/cache-repair-worktree
export CACHE_REPAIR_TARGET="$CACHE_REPAIR_WT/codex-rs/target"
export CACHE_REPAIR_EVIDENCE=/private/tmp/codex-cache-run/evidence
cd "$CACHE_REPAIR_WT"
set -o pipefail
git status --short --branch
git rev-parse HEAD
df -h "$CACHE_REPAIR_WT"
test ! -e "$CACHE_REPAIR_TARGET" || du -sh "$CACHE_REPAIR_TARGET"
mkdir -p "$CACHE_REPAIR_EVIDENCE"
unset RUSTC_WRAPPER SCCACHE ZCCACHE_CACHE_DIR ZCCACHE_PATH_REMAP V8_FROM_SOURCE
export CARGO_TARGET_DIR="$CACHE_REPAIR_TARGET"
export CARGO_BUILD_JOBS=32
export CARGO_INCREMENTAL=1
```

Record the starting commit, branch, dirty/clean state, free space, Rust/Cargo/Just/Nextest versions, expected local version, and `shasum -a 256 codex-rs/Cargo.lock` in the durable run ledger. Record names and hashes, never private configuration values.

After the implementation and version/lock changes are complete:

1. Run package-scoped Clippy fixes once for every changed package. For the expected core-only repair:

   ```sh
   cd "$CACHE_REPAIR_WT/codex-rs"
   just fix -p codex-core 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/fix-codex-core.log"
   just fmt 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/fmt.log"
   git diff --check
   ```

   If shared crates changed, use the repository-required unscoped `just fix`; otherwise keep it package-scoped. Inspect changes produced by Clippy/formatting before testing.

2. Run the narrow regression tests first, then the changed package. If any local exec-completion preservation path or turn-admission code changed, the first command below is mandatory:

   ```sh
   just test -p codex-core -E 'test(background_exec_completion_starts_a_follow_up_turn_without_polling)' 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/exec-completion-regression.log"
   # Run every new cache diagnostic/prefix regression by its exact test filter here.
   just test -p codex-core 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/codex-core.log"
   df -h "$CACHE_REPAIR_TARGET"
   du -sh "$CACHE_REPAIR_TARGET"
   ```

3. Run the already-authorized full suite once, from the final source state. It must have network access because the empty isolated target lacks the rusty_v8 prebuilt archive. In the managed tool environment, request an unsandboxed/escalated `just test` execution up front or retry it that way if the only failure is the known network restriction. Keep `pipefail` in effect so `tee` cannot hide a test failure.

   ```sh
   set -o pipefail
   just test 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/full-just-test.log"
   df -h "$CACHE_REPAIR_TARGET"
   du -sh "$CACHE_REPAIR_TARGET"
   ```

   Nextest already uses `--no-fail-fast` and retries once under the local profile. Do not add `--all-features`; repository guidance says it expands the build matrix and disk use without being part of routine validation. Diagnose real failures; retry only failures justified as transient. Preserve every attempt log.

4. Build the two runnable development binaries after the suite, using the same target. These commands should reuse its compiled dependency graph and avoid a second release-profile tree:

   ```sh
   cargo build --offline --locked -p codex-cli --bin codex 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/build-codex.log"
   cargo build --offline --locked -p codex-code-mode-host --bin codex-code-mode-host 2>&1 | tee "$CACHE_REPAIR_EVIDENCE/build-code-mode-host.log"
   ```

   `--offline` makes this final step deterministic; the full suite should already have populated every dependency and the V8 archive in this same target. If it fails for a missing input, treat that as a prerequisite/provenance failure rather than silently refreshing dependencies.

5. Validate candidates before touching the installed paths:

   ```sh
   export CACHE_REPAIR_EXPECTED_VERSION=0.154.0+local.2
   export CACHE_REPAIR_COMMIT=$(git rev-parse HEAD)
   test "$(git status --porcelain)" = ""
   test "$("$CACHE_REPAIR_TARGET/debug/codex" --version 2>/dev/null)" = "codex-cli $CACHE_REPAIR_EXPECTED_VERSION"
   "$CACHE_REPAIR_TARGET/debug/codex" --help >/dev/null
   "$CACHE_REPAIR_TARGET/debug/codex-code-mode-host" --help >/dev/null
   file "$CACHE_REPAIR_TARGET/debug/codex" "$CACHE_REPAIR_TARGET/debug/codex-code-mode-host"
   shasum -a 256 "$CACHE_REPAIR_TARGET/debug/codex" "$CACHE_REPAIR_TARGET/debug/codex-code-mode-host"
   otool -L "$CACHE_REPAIR_TARGET/debug/codex"
   otool -L "$CACHE_REPAIR_TARGET/debug/codex-code-mode-host"
   ```

   Require arm64 Mach-O files and reject unexpected dynamic links outside `/System/Library` and `/usr/lib`. The clean-tree assertion belongs after the implementation has been committed; if the integration policy intentionally validates an uncommitted tree, record the exact diff hash instead and do not claim commit provenance.

## Atomic matched-pair installation and rollback

Use a versioned directory such as `/Users/viktor/.local/lib/codex-local/<version>-<commit12>-<codex-sha12>/` containing both executables. Stage and validate it completely before changing any entrypoint. Copy the currently resolved `codex` and host into a separate immutable backup directory first and record their hashes. One retained pair costs about 0.4 GiB at current sizes.

For the one-time migration from today's two regular files, point `/Users/viktor/.local/lib/codex-local/current` at the preserved old pair first. Replace each public regular file with a symlink through `current`; during those two replacements both names still resolve to the old pair. Then atomically rename one temporary `current` symlink over the old `current` symlink. Both public executable paths change to the new matched directory in that one rename. Future milestones need only stage a directory and atomically replace `current`. Do not copy one new executable over an installed file in place.

Existing Codex and host processes continue executing their open old inodes. The measured running harness therefore does not need to be stopped. The one-time public-name migration still resolves both names to the old backup; the single `current` rename is the matched-pair cutover for newly started processes.

Installation requires writing outside the repository sandbox and should use the user's existing authorization for validated local milestone installation. The concrete sequence is:

```sh
export CACHE_REPAIR_BIN_DIR=/Users/viktor/.local/bin
export CACHE_REPAIR_INSTALL_BASE=/Users/viktor/.local/lib/codex-local
export CACHE_REPAIR_SHORT_COMMIT=$(git rev-parse --short=12 HEAD)
export CACHE_REPAIR_CODEX_SHA=$(shasum -a 256 "$CACHE_REPAIR_TARGET/debug/codex" | awk '{print $1}')
export CACHE_REPAIR_CODEX_SHA12=$(printf '%s' "$CACHE_REPAIR_CODEX_SHA" | cut -c1-12)
export CACHE_REPAIR_OLD_SHA=$(shasum -a 256 "$CACHE_REPAIR_BIN_DIR/codex" | awk '{print $1}')
export CACHE_REPAIR_OLD_SHA12=$(printf '%s' "$CACHE_REPAIR_OLD_SHA" | cut -c1-12)
export CACHE_REPAIR_RELEASE="$CACHE_REPAIR_INSTALL_BASE/$CACHE_REPAIR_EXPECTED_VERSION-$CACHE_REPAIR_SHORT_COMMIT-$CACHE_REPAIR_CODEX_SHA12"
export CACHE_REPAIR_BACKUP="$CACHE_REPAIR_INSTALL_BASE/backup-before-$CACHE_REPAIR_EXPECTED_VERSION-$CACHE_REPAIR_SHORT_COMMIT-$CACHE_REPAIR_OLD_SHA12"

mkdir -p "$CACHE_REPAIR_INSTALL_BASE"
test ! -e "$CACHE_REPAIR_RELEASE"
test ! -e "$CACHE_REPAIR_BACKUP"
mkdir "$CACHE_REPAIR_RELEASE" "$CACHE_REPAIR_BACKUP"
install -m 0755 "$CACHE_REPAIR_TARGET/debug/codex" "$CACHE_REPAIR_RELEASE/codex"
install -m 0755 "$CACHE_REPAIR_TARGET/debug/codex-code-mode-host" "$CACHE_REPAIR_RELEASE/codex-code-mode-host"
install -m 0755 "$CACHE_REPAIR_BIN_DIR/codex" "$CACHE_REPAIR_BACKUP/codex"
install -m 0755 "$CACHE_REPAIR_BIN_DIR/codex-code-mode-host" "$CACHE_REPAIR_BACKUP/codex-code-mode-host"

test "$("$CACHE_REPAIR_RELEASE/codex" --version 2>/dev/null)" = "codex-cli $CACHE_REPAIR_EXPECTED_VERSION"
"$CACHE_REPAIR_RELEASE/codex-code-mode-host" --help >/dev/null
shasum -a 256 "$CACHE_REPAIR_RELEASE/codex" "$CACHE_REPAIR_RELEASE/codex-code-mode-host" "$CACHE_REPAIR_BACKUP/codex" "$CACHE_REPAIR_BACKUP/codex-code-mode-host"

# One-time conversion of today's regular public files to indirection through
# one pair-level pointer. All three temporary names must be absent beforehand.
test ! -e "$CACHE_REPAIR_INSTALL_BASE/current"
test ! -L "$CACHE_REPAIR_INSTALL_BASE/current"
test ! -e "$CACHE_REPAIR_INSTALL_BASE/.current.old-$CACHE_REPAIR_SHORT_COMMIT"
test ! -L "$CACHE_REPAIR_INSTALL_BASE/.current.old-$CACHE_REPAIR_SHORT_COMMIT"
ln -s "$CACHE_REPAIR_BACKUP" "$CACHE_REPAIR_INSTALL_BASE/.current.old-$CACHE_REPAIR_SHORT_COMMIT"
/bin/mv -fh "$CACHE_REPAIR_INSTALL_BASE/.current.old-$CACHE_REPAIR_SHORT_COMMIT" "$CACHE_REPAIR_INSTALL_BASE/current"

test ! -e "$CACHE_REPAIR_BIN_DIR/.codex.indirect-$CACHE_REPAIR_SHORT_COMMIT"
test ! -L "$CACHE_REPAIR_BIN_DIR/.codex.indirect-$CACHE_REPAIR_SHORT_COMMIT"
ln -s "$CACHE_REPAIR_INSTALL_BASE/current/codex" "$CACHE_REPAIR_BIN_DIR/.codex.indirect-$CACHE_REPAIR_SHORT_COMMIT"
/bin/mv -fh "$CACHE_REPAIR_BIN_DIR/.codex.indirect-$CACHE_REPAIR_SHORT_COMMIT" "$CACHE_REPAIR_BIN_DIR/codex"
test ! -e "$CACHE_REPAIR_BIN_DIR/.codex-code-mode-host.indirect-$CACHE_REPAIR_SHORT_COMMIT"
test ! -L "$CACHE_REPAIR_BIN_DIR/.codex-code-mode-host.indirect-$CACHE_REPAIR_SHORT_COMMIT"
ln -s "$CACHE_REPAIR_INSTALL_BASE/current/codex-code-mode-host" "$CACHE_REPAIR_BIN_DIR/.codex-code-mode-host.indirect-$CACHE_REPAIR_SHORT_COMMIT"
/bin/mv -fh "$CACHE_REPAIR_BIN_DIR/.codex-code-mode-host.indirect-$CACHE_REPAIR_SHORT_COMMIT" "$CACHE_REPAIR_BIN_DIR/codex-code-mode-host"

# Pair-level cutover: this rename switches both public executable paths.
test ! -e "$CACHE_REPAIR_INSTALL_BASE/.current.next-$CACHE_REPAIR_SHORT_COMMIT"
test ! -L "$CACHE_REPAIR_INSTALL_BASE/.current.next-$CACHE_REPAIR_SHORT_COMMIT"
ln -s "$CACHE_REPAIR_RELEASE" "$CACHE_REPAIR_INSTALL_BASE/.current.next-$CACHE_REPAIR_SHORT_COMMIT"
/bin/mv -fh "$CACHE_REPAIR_INSTALL_BASE/.current.next-$CACHE_REPAIR_SHORT_COMMIT" "$CACHE_REPAIR_INSTALL_BASE/current"

test "$("$CACHE_REPAIR_BIN_DIR/codex" --version 2>/dev/null)" = "codex-cli $CACHE_REPAIR_EXPECTED_VERSION"
"$CACHE_REPAIR_BIN_DIR/codex-code-mode-host" --help >/dev/null
test "$(shasum -a 256 "$CACHE_REPAIR_BIN_DIR/codex" | awk '{print $1}')" = "$CACHE_REPAIR_CODEX_SHA"
readlink "$CACHE_REPAIR_INSTALL_BASE/current"
readlink "$CACHE_REPAIR_BIN_DIR/codex"
readlink "$CACHE_REPAIR_BIN_DIR/codex-code-mode-host"
```

Rollback is the same one-rename pair operation, pointing `current` to the preserved backup pair:

```sh
test ! -e "$CACHE_REPAIR_INSTALL_BASE/.current.rollback-$CACHE_REPAIR_SHORT_COMMIT"
test ! -L "$CACHE_REPAIR_INSTALL_BASE/.current.rollback-$CACHE_REPAIR_SHORT_COMMIT"
ln -s "$CACHE_REPAIR_BACKUP" "$CACHE_REPAIR_INSTALL_BASE/.current.rollback-$CACHE_REPAIR_SHORT_COMMIT"
/bin/mv -fh "$CACHE_REPAIR_INSTALL_BASE/.current.rollback-$CACHE_REPAIR_SHORT_COMMIT" "$CACHE_REPAIR_INSTALL_BASE/current"
"$CACHE_REPAIR_BIN_DIR/codex" --version
"$CACHE_REPAIR_BIN_DIR/codex-code-mode-host" --help >/dev/null
```

Record installation time in UTC and Sydney time, source commit, branch, clean-tree/diff-hash status, expected version, Cargo.lock SHA-256, Rust/Cargo versions, exact test commands and outcomes, candidate and installed SHA-256/size, previous-pair SHA-256/size and backup path, versioned release directory, symlink targets, and rollback result. Keep this record with the durable cache-repair run ledger and retain all test/build logs as diagnostic evidence indefinitely.

## Cleanup after a validated installation

Capture final `df -h`, `du -sh "$CACHE_REPAIR_TARGET"`, installed symlink targets, and hashes first. Then remove only the exact target created by this run and, after its commits are integrated and verified, remove its finished worktree/obsolete branch through the coordinator. Do not remove `/private/tmp/codex-cache-run`, the evidence directory, installation provenance, either member of the preserved previous pair, Cargo registry/Git caches, Rust toolchains, or any unknown artifact.

## Open risks

1. **Full-suite disk use is estimated.** No surviving local Codex full-suite target exists to measure. The 15-40 GiB range is a conservative planning estimate derived from the measured 4.1 GiB incremental CLI check and the repository's explicit warning, not a benchmark.
2. **The empty target needs the prebuilt V8 download.** The relevant crate source is cached, but its static archive is not. Network/sandbox failure is the likely first-run prerequisite failure. A missing published arm64 macOS archive would be a real blocker; do not fall back to a V8 source build without a separate disk/time decision.
3. **Bazel is conditionally unavailable.** This does not affect a source-only cache repair, Cargo build, or Nextest run. Any Rust dependency change activates the repository's Bazel lock-update gate and requires installing Bazel/Bazelisk before the change can be called ready.
