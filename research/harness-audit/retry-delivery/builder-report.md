STATUS: DONE
COMMIT: 16af2553d570fba1e44790f6b5cd7cba60d8ba91
CHANGED: codex-rs/codex-api/src/sse/responses.rs
BLOCKERS: none

# HTTP SSE terminal failure repair

`process_sse_with_treatment` now sends the decoded error from `response.failed` immediately and stops reading that response's stream. This preserves the terminal `ApiError::QuotaExceeded` classification when the transport fails afterward. Other event errors, including `response.incomplete` and `response.completed` parse errors, retain their previous handling. The patch adds no API and changes one production branch plus the existing coordinator-provided regression test.

The regression's discriminator is a quota `response.failed` event followed by a network error. On baseline `34d1bd0a11`, the exact test compiled and failed because the receiver got `ApiError::Stream` instead of `ApiError::QuotaExceeded` (see `coordinator-red.log`, summary: 1 failed). On this commit, `just test -p codex-api -E 'test(failed_response_keeps_terminal_classification_when_transport_fails_afterward)'` passed (1 passed, 193 skipped; `focused-green.log`). The test also checks that no second event follows the terminal error.

`just test -p codex-api` passed with 194/194 tests after allowing local mock servers to bind loopback ports (`codex-api-suite-escalated.log`). The first sandboxed suite run passed 173 tests; 21 mock-server tests failed with `PermissionDenied` on port binding (`codex-api-suite.log`). `just fmt` and scoped `just fix -p codex-api` passed; Clippy made no source changes (`codex-api-fix-escalated.log`). `git diff --cached --check` passed before the commit.

The full Rust workspace suite was not run because this change is limited to `codex-api`; repository instructions require separate authorization for that suite. No live provider request was made. The isolated `codex-rs/target` and logs remain for independent verification.
