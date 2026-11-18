# Investigation of the 17 local.14 failures

18 November 2026. Scope: independent evidence/source diagnosis only. No agents, production edits, installed-binary changes, CI investigation, upstream filing, or Rust rebuilds. **This report establishes the failed assertions and several causal constraints; final root causes remain unresolved for some families.** It must not be represented as complete causal proof for all 17.

## Evidence

Read AGENTS.md, stable-0.170.0-sync.md, installation-local14.json, validation/final-validation.json and baseline-disposition.json. Extracted and inspected the original validation archive and current source. Independently verified the archive SHA-256 against validation-manifest.json: `6dab02f43decb6b0a7d8394b431a6ddddf1edeba3e6981db71927432cfbd3d8e`.

Installed version: `0.170.0+local.14`; installed source `11a6ffe239896d7f1b66e0a2ca42301b75e6ec42`; validated source `092c3a4aa1f66c321684c81a881b7b11f36bb12c`. Final result: 24,783 executed, 24,766 passed, 17 failed, 54 skipped. Unmodified baseline: `rust-v0.170.0`, `72f97c309fb0f78b6f35cce455adf416e6a51571`.

All 17 names occur in the executed baseline failures. This excludes the local patch set as a necessary cause in this environment; it does not prove a known upstream issue, release failure, or CI failure. The baseline disposition's `remaining_local_failures` array lists only 14 names; its upstream array includes all 17 plus `inherit_connector_version`. Final-validation.json is the authoritative installed failure inventory.

Evidence line references below are archive members in [validation-evidence.tar.gz](stable-0.170.0-evidence/validation-evidence.tar.gz), principally `validation/verified-workspace.log`. The archive and existing installation/validation documents remain unchanged.

## Relevant environment

This was the Linux x86-64 T3 host, not the user's Mac. Current checks: uid 1002 agent, gid 1003, `/bin/sh` is dash, `/usr/bin/python3` points to real ELF python3.14, and bash/git/ps resolve under `/usr/bin`. Current observations do not reconstruct every historical subprocess environment.

Recorded env.sh uses scratch TMPDIR `/scratch/agent/stable170/tmp` mode 0700, umask 077, 32 Cargo jobs, scratch target/build directories, dev-small helper binaries, private GStreamer/ALSA libraries and Linux V8 overrides. Tests ran via just test/nextest, with `RUST_MIN_STACK=8388608`. Helpers/build output were removed after installation.

The final environment explicitly unsets T3's `CODEX_INTERNAL_ORIGINATOR_OVERRIDE` and the Cargo target runner. Earlier T3 identity contamination and stale cached helper routing were repaired before this run; they are not explanations for these remaining failures. Shell fixtures explicitly supply temporary HOME. A temporary Codex configuration directory alone does not prove OS HOME isolation. No full per-process environment snapshot survives.

Current home profile sources .bashrc and preserves existing TMPDIR; its POSIX kache guard was already repaired. Private profile contents were not copied. Sandbox/network prerequisites were available: these failures ran rather than taking skip paths. Network approvals use a trusted local proxy fixture; search uses a local mock Apps server, not public search.

## Findings by family

### Eight model-lab search failures

Affected suffixes under `suite::v2::web_search::model_lab_search_provider`: `critical_discovery_error`, `failed_invocation_never_replays_on_native`, `host_disables_literature_only`, `matching_query_filter_cannot_authorize_cached_navigation`, `model_with_other_supported_tools`, `work_hosted_only`, `xatu_control`, `xatu_literature_treatment`.

Expected one scoped tools/list request matching model_supported_tools; actual **two**. All fail at `app-server/tests/suite/v2/web_search.rs:949`. Examples: archived verified-workspace.log:3655 and :4465. This is discovery-request count, not number of models or duplicate search executions. The count assertion precedes later exposure/invocation/fallback assertions, so these tests do not establish whether those later behaviors are correct.

Source: session/mod.rs obtains an MCP binding each sampling step and calls model_search::resolve. `core/src/model_search.rs:378` reuses discovery only when base binding Arc identity and request metadata equality both hold. On a miss, `codex-mcp/src/model_search.rs:73` sends scoped tools/list. `codex-mcp/src/runtime.rs:554` can construct another binding when catalog revisions cannot be reused. Changed identity, changed metadata or another turn can therefore cause additional discovery. The archive does not identify which predicate or publication event caused the second request.

Confidence: high in excess discovery; unresolved exact invalidation trigger. Potential impact: extra round trips/repeated discovery errors, not proven unauthorized search or fallback. Next causal check: observe synthetic binding identity, metadata equality, turn/step and catalog revisions for one failing case. Do not merely loosen the count assertion.

### Three network approval failures

Affected suffixes under `suite::network_approval::network_rejection_preserves_execution_and_review_outcomes`: `accepted_review`, `latest_rejection`, `pending_review`.

Expected: final rejection ends execution, releases Python's exclusive file lock, clears background terminals and preserves review outcomes. Actual: lock acquisition remains WouldBlock until the five-second guard fails, at `core/tests/suite/network_approval.rs:1495–1507`; archived verified-workspace.log:10868, :10890, :10911, repeated :11145 onwards. Later outcome assertions are not reached.

