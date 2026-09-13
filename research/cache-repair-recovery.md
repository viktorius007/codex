# Cache repair recovery checkpoint

The run is incomplete. Do not install an unvalidated binary.

RECOVERED: reading final results of already-completed exec sessions through write_stdin released retained resources and immediately restored exec. This was cleanup after completion notification, not passive polling. Reap each completed background session once until the local completion lifecycle is corrected. Initial regression log then showed all three intended behavioral tests failed.

The shared tool harness developed persistent EMFILE (Too many open files) after the baseline core suite and initial regression run. All agents' exec and most apply_patch operations failed before executing. Terminal UI access is prohibited by the computer-use tool; Activity Monitor inspection timed out. No running Rust command was killed, no diagnostics deleted, and no binary installed.

Root branch local/customizations is at fdca0be4f4 (three durable research/policy commits beyond ea55207054). Main was fast-forwarded to origin/main 36f0dbe796 without rebasing local/customizations away from stable 0.154.0.

## Preserved work

- /private/tmp/codex-cache-compact-parity, work/issue-37305: test edit in core/tests/suite/compact.rs plus production edits in core/src/compact.rs and core/src/session/turn.rs. Builder report /private/tmp/codex-cache-run/compact-parity-builder.md. Uncommitted, green gate pending.
- /private/tmp/codex-cache-usage-boundary, work/issue-35935: two test edits in core/src/context_manager/history_tests.rs and core/src/session/rollout_reconstruction_tests.rs. No production edits. Report usage-boundary-tests.md. Builder design remains in conversation handoff.
- /private/tmp/codex-cache-parent-completion, work/issue-37299: tests edited in core/tests/suite/subagent_notifications.rs and core/src/session_prefix_tests.rs; formatted. Report could not be written. Tests: successful_completion_message_is_bounded_and_marks_truncation; multi_agent_v2_terminal_child_wakes_idle_parent_once; plaintext_multi_agent_v2_completion_during_wait_is_delivered_once. No production fix yet.
- /private/tmp/codex-cache-integration, work/cache-integration: coordinator-only target (~11 GiB); contains test-only patches for compaction parity and usage boundary. Both patches are also in /private/tmp/codex-cache-run. Never overwrite these edits unknowingly.
- Diagnostic design/interface reports exist under /private/tmp/codex-cache-run; initial design is already committed under research/cache-repair-reports. Revised design uses immutable per-run files plus offline comparison (no runtime pointers/LRU/locks, no automatic evidence deletion). diagnostic-records-tests.md contains a proposed test/interface handoff, NOT actual tests. Record, transport and analysis worktrees exist but no code landed there.
- Skill locator worktree exists, no edits. Source scout report is committed.

## Executed validation

- Baseline exec-completion preservation regression: PASS, 1 test, 4184 skipped; build 2m58s. Log /private/tmp/codex-cache-run/core-build-baseline.log.
- Baseline core suite: FAIL, 4161 run, 3941 passed (1 leaky), 220 failed, 24 skipped. Log core-tests-baseline.log. Missing test_stdio_server, codex, and codex-code-mode-host explain many failures; remaining failures require rerun after prerequisites. No cache production fixes were present during this run.
- Initial regression-only run exited100. Log initial-regressions-red.log. Its contents have NOT been read because EMFILE began; do not claim the expected behavioral reds were observed.
- Baseline scanner Python3.12 unit tests passed; log scanner-tests.log.
- Bazelisk installed successfully through Homebrew, required for eventual dependency lock refresh.
- Last measured free disk before harness failure: 50 GiB. No final cleanup check could execute after EMFILE.

## Resume next

1. Restore shell/file-tool execution without discarding work; diagnose the running harness descriptor limit/leak. All agents have checkpointed/ended; no continued tool retry loops needed.
2. Inspect initial-regressions-red.log and materialize missing agent reports from conversation if needed.
3. Build prerequisite binaries from the integration worktree with its own target: cargo build -p codex-rmcp-client --bin test_stdio_server; cargo build -p codex-code-mode-host --bin codex-code-mode-host; cargo build -p codex-cli --bin codex. Run just test -p codex-core again. Do not compile from a different worktree into this target.
4. Remaining baseline failures with no binary-locator evidence: hook_runtime::tests::hook_lifecycle_notifications_hide_builtin_and_async_runs_but_preserve_metrics; session::tests::extension_metrics_preserve_session_metadata_tags; session::tests::world_state_extension_metrics_follow_turn_model_switch; suite::client_websockets::responses_websocket_emits_websocket_telemetry_events; suite::client_websockets::responses_websocket_includes_timing_metrics_header_when_runtime_metrics_enabled; suite::exec::openpty_works_under_real_exec_seatbelt_path.
5. Resume diagnostics test-writer/builder stages and root-cause patches. Use Sol for load-bearing work, fresh context, isolated worktrees, coordinator-only heavy builds. User asleep; full tests/install authorized; adapt priorities within stages1-5.
6. Integrate small green commits, independent verification, full authorized tests, version/lock bump, paired binary build/install with rollback and exact provenance. Cleanup all agent-created worktrees/targets only after verified integration; preserve diagnostics and unknown/user data.

## Cleanup ownership

Coordinator owns all /private/tmp/codex-cache-* worktrees and /private/tmp/codex-cache-run. Preserve reports, patches, and uncommitted work. Also remove agent-created /private/tmp/codex-parent-completion-uv-cache when safe. Completed scout/readiness worktrees were already removed; design/policy/integration/fix worktrees remain. No evidence may be automatically deleted.

## Restarted continuation (current)

Inherited shell soft limit4096, hard unlimited; commands work. Last disk39GiB. Prerequisite build succeeded using checksum-verified Codex V8 archive+binding via existing scripts/codex_package/v8.py (default upstream download404). Env /private/tmp/codex-cache-run/v8-env.json; runner run-check.py applies this to all coordinator-only target commands. Matching codex, codex-code-mode-host, test_stdio_server built but NOT installed.

Integration has compaction fix and its regression: targeted PASS after baseline red. New parent tests baseline: success bound FAIL, idle wake FAIL (timeout), active wait3 cases PASS. Root final-envelope cap patch applied to integration; full core check running with only3 intentionally red usage/idle tests excluded. Log core-prerequisites-complete.log, exec8872. After each real background completion, consume final result once as resource cleanup, no polling.

Fresh active roles: records_builder_recovered owns new record crate+manifests; transport_tests_recovered owns API observer tests; usage_builder_recovered owns usage boundary production; wake_builder_recovered owns wake production (root cap+test edits frozen); locator_tests_recovered owns skill-locator tests. All in their previously assigned isolated worktrees, no agent Rust builds. Root fixed4 small pending record test issues before builder dispatch. Partial old reports remain historical; current role reports use matching recovered names in /private/tmp/codex-cache-run.

## Integration checkpoint after prerequisite-complete core run

