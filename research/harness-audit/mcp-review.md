STATUS: DONE  
COMMIT: 34d1bd0a110879b6c8d0b7d517a22ceb367aba9b  
CHANGED: test-only `codex-rs/core/tests/suite/mcp_optional_startup_grace.rs`; `AGENTS.md`; this report, `findings.json`, `red-test.patch`, and raw logs  
OPEN: 1  
BLOCKERS: none  
PROVEN_DEFECTS: 1  
REVIEWED: 9 entry points / 17 bounded source and test regions  
RED_TESTS: 1 compiled and executed / 1 causally failed on unchanged production (two attempts)

# Interrupted MCP startup recovery audit

## Slice and source coverage

The bounded path is thread startup and publication, idle or active interruption, and the next turn's tool capture. The actors are the user interrupt, the MCP server's initialize response, the session refresh worker, and the model turn. GitHub #46399 was a lead only; the conclusion below rests on local code and an executed test.

| Entry point or boundary | Terminal outcomes reviewed | Disposition |
| --- | --- | --- |
| `Session::install_initial_mcp_runtime` (`core/src/session/mcp_runtime.rs:111-153`) | Publish initial runtime, then validate required servers | Source mapped |
| `McpRuntime::publish` (`codex-mcp/src/runtime.rs:329-388`) | Store immutable snapshot, open publication gate, update event stream cancellation | Prepublication notification candidate dismissed by source ordering |
| `McpConnectionSet::new` (`codex-mcp/src/connection_manager.rs:218-827`) | Reuse ready or pending connections; start new clients; emit terminal status and summary | Cancellation and reconnection paths mapped |
| `Session::interrupt_task` (`core/src/session/mod.rs:4895-4902`) | Active turn abort, or idle startup cancellation | **Proven defect on idle path** |
| `ManagedClientStartup::start` and `AsyncManagedClient` (`codex-mcp/src/rmcp_client.rs:320-620`) | Ready, failed, or canceled shared startup; Apps-only failed-startup reconnect | Canceled result is retained; regular server has no retry |
| `Session::mcp_runtime_for_step` (`core/src/session/mcp.rs:339-388`) | Refresh on changed inputs, then capture binding | Unchanged turn retains canceled snapshot |
| `McpConnectionSet::capture_binding_with_metadata` (`codex-mcp/src/connection_manager/tool_catalog.rs:206-386`) | Ready tools captured; error client omitted | Missing model tool directly observed |
| `Session::refresh_mcp_if_dirty` and `McpRefreshInvalidationGuard` (`core/src/session/mcp.rs:172-247`, `mcp_refresh.rs:1-56`) | Clean return, publish, or restore claimed invalidation on task cancellation | Claimed-refresh loss dismissed; idle interrupt never invalidates |
| Explicit refresh, prewarm, status, and shutdown (`core/src/session/handlers.rs:255-258`, `mcp_prewarm.rs:1-81`, `codex-mcp/src/connection_manager/status.rs:1-29`, `runtime.rs:675-768`) | Manual reconnect, coalesced background refresh, read-only status, teardown | Source mapped; manual refresh can recover but is not triggered by the idle interrupt |

The codebase-memory graph generation at 2026-09-23T12:39:48Z reported no recorded file gaps for the relied-on paths. Rust call and implementation relationships were marked semantically partial; direct worktree source reads and `rg` closed the material call chains. The edited test was read directly after the graph generation. Scope was bounded to these functions and adjacent branches, not a complete audit of each large module.

## Proven finding: idle interrupt poisons the published MCP startup

The test creates one configured streamable HTTP server, holds its first initialize response, observes `McpStartupStatus::Starting` and exactly one initialize request, sends `Op::Interrupt` with no active turn, and observes `Cancelled`. It then releases the server and starts a new turn without changing thread settings. The test asserts both a second initialize and the recovered tool in the model request. On baseline production, the observed pair was `(1, false)` instead of `(2, true)` in both nextest attempts. The turn completed; the tool was absent. The full test patch is [red-test.patch](red-test.patch), embedded in [findings.json](findings.json), and the raw output is [red-test-final.log](red-test-final.log).