The Python fixture takes fcntl.flock, launches two local-proxy fetch threads, joins them and waits indefinitely on an Event. The accepted case waits for an HTTP 200/502 result before rejection. A held lock is stronger lifetime evidence than a possibly recycled PID; dismissing this as stale PID or generic connectivity is unjustified. It does not identify which surviving process holds an inherited descriptor.

Termination relies on process-group killing. Linux sandbox launch creates a separately grouped bubblewrap child; bubblewrap uses --new-session and PID-namespace inheritance varies. These are relevant boundaries, not proof of the root cause. The archive lacks lock-owner identity, process tree, signal result and termination-control receipt.

Confidence: high in unreleased execution lifetime; unresolved host versus product mechanism. Product risk: work remains alive after network rejection/turn completion. Required check: observe lock owner, PID/PGID/SID and signal result at rejection, synchronizing on actual cancellation rather than extending the timeout.

### One app-server command lifecycle failure

`suite::v2::command_exec::command_exec_process_ids_are_connection_scoped_and_disconnect_terminates_process` expects Python's unique argv marker to disappear when websocket one closes. Websocket two correctly receives “no active command/exec” for the same client process ID: connection scoping passes. After websocket one closes, the marker remains for five seconds. Archived verified-workspace.log:1987–2041 includes the timeout and disconnected-connection warning.

`app-server/src/command_exec.rs:388` removes that connection's sessions and sends Terminate; execution calls session.request_terminate. `utils/pty/src/process.rs:229` takes the killer once and discards the kill result; the Linux pipe killer signals its stored process group. The test helper at command_exec.rs:1427 scans `ps -axo command`, not a lock. Logs do not prove control receipt, correct target/signal, or the matching process state. Do not combine this with the network family without evidence of a common failure point.

Confidence: high in marker persistence, unresolved termination mechanism. Potential impact: command survives client disconnect. Next check: connection close -> control receipt -> killer result -> exact matching ps row, with deterministic event synchronization.

### One sandbox fixture: scratch layout lacks a write grant

`suite::sandbox::sandbox_keeps_parent_repo_discovery_while_blocking_child_metadata` fails before intended metadata assertions with `mkdir: Read-only file system`; linux-sandbox/tests/suite/sandbox.rs:1183; archived verified-workspace.log:15553 and :15603.

The fixture creates repo/sub, real-tmp, redirected-tmp and tmp-alias -> real-tmp under a temporary parent. It grants writes to repo/sub and TMPDIR through tmp-alias, then creates marker directories under sibling redirected-tmp. That sibling is outside both granted roots. The ordinary /tmp write grant covers it when tempfile defaults under /tmp, but not under recorded scratch TMPDIR. This is a concrete test-layout assumption exposed by scratch placement. Denial is consistent with the permissions; the error does not prove broken repo discovery or sandbox escape.

Confidence: high from source and error; not confirmed by controlled rerun. Surgical proof: place only this fixture under /tmp or grant only redirected-tmp, preserving all assertions.

### Four shell snapshot failures: distinct paths

`remote_sandbox` under shell_snapshot_v2_filters_profile_exports_and_stays_in_memory expects helper/profile output, empty stderr, exit 0. Actual: empty stdout, `.bashrc: line 1: .../captures: Read-only file system`, exit 41 (exec-server/tests/exec_process.rs:367; archive :13732). The fixture uses read_only on its first launch, but .bashrc writes HOME/captures and defines an exec trap exiting 41. Temporary HOME is under scratch. Capture runs the prepared sandbox and explicitly sources .bashrc (exec-server/src/shell_snapshot.rs:384; shell-command/src/startup.rs:23). The writable-profile assumption conflicts with read-only scratch HOME; fallback reaches the startup trap. High confidence in permission failure, medium in complete fallback chain without telemetry. Startup side effects can prevent capture; permission denial itself is expected.

`remote_pipe_recovery` and `remote_tty_recovery` under shell_snapshot_v2_capture_failure_falls_back_and_retries expect final captures `xx`, actual `xxxx` (exec_process.rs:1123; archive :13704 and :13798). Earlier fallback and recovered-output checks pass. The test fails a profile capture, waits past backoff, repairs the profile, then launches twice. Capture cache keys include sandbox equality; protected credential captures deliberately bypass cache (shell_snapshot.rs:115–154). No per-launch counts or protected-mode telemetry survive, so extra capture versus another profile execution is unresolved. High confidence in repeated profile side effects, not a proven failure to recover. Next check: profile PID/phase plus exact cache hit/miss events on the repaired launches.

`blocked_descriptor_path` under shell_snapshot::shell_snapshot_concurrent_replays_keep_independent_readers expects eight independent `restored:input-N` outputs, exit 7, and one startup capture. Actual: empty stdout fails first assertion (exec_process/shell_snapshot.rs:253; archive :13674). It denies /dev/fd, grants cwd and omits the broad temporary-directory write grant used in its alternate branch; it exercises environment replay. A later retry loses its executor websocket and reaches the 45-second recovery guard (:19538), a separate observed symptom. The initial assertion prints only stdout, omitting errors/status. Exact capture/replay cause remains unresolved; no proof of cross-talk or escape. Next check: preserve the complete four-field output and replay-mode event.