Core run completed: 4,163 tests, 4,156 passed, seven failed, 27 skipped. Three intentionally red pending usage/idle tests were excluded. Six remaining failures match the pre-repair baseline (five missing metrics, one macOS Python warning); Sol task remaining_core_failures owns read-only triage. The seventh is the new completion cap: shared truncation appends its marker outside the requested budget. Coordinator reserved 32 approximate tokens for that marker and applied the reviewed one-line V2 idle wake change in integration. Targeted parent tests completed (parent-completion-green.log): cap and three active-wait cases PASS, idle wake remains FAIL after trigger_turn=true. The scheduling agent is investigating the remaining mechanism; do not treat the one-line edit as sufficient. Exec65692 output was consumed after its completion notification. The agent wake edit was rejected by auto-review for delegated authorization context; root reviewed the proposed edit against explicit user scope and applied it normally.

Compaction targeted red/green is complete; fresh compaction_verifier owns independent review. Other recovered implementation/test agents continue in their assigned worktrees. Root checkout is now dc8f07458a plus this checkpoint. Last disk33GiB free; integration target24GiB. No installed binary has changed. Final full suite, diagnostics integration, version/lock updates, matched installation, and cleanup remain required.

## Follow-on integration checkpoint

Compaction independent review PASS (compaction-verifier.md); small commit awaits remaining integration gates. Usage production applied: 107/108 surrounding history/replay tests pass, resumed boundary test fails 269 versus209 (usage-boundary-green-2.log); builder and verifier are correcting/reviewing. Root migrated two existing private-helper test calls mechanically. Dead token-info setters remain under review.

Idle-wake follow-up identified fixture over-counting: submit_turn already consumes the initial completion, so root changed the test to wait for one additional completion. Production trigger_turn=true remains. Corrected targeted gate pending.

Remaining core failure triage: five telemetry failures come from rejecting '+' in required local SemVer version tag. Root strengthened existing metric and sanitizer fixtures; red run exec25531, semver-metrics-red.log. A narrow matching validator/sanitizer correction follows. The sixth failure is exact Apple Python dirhelper warning despite successful PTY round-trip; no sandbox-policy expansion approved or applied.

Stage T tests are finished; fresh transport_builder implements API hooks in existing isolated worktree. Stable locator baseline failed both tests (skill-locators-red.log), including ordinary-host guard; test author is correcting fixture isolation while locator_builder owns production. Pending-input compaction test author owns new /private/tmp/codex-cache-pending-input branch work/cache-pending-input. No builds in agent worktrees; root integration target remains sole build owner. Diagnostic retention remains unlimited, binary unchanged.

## Diagnostics integration and current gates

Stage R record crate is integrated, all7 tests PASS twice (records-green.log, records-green-2.log). Independent records review cleared privacy, HMAC, bounded manifests, failsoft and no deletion; root corrected missing keyScope and dated YYYY/MM/DD archives, with regression assertions. KeyScope is a Fingerprint object on request/outcome, HMAC domain installation.keyScope. Added existing workspace chrono dependency; Bazel lock generation still required. Stage R size needs coherent two-commit staging per records-verifier.md.

Stage T API hooks integrated: initial crate run181/182PASS, new WS test fixture lacked existing compression configuration. Root matched core's WebSocket helper config; both exact transport tests PASS (transport-green-2.log). Transport source otherwise unchanged. Fresh core_diagnostics_builder owns runtime wiring in /private/tmp/codex-cache-diagnostics-core (prerequisite R/T copied; frozen3tests authored). Explicit warmup category/lifecycle coverage remains after first inference slice. Fresh diagnostics_analysis_tests owns synthetic offline comparison tests in /private/tmp/codex-cache-diagnostics-analysis; schema keyScope provided.

Stable plugin locator production is integrated and new2tests PASS. Pure filesystem prompt regressions corrected by dedicated plugin prompt kind. Root isolated two pre-existing host_service fixtures with empty config+explicit extra root; full skills crate now175/176PASS (locator-green-2.log). Remaining symlink test actual plugin_id Some versus expectedNone: fixture priority must be preserved, not assertion weakened. Need finish isolation correctly. Stale .render_tests.rs.pending-snap is our test artifact; original pure-host snapshot now passes, remove pending file when verified.

Telemetry SemVer fix:80/80 affected crate tests PASS, five formerly failing core telemetry tests PASS. Root PTY test now asserts exit_code0 and tolerates only exact Apple Python launcher dirhelper warning; PASS. No sandbox production/environment changes.

Usage paired-state patch remains failing modern resume269vs209. Five production files integrated, old test setter calls migrated. Independent verifier also requires legacy/post-compaction base-inclusive estimate, invalidation and reasoning coverage. Builder has temporary numeric-only cfg(test) traces in three production files; integrated and running exec53727, usage-boundary-instrumented.log. MUST REMOVE those temporary diagnostics after mechanism identified.

Wake production trigger_turn=true remains. Fixed consumed-completion count, but child mock overlap persists with parent_turn_id because root automatic turn can carry it. Root now uses exact x-openai-subagent=collab_spawn matcher; latest targeted green pending. Keep global request count4. Pending-input projection production+2tests integrated; original two tests failed before fix in wake-and-compaction-threshold.log. Need validate with corrected usage mechanism.

Root latest commit remains dc8f07458a; all production changes uncommitted in integration/issue worktrees pending full green+lint, then split small commits. Last cleanup removed only integration target/debug/incremental when disk20GiB; recovered to33GiB. No new binary installed. Existing target/deps/binaries retained, evidence never deleted.

## Latest continuation checkpoint

Modern resume fixture corrected to include hydration's content_item_kinds=unknown metadata; focused modern regression now PASS (combined-mechanisms-green.log). Temporary numeric traces removed. Usage replay fallback now recomputes prepared history plus current base instructions when no positioned TokenUsageRecord survives; it does not invent a TokenCount boundary. Local recompute estimates history and installs the paired count/cut under one state lock. Independent verifier cleared source after catching and closing the intermediate snapshot/current-cut race; final regression gates pending. New isolated usage-regressions worktree/agent owns reset/removal, legacy/postcompaction and encrypted-reasoning coverage.

Pending-input integration test now PASS. Its usage-less-tail companion was accidentally using ev_completed, which supplies explicit zero usage. Root corrected only that completion to omit usage; usage-and-pending-green.log (exec92299) is running with history/replay tests. Idle-wake still fails child recorder count despite exact header matcher; wake agent now inspects recorder semantics, no production conclusions from faulty fixture.

Stable locator full crate now PASS176/176 (locator-green-3.log). Root integrated test-only relocation of symlink User/plugin precedence to existing private resolver/home-dir seam, retaining exact plugin_id=None oracle. Removed owned stale render pending snapshot. Independent locator verifier active.

Analyzer production agent active with14 frozen tests; root corrected equality fixture to use identical wire manifest (previously it falsely expected equality despite differing exact-wire hashes). Runtime core builder active; StageR/T prior tests still green. No production commits or binary installation yet. Last disk19GiB after removing owned inactive incremental cache; preserve all diagnostics/evidence. Current root remains dc8f07458a plus this ledger. New worktree /private/tmp/codex-cache-usage-regressions belongs to root cleanup.