The cause is the idle branch in `Session::interrupt_task` (`core/src/session/mod.rs:4895-4901`): `cancel_mcp_startup` cancels the connection token, but never marks the runtime dirty. `ManagedClientStartup::start` converts that token into a shared `Cancelled` outcome (`codex-mcp/src/rmcp_client.rs:399-420`). The next turn's dirty checks cover auth, environments, and capability roots (`core/src/session/mcp.rs:172-202,339-372`), so an unchanged turn keeps the same published connection. Binding capture drops its errored client (`codex-mcp/src/connection_manager/tool_catalog.rs:285-298`). `reconnect_failed_startup` handles `Failed`, not `Cancelled` (`codex-mcp/src/rmcp_client.rs:556-565`).

User consequence: if a user interrupts while an MCP server is starting and no turn is active, that server's tools remain unavailable in subsequent turns despite the server becoming ready. An explicit MCP refresh or unrelated runtime reprojection can restore them. This affects one server in one thread; no data loss was shown. Severity is high for workflows that rely on that server's tools.

## Dismissed candidates and limits

- Ready client canceled by the idle interrupt: `McpServerConnection::cancel_startup` checks `startup_complete` (`codex-mcp/src/connection_manager.rs:141-145`), and the existing ready-client case is at `connection_manager_tests.rs:3920-3936`. Reopen if a ready client can still have the flag unset.
- Claimed dirty refresh lost when its task is dropped: `McpRefreshInvalidationGuard` restores the pending flag (`core/src/session/mcp_refresh.rs:44-55`). Reopen if another publication path bypasses this guard.
- Prepublication status emitted before runtime state is visible: the publication gate opens after `current.store` (`codex-mcp/src/runtime.rs:329-378`). Reopen if another event path bypasses that gate.

The test uses a synthetic local server and mocked model response; no live provider was contacted. This audit did not test cancellation inside an in-progress replacement before `current.store`, or a canceled refresh that mutates shared elicitation authority before publication. Those are coverage limits, not findings. The existing active-turn cached startup test (`core/tests/suite/mcp_tool_cache.rs:1248-1349`) exercises a distinct deferred-startup path.

## Commands and resource evidence

- `just fmt` passed with `UV_CACHE_DIR=/private/tmp/codex-harness-audit/uv-cache-mcp`. The default UV cache was inaccessible in the sandbox; formatter changes to three unrelated Python files were restored to HEAD, leaving only the test diff.
- First `just test -p codex-core -E 'test(interrupted_idle_mcp_startup_reconnects_for_next_turn)'` compiled but could not bind WireMock's loopback port in the sandbox. Its log was overwritten by the next run; the bind error remains in the tool transcript and is not defect evidence.
- The focused command with local socket access first failed on one initialize attempt instead of two: [red-test.log](red-test.log). After tightening the assertion, it failed at `(1, false)` versus `(2, true)` twice: [red-test-final.log](red-test-final.log).
- Existing startup control selector `test(optional_mcp_startup_grace_controls_initial_turn_tool_catalog)` had three passes and one 500 ms deadline failure in a four-case run: [control-test.log](control-test.log). Isolating `zero_grace_waits_for_server_startup` produced a pass on retry after one 5 s startup deadline failure: [control-success.log](control-success.log). These timing failures are incidental load sensitivity; the causal red test reached its exact assertions twice.
- `git diff --check` and JSON/patch consistency checks passed. At completion, the isolated `codex-rs/target` is approximately 12 GiB and is retained for coordinator handoff. Final disk checks found 26–28 GiB free on the shared volume as other work continued. The agent-created 47 MiB UV cache was removed; unknown or user-created files were not touched. The raw `*.log` files are ignored by Git but remain in this worktree for replay and should be copied explicitly if the worktree is retired.

Minimal repair consideration for the coordinator: when the idle interrupt actually cancels a published startup, arrange a fresh connection on the next turn while leaving ready clients alone. Avoid scheduling immediate background reconnection as a side effect of the interrupt; the user has just asked to stop current work. The same regression should turn green before integration.
