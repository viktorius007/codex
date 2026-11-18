#!/bin/bash
# Invoke from the repository root. Temporary instrumentation is stored separately.
set -uo pipefail
cd codex-rs
source ../research/stable-0.170.0-evidence/validation/env.sh
export CARGO_HOME=/home/agent/.cargo RUSTUP_HOME=/home/agent/.rustup
export PATH=/home/agent/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/scratch/agent/failure-investigation/target
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"
just test -p codex-core -p codex-app-server -E 'test(network_rejection_preserves_execution_and_review_outcomes) | test(command_exec_process_ids_are_connection_scoped_and_disconnect_terminates_process) | test(model_lab_search_provider)'
just test -p codex-exec-server -p codex-linux-sandbox -E 'test(shell_snapshot_v2_capture_failure_falls_back_and_retries::remote_) | test(shell_snapshot_v2_filters_profile_exports_and_stays_in_memory::remote_sandbox) | test(shell_snapshot_concurrent_replays_keep_independent_readers::blocked_descriptor_path) | test(sandbox_keeps_parent_repo_discovery_while_blocking_child_metadata)'