## Runtime first-slice and edge gates

First core diagnostic runtime integrated. Initial compile failed because root omitted changed untracked codex-api/request_observer.rs from copy; copied matching source and core-diagnostics-green-2.log PASS56/56 (1 leaky), including all3 new HTTP/reusedWS/unavailable-directory diagnostics tests and corrected idle-parent-wake regression. Filter also matched unrelated records-named tests; it did NOT execute all recordcrate tests, so full crate gate is now running. StageRextended wire.transport allowlist and signedi64/missingusage reviewed source-safe; root fixed TransportManifest visibility warning. Fresh transport_record_tests agent owns2focused schema/privacy/usage edge regressions in recordsworktree.

Usage extra gates PASS10/10 (usage-extra-and-transport-green.log):8 new accounting executions (reset/removal, reasoningbothsources, forcedfullbothflags, legacy/postcompactionbase-inclusivefallback) plus2APIobserver tests afterruntimechanges. Full core+API+record suites running integrated-core-api-records.log exec14169. No installedbinarychange.

Analyzer14tests PASS after production listcomparison fix (aggregatehash equality must not suppress retainedleaf/countdifferences). Root copied scripts intointegration. Independent analysis_verifier active, testauthor reported emittedschema edge drift requiring followup: transport incremental flag, nulltimestamps/outcomes and realopaquevalues. No backendfault/causationclaims.

Locator independentreview caught advertisedreferencefiles unsupported; builder fixed snapshot-bound relative resource reads with lexical/canonical containment; testauthor added distinctrevision reference and traversal/symlinkregressions. Latestprovider/tests notyetintegrated pending currentRustcommand. Original176PASS still firstsliceonly. Locator verifier re-review active.

Core lifecyclebuilder continues explicitwarmup, legacy/v2compaction, caller-owned retryordinal acrossactualsends; failedWS thenHTTP ordinal+transport are truthful fallback evidence, no fabricatedextra request for handshake426. Firstslice saved core-diagnostics-first-slice.patch/.md. Newisolated lifecycle testworktree /private/tmp/codex-cache-diagnostics-lifecycle-tests, writerowns tests. Newfreshprefixworktree /private/tmp/codex-cache-fresh-prefix withagent testing stableparent/freshchildprefix withoutforcingthreadidentity. Pendingprojection independentreview PASS; worldstate/hookprojection retained asboundedParetofollowup. Lastdisk12GiB; target~30GiB. Consider existing dev-small profile for finalversion/fullsuite to keep diskbounded; no policycodechange yet. Preserveall diagnostic evidence, only ownedbuildcaches cleanup.

## Final validation profile and current corrections

Full affected run finished integrated-core-api-records.log:4368run,4350pass(1flaky),18fail,24skip. Source fixes from that gate: replayfallback now keeps persisted TokenUsageInfo UI unchanged and installs separate fullactive AcceptedTokenUsage; BodyAfterPrefix with no prefill uses active projected total asbaseline, notstoredtotal. Updated two bounded fixtures to retain their intended threshold crossings. ExistingUI+legacy/postcompact+BodyAfterPrefix gates nowPASS (prefix-and-lifecycle-green.log14/15). Stale capturedcompletion test nowexpects trigger_turn=true. Wake-related fixture adaptations inprogress: wake_fixture_tests owns service-tier/cyber innew /private/tmp/codex-cache-wake-fixtures; residency_fixture_tests owns agent_execution/guardian innew /private/tmp/codex-cache-residency-fixtures; peer_fixture_tests owns onlypeerfollowup function inexistingparentworktree. Sourcewakeaudit foundno scheduler/securitydefect; preserveoracles.

Timingdiscriminator with4testthreads:all4MCPcasesPASS(oneflakyretry), bothparalleltooltimingcasesPASS, under lifecycle-prefix-and-timing-2.log. Use4testthreads for broadfinalgate, keepCargo32/incremental1. No timingoracleweakened.

Diagnostic lifecycleproduction integrated includingwarmup, legacy/v2compaction, outerretryseries, truthfulfailedWS->HTTP sequence. Rootadded localResponsescompaction sharedsequencer afterverifiercaughtordinalreset; rootremovedunusedcrate-privatecompactwrapper andupdatedoneclienttestordinalarg. SourceverifierfollowupPASS. Records+API full191/191PASS. All7corediagnosticscurrenttestsPASS inprefix-and-lifecycle-green.log afterrootcorrectedindependentnormalrequest retryordinals to[0,0]; realfailedretry0,1testPASS. Newlocalcompactretrytestready inlifecycleworktree, NOTYETcopiedafterlastrootrun. Rootfixederroneousdoublemodule registration (lifecyclefile isprivatechild ofcache_diagnostics, notsuitepeer).

Freshprefix discriminatorfinallyproperRED atGLOBALprefixassert afterfixturecorrections: submit_text_turn preservesHigh; modelInfo overrideV2 grantsbothroot/childsamecollaborationtools. Production16+/8- initial-only reorderintegrated;sharedGLOBAL andexactroleboundary assertionsnowPASS. Lastremainingtestfailurewasusinguser-texthelper forV2childtask; rootchangedtoexactagent_message contentarray+wholechildbodyparentprivateabsence inintegration+freshworktree, greenpending. Freshprefixindependentrevieweractive; Optioncapturemutualexclusivityproved byworldstateone-section/renderonefragmentinvariant.

Analyzer19testsPASS andindependentfollowupPASS (all5reporteddefectsfixed). Latest881lineCLI and887linetest copiedinto integration beforefinalsourcefreeze; documentedcoherentstandaloneexception acceptedbyreviewer, no cosmeticcompression. Supportsrealtransportidentity,strictkeyScope,nullabletime/missingusage,boundedSQLints,prioritytierambiguity, honestnullrates+textwindows,requestKindturn/warmup/compaction/memory. Detachedmemoryclient remainsunwired/outsidecoregate; genericlowertransportretry observations remainonepreparedendpointattempt. No backendfault/causationproofclaims.

Finalartifactversionnow bumped inintegration Cargo.toml/Cargo.lock to0.154.0+local.2. Current rootcheckoutstilldc8f07458a+ledger. No productioncommits/install. Old integrationtarget beingremoved (exec8243) whileNO Rustcommandsrunning; allours,alllogs/evidence/V8inputs preserved. Finalbuild/tests MUST useexisting `dev-small` Cargo profile toavoidduplicatedlimited-debugartifacts: `run-check.py cargo build --profile dev-small ...`; `run-check.py just test --cargo-profile dev-small --test-threads 4 ...`; scopedfix canuse `--profile dev-small`. HelperstillsetsCargo32/inc1/soleintegrationtarget, DOESNOTinjectprofileautomatically. Needprebuildcodex/codex-code-mode-host/test_stdio_server innewprofilebeforefinalsuites. Bazellockrefresh, lints/fmt, fullworkspace, smallcommits, matchedinstalledpair+rollback/provenance+smoke andcleanupstillpending. No diagnosticsdeleted; removedonlyownedtargetandparentformattercache.


