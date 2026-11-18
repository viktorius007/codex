# Source from this worktree's codex-rs directory before build/test commands.
export PATH=/scratch/agent/stable170/sysroot/usr/bin:$PATH
export HOST_CC=/home/agent/.local/lib/kache/shims/cc
export HOST_CXX=/home/agent/.local/lib/kache/shims/c++
export TMPDIR=/scratch/agent/stable170/tmp
export CARGO_TARGET_DIR=/scratch/agent/stable170/target
export CARGO_BUILD_BUILD_DIR=/scratch/agent/stable170/target
export CARGO_BUILD_JOBS=32
export CARGO_PROFILE_DEV_DEBUG=none
export CARGO_PROFILE_TEST_DEBUG=none
export RUSTY_V8_ARCHIVE=/home/agent/.cache/rusty-v8-codex/librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz
export RUSTY_V8_SRC_BINDING_PATH=/home/agent/.cache/rusty-v8-codex/src_binding_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.rs
export PKG_CONFIG_PATH=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig
export LD_LIBRARY_PATH=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
test -L /home/agent/.cargo/registry || { echo 'Registry relocation has not completed' >&2; return 1; }
