# Source from this worktree's codex-rs directory before build/test commands.
export PATH=/scratch/agent/stable170/sysroot/usr/bin:$PATH
export HOST_CC=/home/agent/.local/lib/kache/shims/cc
export HOST_CXX=/home/agent/.local/lib/kache/shims/c++
export TMPDIR=/scratch/agent/stable170/tmp
export CARGO_BUILD_JOBS=32
export CARGO_PROFILE_DEV_DEBUG=none
export CARGO_PROFILE_TEST_DEBUG=none
export RUSTY_V8_ARCHIVE=/home/agent/.cache/rusty-v8-codex/librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz
export RUSTY_V8_SRC_BINDING_PATH=/home/agent/.cache/rusty-v8-codex/src_binding_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.rs
export PKG_CONFIG_PATH=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig
export LD_LIBRARY_PATH=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
mkdir -p /scratch/agent/stable170/tmp

# Keep the invoking T3 client identity out of first-party-only test fixtures.
unset CODEX_INTERNAL_ORIGINATOR_OVERRIDE

chmod 700 "$TMPDIR"
export GST_PLUGIN_PATH=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu/gstreamer-1.0
export GST_PLUGIN_SCANNER=/scratch/agent/stable170/sysroot/usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner

# Run test binaries directly so the cache runner cannot substitute stale helper paths or environment.
unset CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER

umask 077
export RUSTY_V8_SRC_BINDING_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/v8-binding-linux.rs"
export GST_REGISTRY=/scratch/agent/stable170/gst-registry-complete.bin