## Smaller-profile build checkpoint

Owned old target removal completed; local.2 codex, code-mode-host, and test_stdio_server built successfully in dev-small (local2-dev-small-prerequisites.log, 2m30s). Bazel 9 lock update completed successfully; MODULE.bazel.lock remained unchanged. Focused diagnostics/fresh-prefix/exec-completion preservation gate is running as exec47698, local2-focused.log, dev-small and4 test threads. Latest local-compaction retry regression copied from lifecycle worktree. Disk22GiB free, target12GiB; Bazel owned cache249MiB. No installation or production commit yet. Existing wake/security fixture agents continue; usage/pending reviewers asked to close corrected findings.


## Focused gates closed; full workspace started

`local2-wake-fixtures-3.log`: all 17 tests PASS. This closes fresh-child prefix, local compaction retry ordinals, service-tier changes/reload, cyber inheritance, residency permission/environment reload, Guardian authorization (five variants), and peer completion routing. Root corrected fixture compile errors (`sse` returns a String; delays require `sse_response`/`mount_response_once_match`) and redundant post-submit completion waits. Exact security and service-tier assertions remain intact. Usage and pending-projection reviewers now both PASS with no open findings.

`local2-focused.log`: existing diagnostics lifecycle and background exec preservation PASS; the two new test oracles were corrected before the final17 gate. The child private task is an encrypted_content block plus a NEW_TASK envelope, not plain user text. Client metadata on compaction retry is semantically equal but rebuilt as a randomized HashMap; its JSON member order can change exact logical/wire hashes. The retry test now compares captured semantic JSON, stable logical component fingerprints and byte counts without assuming whole-body byte equality. Ordinals0/1 and exact outcomes remain tested. Do not describe this harmless ordering observation as a proved cache root cause.

Both Python scanners passed21 tests (`final-python-scanners.log`). Required Bazel lock update succeeded with no lockfile changes. Guide created in integration `research/cache-diagnostics-guide.md`, corrected to distinguish original3762 summary from supplementary3768 saved results and explicitly acknowledge no immutable byte cohort.

Full workspace `local2-full-workspace.log` stopped before tests: missing CMake required by opusic-sys audio build. Authorized prerequisite installation is running (`install-cmake.log`, exec5237); rerun the same dev-small/4-thread full command afterward. All production remains uncommitted in integration; installed local.1 untouched. Last disk16GiB. Full suite, final lint/fmt, small commits, validated matched-pair install/provenance and cleanup remain outstanding.


## Full affected suite and workspace prerequisites

Affected suite completed: 4,552 run, 4,551 pass (two passed on retry; one leaky), one snapshot failure, 24 skipped; `local2-full-affected.log`. Sole failure was expected initial context order in `context_annotations.rs`; root reviewed exact environment-context move before usage-hint/mode and updated inline snapshot in integration and fresh-prefix worktrees. Pending generated `.context_annotations.rs.pending-snap` is owned cleanup after the updated test passes. No other production failure remains from this gate.

Full workspace prerequisites now include CMake and GStreamer1.28.7 installed successfully through Homebrew; pkg-config finds both gstreamer and gstreamer-base1.28.7. Third full attempt running exec61828, `local2-full-workspace-3.log`, dev-small/4threads. At latest build check12GiB free, own incremental cache15GiB; do not remove it while Cargo runs. Can recover that owned cache after command finishes if final Clippy/build needs space.

Diagnostics staging plan saved `diagnostics-commit-plan.md`, advisory only: its asserted missing Bazel hunk is obsolete (update passed and produced no change). Root must keep stages compilable, including cfg(test) module declarations with their actual files, and avoid artificial staging merely to meet line counts. Source/test pairing and final clean validation matter more than reconstructed transient intermediate code. No production commits/install yet.

Read-only final tool-stability audit delegated to remaining_core_failures: reconcile stage5 with source-proven existing catalog atomicity/deterministic ordering and distinguish stale-capability notification work from cache-bust mechanisms. No broad issue rediscovery or speculative feature work authorized by that dispatch.


## Stage5 audit pivot during full workspace run

Full workspace build succeeded and17,634 tests are running (`local2-full-workspace-3.log`, exec61828). Initial app-server Guardian failures delegated to residency_fixture_tests in its existing isolated worktree, ownership only app-server/tests/suite/v2/guardian_v2.rs; retain all authorization oracles and diagnose before changing fixtures.

Final Stage5 audit contradicted the earlier broad atomicity claim: Core McpHandler samples from frozen step router but prepares through Session::prepare_mcp_call, which refreshes and selects current_binding_for_call. Same-name tool could execute against N+1 instead of advertised N. remaining_core_failures is checking safe minimal fix and revocation semantics; root owns production correction. Fresh Sol step_binding_tests owns tests only in new /private/tmp/codex-cache-step-binding, work/issue-43642, based dc8f07458a. No Cargo outside integration. This is in-scope direct mechanism work, not tools/list_changed capability feature expansion. Ensure captured binding never silently crosses revision and permission narrowing still fails closed. No code correction yet; wait for focused oracle/seam. New worktree is root cleanup responsibility.


## Workspace test follow-ups

Full workspace continues through17,634 tests. New failures: four app-server Guardian variants (residency_fixture_tests diagnosing in isolated residency worktree); compacted_full_history_fork_replaces_parent_developer_instructions (fresh_prefix_builder read-only diagnosis); remote-thread-store directory assertion (root now allows independent cache-diagnostics dir while all rollout/database tripwires remain); model-verification no-warning fixture (root isolated child HOME/USERPROFILE to prevent real-home skill-budget warnings); cold-root-resume sibling registration (peer_fixture_tests owns only core/tests/suite/multi_agent_resume.rs in parent worktree). No production conclusions from these fixtures until diagnosis. Context-order snapshot now PASS in workspace log; owned pending snapshot file removed.

Stage5 safe correction source-reviewed: retain refresh_mcp_if_dirty, require captured binding.config Arc equals current published config Arc, prepare from captured binding; target catalog revision already enforces stale-call failure. This preserves owner permission narrowing and avoids overly broad whole-binding comparison. Production NOT YET applied, awaiting frozen actual-handler baseline test from step_binding_tests. Additional ordering assurance agent remaining_core_failures owns tests only in new /private/tmp/codex-cache-tool-ordering branch work/cache-tool-ordering; complete mixed serialized tool arrays with reversed inventory, no new ordering production feature. Both new worktrees belong to root cleanup.


## First complete workspace result and Stage5 fix

`local2-full-workspace-3.log`: 17,634 tests ran, 17,589 passed, 45 failed, 47 skipped, 919s. Failure groups: app-server fixtures listed above; cold-root-resume race; macOS login version regex; two Seatbelt stderr wording checks; TUI version-sensitive snapshots plus four terminal color tests; V8 POC feature-unification assertion. All voice-host prerequisites now build/test. No installed binary change.

