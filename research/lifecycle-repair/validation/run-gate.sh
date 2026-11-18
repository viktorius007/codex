#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../../codex-rs"
source ../research/stable-0.170.0-evidence/validation/env.sh
export CARGO_TARGET_DIR=/scratch/agent/fix170/lifecycle/target
export CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR
log_dir=../research/lifecycle-repair/validation
case "${1:-}" in
  prepare)
    cargo build --bins -p codex-linux-sandbox -p codex-cli -p codex-app-server -p codex-code-mode-host > "$log_dir/helpers.log" 2>&1
    just fmt > "$log_dir/fmt.log" 2>&1
    CARGO_INCREMENTAL=0 just fix -p codex-linux-sandbox > "$log_dir/lint.log" 2>&1
    ;;
  test)
    just test -p codex-linux-sandbox -p codex-core -p codex-app-server -E 'package(codex-linux-sandbox) | test(network_rejection_preserves_execution_and_review_outcomes) | test(command_exec_process_ids_are_connection_scoped_and_disconnect_terminates_process)' --retries 0 > "$log_dir/gate.log" 2>&1
    ;;
  *) echo 'usage: run-gate.sh prepare|test' >&2; exit 2 ;;
esac