## Disposition and limits

Scratch temporary-root assumptions explain a specific sandbox fixture mismatch and the read-only shell HOME failure. Held execution locks and persistent command markers remain possible upstream lifecycle defects. Search has excess scoped discovery; shell recovery has excess profile side effects. The preserved evidence does not identify their final causal trigger. No upstream issue or CI conclusion is asserted.

No Rust test reruns were performed: artifacts were removed, and evidence-first inspection avoided a substantial rebuild. Thus exact causal diagnosis of all 17 remains unfinished. Follow-up should instrument and run the named focused tests through just test, not rebuild/test the full workspace or weaken assertions. Root coordinator owns CI/release gates and any fix selection.

Cleanup: extracted archive and transient instruction-read files removed after report creation; no original evidence, source, installation or unknown artifacts deleted. No build artifacts created. Home about 1.9 GB free; scratch 360 GB.

## Executed causal follow-up (18 November, in progress)

The authorized follow-up uses this prepared worktree, no agents, and an isolated `/scratch/agent/failure-investigation/target` after sourcing the checked-in validation environment. No full suite, installed binary change, publication, or production repair is authorized or performed. Commands are preserved in [failure-causal/replay.sh](failure-causal/replay.sh). Temporary diagnostic source edits will be preserved as a patch and removed.

**Search mechanism now observed:** the focused `model_lab_search_provider` replay ran 23 cases: 14 passed, nine failed (the eight original cases plus the previously known `inherit_connector_version`). Instrumentation shows the first `stable_catalog_revisions` result is `None`; the next is `Some(Ready(client identity, 0))`. The second search resolve has a different base binding pointer, pointer equality false, and request metadata equality true. Later resolves reuse that second pointer. Thus startup readiness changes binding identity and invalidates the per-turn discovery cache. This causes the second tools/list; it does not establish repeated search execution. The source predicate is `runtime.rs::binding_from_published_runtime` plus `model_search.rs::resolve`'s `Arc::ptr_eq`. No assertion has been loosened.

Lifecycle and shell focused replays are still building/running. This section is an interim checkpoint, not a completion claim.

### Lifecycle causal result

The focused lifecycle selection executed all four original cases and reproduced all four failures, including nextest retries. App-server logged receipt of `CommandControl::Terminate`. Every observed process-group SIGKILL returned 0 (success); the logged errno after success is stale and must not be interpreted as an error. This excludes failed signal delivery to the selected group as the explanation.

The selected outer groups differ from inner sandbox sessions. Example accepted-review replay: outer target PGID 2986825; surviving namespace leader PID/PGID/SID 2986832; Python PID 2986928, PGID/SID 2986832, state S, executable `/usr/bin/python3.14`, namespace PID 3. Its fd 3 still owns an exclusive FLOCK on inode 1562117. The other two first-attempt lock owners were 2986949 and 2986950; all six owners across retries were live sleeping Python processes with the exact lock descriptor. App-server targets outer group 2986723 while inner namespace leader 2986745 and Python 2986752 use group/session 2986745. These are real live descendants, not zombies, reused PIDs, or an unrelated descriptor holder.

A direct bubblewrap 0.11.1 reproducer independently establishes why killing the outer group fails on this host: bubblewrap calls `PR_SET_PDEATHSIG(SIGKILL)` successfully in its child, then execs the target. The target reads `PR_GET_PDEATHSIG == 0`. Killing the outer group succeeds, but the target remains alive and keeps stdout open. Rearming parent-death SIGKILL **after exec**, synchronized by an `armed` stdout record, makes the identical outer-group kill close stdout and leave only a zombie. No timeout was increased. This is a diagnostic experiment, not a production fix.

The host enforces `/etc/apparmor.d/bwrap-userns-restrict`: `/usr/bin/bwrap` enters `bwrap`, and its executed target transitions to `bwrap//&unpriv_bwrap`. The target's observed label matches. Linux's [AppArmor exec credential hook](https://github.com/torvalds/linux/blob/v7.0/security/apparmor/lsm.c#L890-L905) explicitly clears the parent-death signal on a confined profile transition. Combined with syscall traces, this explains the lost death signal. A scratch copy of bubblewrap cannot provide a profile-free comparison: this host denies its user-namespace UID-map setup. The profile itself was not changed.

Product implication: on this host's enforced bubblewrap/AppArmor transition, outer process-group termination does not contain the sandbox workload. This is an observed host-policy/product interaction, not evidence of a Rust kill error or an ordinary unsandboxed command failure. Source also has a synthetic-mount cleanup wrapper that can create another group; production repair selection must account for the complete wrapper chain. No production change was made. Confirmed replay namespace leaders were explicitly killed after recording their identity and lock ownership; unrelated old processes were preserved.