Root integrated Guardian fixture correction and isolated child HOME/USERPROFILE (app-server-guardian-wake-fixtures.md); integrated cold-root-resume exact auto-wake turn wait (peer-resume-fixture-tests.md). Fork fixture diagnosis: truthful reconstructed child token estimate now triggers tiny100/90 context threshold. Fresh-prefix reorder itself is not at fault. Fixture now limits the parent compaction model only, then switches spawn/child to otherwise-identical272k model; exact parent compaction/child instructions assertions preserved. Local compaction drops structured agent messages, so adding an auxiliary fake child-compaction response would test unrelated summary behavior. See fresh-prefix-fork-followup.md.

Root fixed login macOS user-agent test to escape exact CARGO_PKG_VERSION (SemVer build metadata accepted); Seatbelt tests now accept exact alternate `bash: line 1: <path>: Operation not permitted` while keeping denial/status/file oracles. No sandbox implementation or environment-policy changes.

Stage5 proper RED: step-binding-red-with-controls.log proves replacement endpoint received tools/call from previously sampled step (actual RPC diff), unchanged control PASS. Root corrected test helper for structured error output and applied safe source fix in integration+step-binding worktree: prepare_mcp_call takes captured &McpBinding, publishes dirty state, checks config Arc against current_config, then captured prepare_call; handler passes step_context.mcp. Mixed complete-array ordering regression PASS; tests copied from ordering worktree to rmcp_client.rs. Independent re-review requested. Root fixed typed empty slices/control fixture structured JSON, retaining exact no-call assertions.

Current Rust exec46438 runs `final-mechanism-and-workspace-fixtures.log` for Stage5 controls/order/resume, all Guardianv2 cases, fork/remote-storage/model-verification fixtures, login version and Seatbelt tests. Other agents: diagnostics_analysis_builder read-only TUI failure triage; core_diagnostics_builder V8 diagnosis DONE. V8 artifact trust chain matches and runtime sandbox is true; local POC feature marker false under Cargo unification is stale assertion. Final full gate should explicitly add `--features codex-v8-poc/sandbox`, preserving strict runtime assertion and all checksums without changing unrelated V8 source. TUI correction pending (likely cfg(test) version0.0.0 before rendering plus explicit color capability in test fixture). No production commits yet; final full gate/fix/fmt/build/install/cleanup remain.


## Stage5 green and remaining fixture gate

`final-mechanism-and-workspace-fixtures.log`: 99 run,87 passed,12 failed. All three actual-handler MCP sampled-binding regressions PASS (unchanged authority, endpoint replacement, allowlist narrowing), as does complete mixed-tool ordering. Fork developer-instruction, remote persistence, model-verification warning isolation, local-version login, and Seatbelt wording corrections PASS. Remaining failures: eleven app-server Guardian cases (ten now reviews count0 at line1182, one timeout), plus cold-root-resume thread-not-found. Returned these to their existing isolated fixture agents; do not weaken authorization/resume oracles.

Root applied TUI fixture normalization (test version0.0.0, CaptureBackend forces color like existing VT100 test backend). Production version remains actual build metadata; no snapshot expectations changed. Full TUI running `tui-final-gate.log` exec24168. Final full workspace must add `--features codex-v8-poc/sandbox` to align the POC marker with the already sandbox-enabled unified V8 dependency; verified archive/binding unchanged. No production commits or installation yet. Disk31GiB free; diagnostic evidence untouched.


## TUI gate closed

`tui-final-gate.log`:4,314/4,314 PASS,6 skipped,69s. Independent review confirms production version unchanged and color forcing test-only. Successful Insta tests removed stale generated snapshots; root deleted the single remaining owned pending-snapshot file. Candidate matched prerequisite binaries rebuilding as exec69746 (`final-prerequisites-build.log`) with dev-small. Current free disk23GiB.

Guardian fixture agent traced pre-review failures to MCP unavailable after HOME isolation; root requested deeper check against new sampled-config guard before accepting removal of isolation. Cold-resume agent identified live-thread lookup race; root requested deterministic completion barrier instead of another one-second delay. Both remain under investigation, no weakened oracles accepted.


## Detached memory coverage closure

Final scope review identified detached memory Responses as an actual coverage gap in the requested model-request diagnostics. Root applied the existing builder seam: `ModelClient::with_cache_diagnostics(&Path)` is public, opens private collector internally/fail-soft; normal Session and detached `memories/write/runtime.rs` both attach via Codex home. No new dependency or public collector type. core_diagnostics_tests owns only memories/write/src/startup_tests.rs in lifecycle worktree for paired record/usage/privacy oracle; runtime verifier checks source. Candidate prerequisite build69746 passed BEFORE this small new production change and must be rebuilt again after final gates.

Guardian environment differential57565 currently running `guardian-final-home-differential.log`; isolated-HOME overrides removed, authorization assertions intact, final guard robustness source diagnosis still pending. No installation/production commits.


## Stage5 guard finding: do not accept Arc identity

remaining_core_failures found real false rejection: latest_mcp_desired_state creates a fresh Arc<McpConfig> during normal dirty refresh; connection set can reuse the same server/client. Whole-config Arc::ptr_eq is not a valid authority generation. Guardian request0 advertises tool then rejects; request1 succeeds. Withdraw prior conditional PASS despite three green targeted tests. Agent identifying smallest semantic/per-call authority comparison; root owns production correction. step_binding_tests extending actual-handler regression with equivalent-publication control. No commits/install until this closes. Removing HOME fixture override alone is NOT an accepted root-cause fix.

Guardian differential57565 compiled overlapping root memory API edits and stopped on stale private-method metadata; no runtime evidence. Current source builder is public/coherent and independently verified PASS. Future source edits must wait for active Cargo compilation to finish.


## Equivalent-refresh discriminator and ready final fixtures

`step-binding-equivalent-red.log` proves the Arc-identity guard rejects an equivalent runtime republish (one test failed twice at exact expected MCP result). All Cargo work is stopped. Guard correction still awaiting concrete safe existing API recommendation from remaining_core_failures. Memory test copied into integration (129insertions/8deletions); exact request/outcome, six nonzero usage fields, privacy canary. Cold-resume fixture now captures thread-created ID/Arc concurrently rather than list-then-get; copied integration. Guardian HOME isolation restored in agent worktree; ensure latest restoration copied when guard corrected.

TUI all4314passed. Prepared installer `/private/tmp/codex-cache-run/install-validated.py` (536lines) read fully by root; clean exactcommit/version + fullgreenNextestlog required, matched dev-small pair backup/singlepointer/rollback/provenance. Not executed.

Disk14GiB; measured own dev-small/incremental48GB, deps27GB. Root removing ONLY exact owned incremental directory as exec11880 while noCargo runs. Preserve binaries/deps/evidence. No productioncommits/install yet.


## Corrected execution-authority comparison integrated

Root applied the reviewer-recommended minimal guard in integration only: derive PartialEq for McpConfig/ResolvedMcpCatalog/CatalogAction; PreparedMcpCall::has_same_execution_authority compares exact server/tool, underlying RmcpClient and catalog Arcs, catalog revision, semantic config. Session refreshes, prepares captured/current calls, admits equality, and returns ONLY captured call. Equivalent allocation does not invalidate; replaced client/auth/env, catalog or effective permissions still invalidate. Added codex-mcp production files binding.rs, catalog.rs, mcp/mod.rs to final change/gate scope. remaining_core_failures reverifies including possible catalog actions ordering.

Restored Guardian isolated HOME from agent; latest deterministic sibling thread-created capture and memory usage/privacy tests present. Running66544 `final-authority-memory-resume.log`: full codex-mcp + sampled-call controls + all Guardian + coldresume + memory metadata/diagnostics test, dev-small/4threads. Do NOT edit compiled source until process completes. Incremental cleanup11880 succeeded;50GiB free afterward. No productioncommits/install. Final fullworkspace must use `--features codex-v8-poc/sandbox`, then scoped fix/fmt, coherentcommitseries/rootintegration, rebuiltmatchedpair/installprovenance and cleanup.


## Final authority gate rerun

66544 stopped at one memory-test compile error: traversal vector inferred AbsolutePathBuf then pushed PathBuf. Root corrected initial value with to_path_buf in integration/lifecycleworktree. Production derived equality compiled. Reviewer then identified ResolvedMcpCatalog action-history ordering trap; root removed CatalogAction PartialEq and replaced resolved catalog derive with explicit equality of disabled veto names and effective servers only (documented). McpConfig still derives PartialEq; call comparison unchanged.

step_binding_tests extended existing equivalent-republish control with controlled extension contributor emitting two disabled same-tier registrations in opposite order on refresh, preserving winning server configurations. Copied latest one-file test into integration. Running12426 `final-authority-memory-resume-2.log` with full codex-mcp, four sampledcall tests, all Guardian, coldresume and memory diagnostics. No compiled source edits while run active. Remaining reviewer checking equality semantics. No productioncommits/install yet.


## Authority/memory runtime evidence and stale-binary check

`final-authority-memory-resume-3.log`:316run304pass12fail. All4sampled MCP controls incl equivalent/reversedregistration PASS; all codex-mcp tests PASS; detached memory exact pairedrecords/usage/privacy PASS. Eleven Guardian failures retain pre-review MCPunavailable and coldresume stillthreadnotfound. Important suspected evidence issue: app-server harness may launch standalone codex that was last rebuilt BEFORE authority fix. Root now rebuilding39651 `final-authority-prerequisites-build.log`; residencyagent verifieslaunchpath. Do NOT infer more guard defects from stale executable.

Peer agent added precise anyhowContext to every coldresume lookup/lifecycle boundary to identify actual failure; copied latest test-onlyfile. Need run new Guardian+coldresume after39651 completed, RUST_BACKTRACE=1, beforefinalfullsuite. Disk42GiB. Installer now records unixms and before/after cutover bracket; guide explains oldrunningprocesses keepoldbinary andtimestampaloneisnotversioncohort.


## Exact failure tracing active — MUST REMOVE BEFORE FINAL

39651 matched prerequisite build PASS. Residencyagent proved appserver harness actually launches `codex-app-server`, not `codex`; mtime shows it already postdated guardfix. Stalecodex hypothesis rejected. Root added temporary fixed-name-only eprintln diagnostics to core/src/session/mcp_runtime.rs (sampled/current binding/prepare missing) and codex-mcp/src/binding.rs (30 authority dimensions incl configfieldnames). No values/prompts/IDs logged. Exact pre-instrumentation backups: `/private/tmp/codex-cache-run/mcp_runtime.before-authority-diagnosis.rs` and `binding.before-authority-diagnosis.rs`. Restore these before final production validation; preserve any deliberate subsequent fix. No source-writing agents own these paths.

Running49257 `authority-and-resume-exact-diagnosis.log`, RUST_BACKTRACE=1, two selected tests (Guardian worker_root_answer and coldresume), dev-small2threads. Latest peer fixture provides named errorcontext across lifecycle. Root must read exact diagnostics before further fixes; no more guessed timing changes. Current temporary instrumentation makes candidate non-final; no install/commits.


## Exact diagnoses obtained; temporary tracing removed

49257 completed exit100. `authority-and-resume-exact-diagnosis.log` confirms Guardian rejected at `AUTHORITY_DIAG sampled_prepare_missing`, before authority predicate, twice. Cached lazy MCP binding exposes tools with no PreparedMcpCall until server startup; frozen calls map remains empty while handler starts current server. remaining_core_failures identifying captured-connection lazy preparation seam; must preserve sampled server/schema, no current N+1 reroute.

Coldresume BOTH failures have context `flush initial worker rollout <worker-id>` ThreadNotFound, not siblinglookup. Peeragent now correcting flush ownership/lifecycle via retained worker handle. Do not blindly adddelays.

Root restored exact pre-instrumentation backups into binding.rs and session/mcp_runtime.rs; NO AUTHORITY_DIAG tracing should remain in source. CurrentCargo none. Latest commit plan `/private/tmp/codex-cache-run/final-commit-groups.md`14groupscoversallintendedpaths, but latestlazy correctionstillpending; usefinalsource/nocopytemporarytraces. No commits/install.


## Cold resume correction ready; lazy guard design simplified

Peeragent found exact lifecycle: configuredmax3 countsroot, leaving2childslots. Worker+grandchild fillslots; sibling spawn evicts completedworker and closes writer. Move required materialize+successfulflush to immediatelyafterworkerterminal/roleassertions BEFOREsibling spawn; removeinvalidpostevictionflush. Latest correctedtest copiedintegration; allidentity/durabilityassertions preserved; temporarybroaderrorcontexts removed. No Cargoactive.

LazyMCP sourceconfirmedcallsNoneexpectedforcachedtools. Reviewer initiallyproposed starting capturedoldconnection; root challenged complexity/revokedendpointsideeffects. Pending simpler safe design: prepare currentreadycall, readycapturedpath strictclient/revision/config returns captured; cachedpath comparefrozenToolInfo exact + fullconfig + captured/current serverconnectionidentity thenusecurrentreadycall (sameauthority,noN+1semanticchange). Nooldserverstartup. Reviewercheckingidentitycompleteness/ToolInfoequality/accessorbeforeimplementation. KeepcorrectedMCPcodein integration; oldissueworktreeprodstillobsoleteArcguard.


## Lazy cached authority implementation integrated

Root implemented reviewed simpler binding seam in integration: McpBinding::prepare_call_if_current(current) returns ready sampledcall understrictclient/catalog/revision/config predicate; cachedno-call path findsfrozenvisibleToolInfo, requires semanticconfig and fullserverconnectionidentity equality, compares entireToolInfo after clearingonlylive read_only_hint (matchesexistingcachedsafetyredaction), restoresfrozenToolInfo intoacceptedcurrentcall. This permits already-started SAMEauthority withoutstartingold/retiredendpoint and preventstrusthint escalation. PreparedMcpCall predicate nowprivate. Added ToolInfo PartialEq in tools.rs and crate-private connectionidentitycomparison in connection_manager.rs. Core preparescurrent thenasksfrozenbindingtoadmit. No temptracing. Reviewer sourcePASS (runtimepending).

Running90382 `final-lazy-authority-and-resume.log` same316-testgateaftercoldflushfix. step_binding_tests owns newtest-only codex-mcp/src/connection_manager_tests.rs in step-bindingworktree toprove cachedsame-schemaaccept+readOnlyNone preservation andchangedlive-schemareject. No Cargooutsideintegration. Finalcommitplanmustinclude2newMCPproductionfiles andcachedtests. No commits/install.


## Lazy gate tracing active — MUST RESTORE BEFORE FINAL

90382 ended305/316PASS,11Guardianfailuresonly; coldresume nowPASS. Root copied two newcachedMCPtests into integration connection_manager_tests.rs. Active12848 `lazy-authority-exact-gates.log`: oneGuardian worker_root_answer +2cachedschema tests. TEMPORARY fixed-name traces in binding.rs label config/identity/frozenmissing and differingconfig/ToolInfo/toolJSONfieldnames ONLY (no values); exact cleanbackup `/private/tmp/codex-cache-run/binding.before-lazy-diagnosis.rs`. Restoreafterdiagnosisbeforefinaltests. No trace in core mcp_runtime now.

Reviewer noted cached-test manager uses identityNone but same exactMcpServerConnection Arc. After12848 completes add strongerexactconnectionArc proof in has_same_server_connection_identity before requiring bothSomeidentities; no test-only API needed. This will allowmatchingfixture toreachToolInfo check; doesn'texplainGuardianyet. Agentreportsource stillrequiresprecisegatelabel evidence. No commits/install.


## Final cache-contract normalization; traces removed

12848 provedGuardianconfig_equal=true/identity_equal=true and ONLYtool.annotations differed; matchingAppsunitwasblockedsolelybyidentityNone. Sourceproof: generic tool_catalog_cache publication strips ALLannotations; Apps cache preservesnonreadonlyhints andcaptureclearsonlyreadOnly. Rootrestoredcleanbindingbackup/removingALLLAZY_DIAGtraces, addedexactconnectionArcproofbeforeSomeidentityeq, andcomparisonnowsets live.annotations=None iffrozenNone elseclearsonlylive.read_only_hint. FullToolInfoeqstillpinsallothercontractfields andreturnedtoolinfoalwaysfrozen. Reviewerverifiedbothcachecontracts; Appsnonreadonlyhintsremaincompared.

Running48559 `final-cache-contracts-green.log`: fullcodex-mcp(twofreshcachedtests included),4sampledcontrols,onepreviouslyfailingGuardianworker_root_answer,memorydiagnostics,coldresume. Ifgreen nextfullworkspace with `--features codex-v8-poc/sandbox`; do NOT addmorefixturechangesunlessnewfailureevidence. No sourceeditswhileCargoactive. No commits/install. LatestcandidatebinarypairprecedesfinalMCPcachedfix; finalrebuildrequired.


## Cached call admitted; review-phase mismatch under investigation

48559 finished230/231PASS: fullMCPinclbothnewcachedtests,4actualhandlercontrols,memory,coldresumePASS. Guardianworker_root_answer nowFAILS later atguardian_v2.rs1208 expected mcp_elicitation reviewcorrelation, nottoolunavailable. KeepingfrozenannotationsNone onexecution causeddirecttoolreviewbeforeactualMCPelicitation. Root challengestheearlierassumption: genericcachecontract says annotations ONLYtrustedfromliveconnection; afteridentity+schema validation freshannotationsmaycorrectlygovernexecution asbefore. Askremaining_core_failures inspecttrusted_access/codecontract beforechangingeitherprodortest; DO NOT weakenGuardianoracle. Likelyminimalfix returncurrent aftervalidatedcomparisonwithoutoverwritingtool_info; frozenmodelcatalogremainsimmutable. NewAppsunit 'preservefrozenannotations' mayencodewrongnewassumption andmustbereframedifsourceconfirms. NoCargoactive. No temptracing. No commits/install.

## Final live execution authority gate passed

The cached-tool guard compares the sampled schema and server/configuration identity after applying the existing cache annotation redactions, then retains the verified live execution annotations. This preserves the original authorization-review phase. It does not rewrite the sampled tool catalog. Temporary AUTHORITY_DIAG and LAZY_DIAG traces have been removed. `final-live-execution-authority.log`: 318 tests passed, including all Guardian cases, full MCP tests, sampled-call controls, cold resume, and detached memory diagnostics.

The full workspace gate is running as exec session 52390: `just test --cargo-profile dev-small --test-threads 4 --features codex-v8-poc/sandbox`, with the shared run-check helper; log `/private/tmp/codex-cache-run/final-full-workspace.log`. No binary installed yet. Next: review full-suite result, scoped lint/fmt, commit staging and root mergebacks, rebuild/install validated pair with provenance, cleanup owned temporary/build artifacts while preserving evidence. Disk free at launch: 23 GiB.

Final independent Stage 5 review is PASS in `/private/tmp/codex-cache-run/tool-stability-final-audit.md`. An independent acceptance matrix is being prepared by analysis_verifier in `final-acceptance-audit.md`; no Cargo or production changes delegated. Full workspace session52390 remains the sole Cargo job. Disk fell to 13 GiB during compilation; preserve active target and remove owned incremental cache only after Cargo completes if needed. Current build cache: approximately20 GiB incremental and29 GiB deps. Commit groups corrected for lazy MCP connection identity, ToolInfo equality, tests and live execution metadata. No commits or installation yet.

Prepared commit patches without modifying the real index/source: `/private/tmp/codex-cache-run/prepare-commit-patches.py` reconstructs all90 changed files into14 logical groups from final-commit-groups.md; `/private/tmp/codex-cache-run/commit-patches/01.patch` through14.patch. All14 apply sequentially in a disposable index and every final staged blob matches candidate bytes. Regenerate after lint/format or any source correction before using them. Do not blindly commit until final gates pass. Large groups are the coherent new archive/schema+tests, transport observer+tests, lifecycle wiring+tests, analyzer+tests, usage accounting, skills, and parent wake+fixtures; inspect any further useful split without inventing intermediate public contracts. Full workspace52390 still compiling at last required disk check,12GiB free.

Full workspace52390 compilation completed and test execution began; no FAIL/TIMEOUT/error observed at the first failure-triage read. Do not infer final success until terminal summary. core_diagnostics_tests is preparing only `/private/tmp/codex-cache-run/installed-diagnostics-smoke.py`: isolated local mock-provider CLI roundtrip and sidecar privacy/usage check, preserving produced diagnostic evidence. Run after final candidate/install is ready; no external provider calls or Cargo delegated. Final full suite log remains final-full-workspace.log.

Prepared explanatory commit messages in `/private/tmp/codex-cache-run/commit-messages.json` (14 groups, subjects<=72 characters). Installed smoke script is ready and root-reviewed; root replaced TemporaryDirectory cleanup with explicit cleanup only after evidence preservation, so a move failure leaves original evidence intact. AST parse passed. Invoke `python3 /private/tmp/codex-cache-run/installed-diagnostics-smoke.py /Users/viktor/.local/bin/codex` after final install; it contacts only its localhost mock. Do not confuse preparation with an executed smoke result.

Final workspace failure triage: `cached_mcp_startup_is_eager_for_root_and_lazy_for_subagents` fails twice with echo unavailable at mcp_tool_cache.rs1091. Fixture intentionally uses MCP_TEST_DYNAMIC_SERVER_METADATA and expects cached old-PID instructions/description to refresh to new-PID live metadata while the first same-schema lazy echo succeeds. New complete ToolInfo comparison rejects that metadata change; production-vs-fixture contract judgment delegated read-only to remaining_core_failures. Root inspected the entire startup/call/result portion. Do not weaken changed-schema or endpoint/allowlist controls. No integration source edits while fullsuite52390 runs. `grandchild_full_fork_preserves_context_baseline::paginated_full_history` timed out once and passedretry; track as flake, no speculative fix.

Sol diagnosis agrees on minimal production correction after current suite finishes: in lazy `McpBinding::prepare_call_if_current` comparison clone, normalize only `live.namespace_description` and `live.tool.description` to frozen values before existing annotation normalization. Existing generic-cache lifecycle intentionally refreshes these PID-bearing presentation fields on first startup; they are not execution identity/schema/permission. Preserve original current call for live approval/reporting. Keep all other fields strict and all schema/endpoint/allowlist regressions. Existing failing integration test provides earned RED; do not change its success oracle. Final Stage5 verdict is pending this correction and rerun, superseding318-only PASS for complete readiness.

Cleanup progress during full tests: removed the clean `codex-cache-policy` worktree and obsolete `work/cache-run-policy` branch after git cherry proved all4 commits patch-equivalent on local/customizations and git diff proved identical trees. Other uncommitted mechanism worktrees and active build untouched. Copied8 completed stable mechanism reviews into research/cache-repair-reports; final MCP and acceptance verdicts remain pending the latest correction/gates.

Full workspace52390 completed exit100:17641 tests,17640 passed (5 slow,1 flaky),1 failed,47 skipped,945.502s. Sole failure cached_mcp_startup_is_eager_for_root_and_lazy_for_subagents; paginated_full_history passedretry. Applied lazy-presentation-production.patch (only normalize namespace_description and tool.description in lazy comparison). Focused MCP/core-cache/Guardian gate started; log `/private/tmp/codex-cache-run/lazy-presentation-green.log`. After green rerun full workspace, then scopedlint/fmt and delivery. No other Cargo active.

Final commit/provenance integration sequence: root dc8 differs from integration base3af only by AGENTS.md policy and durable research commits. After final gates, commit root research checkpoints, create the14 source commits in integration, then rebase integration onto the root customization HEAD and fast-forward local/customizations to integration. This preserves identical source bytes and worktree absolute paths while making the exact clean build commit reachable from the permanent branch. Rebuild in integration only after that rebase; installer commit must match this final shared HEAD. Post-install documentation can follow as a separate root commit. Do not build from the root checkout through the integration target or record a transient pre-rebase SHA.

Lazy presentation correction gate70811 exited0:322/322 passed,5641 skipped,123.912s; includes entire mcp_tool_cache, fullMCP and allappserverGuardian. Disk now22GiB free. Full workspace rerun started with identical full command/feature/profile/thread settings; log `/private/tmp/codex-cache-run/final-full-workspace-2.log`. This is the final authoritative workspace gate, superseding the previous1-failure run only after its actual terminal result.

Pre-install candidate smoke34803 exited0 using the standalone localhost mock script against target/dev-small/codex. Result log `candidate-diagnostics-smoke.log` records the preserved smoke-evidence directory. This demonstrates a real CLI model roundtrip and private request/outcome sidecars with expected usage. It is not an installed-binary verification; repeat against ~/.local/bin/codex after final rebuild/install. Fullworkspace73524 remains the sole Cargo run.

All agents are completed. Before final cleanup, preserved16 non-integration agent worktrees in private `/private/tmp/codex-cache-run/worktree-evidence-20260913T201200Z.tar.gz` (125538 bytes,SHA2568dfd3e146acfa7434918109fed1f5bdc6b58617f53bbf3dedd808b7e3327e15f). Archive contains each HEAD/branch, binary-capable tracked diff againstHEAD, untracked files, and manifest. It excludes root and active integration, build output ignored byGit, and performs no deletion. Preserve this archive with diagnostic/run evidence after cleanup.

Current fullworkspace73524 has no persistent failure as of8354/17641. Another grandchild_full_fork variant (legacy_full_history) timed out once and passedretry~0.4s, like paginated variant in precedingrun. Delegated only read-only fixture synchronization review to peer_fixture_tests; no Cargo/source writes. Preserved latest pending-projection-fullsuite-review.md and runtime-diagnostics-memory-assessment.md alongside earlier stable reports and updated the run-ledger references.

Full workspace73524 is GREEN:17641/17641 passed,47 skipped,924.704s,5 slow,1 flaky,1 leaky. Logfinal-full-workspace-2.log. Flakelegacy_full_history passedretry, peer_fixture_tests read-only reviewpending. Scoped required `just fix --profile dev-small` started as exec44942 across12 affectedcrates; log `/private/tmp/codex-cache-run/final-clippy-fix.log`. No otherCargo. Nextinspectlintedchanges,format,regeneratepatches,commit/rebase/ff,cleanexactcommitbuild/install/smoke/cleanup. Preserve real greenfullresult and disclose retry/leak qualifications rather than hidingthem.

Flake review found a transient-status fixture race: child can complete its sampled turn then immediately becomeRunning on grandchild completion; 10ms polling may never observeCompleted. Prepared `/private/tmp/codex-cache-run/grandchild-turn-completion-tests.patch`: use captured client_metadata.turn_id to await exact queued TurnComplete and assert errorNone. No timeout increase or response/oracle weakening. Apply only after Clippy44942 completes, then validate these existing fork variants before finalfmt. Source report copied toresearch/cache-repair-reports/grandchild-context-flake-diagnosis.md. The fullrunLEAK flag names pure in-memory cursor test terminal_draw_omits_cursor_style_without_an_owned_glyph; no subprocess in its inspected code, causeunproven, retainqualification.

User reinforced “assert the mechanism, not the clock”; persisted in AGENTS.md. Applied exact sampled-turn completion fixture patch after Clippy finished. Clippy44942 exited0; accepted unused body_json import removal (no calls remain), plus automated records/runtime test simplifications. Root corrected four remaining warnings without suppressions: Box only large wire transport manifest payload (same JSON), inline the sole private WebSocket connection constructor, and bundle the three skill catalog inputs through existing two functions (Sol read-only proposal). Focused regression run93452 active: all API/records/skills tests plus grandchild full-fork context variants; log final-mechanism-and-lint-regressions.log. No source commits/install yet. Full workspace evidence remains 17641 passed with one retry and one leaky label; source changes since then are lint refactoring and causal fixture synchronization. Regenerate and validate commit patches after final formatting.
