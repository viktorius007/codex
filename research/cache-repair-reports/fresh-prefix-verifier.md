PASS

# Fresh-child initial-prefix reorder verification

Target: `/private/tmp/codex-cache-fresh-prefix` at `dc8f07458aad1db2d7fe3940643b23732cba6876` plus the uncommitted two-file diff on branch `work/cache-fresh-prefix`.

Reviewed scope:

- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/tests/suite/prompt_cache_key.rs`

Both scoped files were read in full from the worktree. I also read the directly relevant world-state, fragment, reconstruction, history, compaction-callsite, and test-support source. No repository file was edited and no build or test command was run by this verifier.

## Verdict

No correctness finding remains in the reviewed diff. The production change is a 24-line local reorder inside the existing initial-context builder; it preserves fragment bytes and roles, keeps active multi-agent mode later than the usage hint, and does not change request identity, cache-key construction, world-state snapshots, persisted representations, or incremental diff logic. The strengthened integration test has a specific red discriminator and narrowly projects only response-item identity and internal transport metadata.

## Acceptance checks

### Privilege, role, and ordering: cleared

- The changed classification stores the usage-hint fragment without rendering or converting it (`core/src/session/mod.rs:4079-4107`). Both possible concrete forms remain standalone `developer` fragments: marked `MultiAgentRoleInstructions` (`core/src/context/multi_agent_role_instructions.rs:26-53`) and unmarked `MultiAgentUsageHint` (`core/src/context/multi_agent_usage_hint.rs:18-41`). The contextual-user bundle is still rendered under its original user role; no content crosses role boundaries.
- Emission is now: aggregated developer context, pre-existing separate developer context, contextual-user context, per-agent usage hint, active multi-agent mode, guardian policy when applicable, then managed developer policy (`core/src/session/mod.rs:4114-4164`). Thus stable AGENTS/global context occurs before the truthful root/child role boundary while active mode remains after the usage hint and can override it (`core/src/session/mod.rs:4127-4145`). Managed developer instructions retain their prior late position.
- Moving a developer-role item later than contextual-user material does not lower its instruction authority. The final real task remains later still, so the usage hint and active-mode policy are established before the task.

### No fragment loss and `Option` mutual exclusivity: cleared

- `build_world_state_for_step` resolves at most one usage hint and adds exactly one `MultiAgentUsageHintState` before the mode section (`core/src/session/world_state.rs:316-326`). `WorldState::add_section` rejects duplicate section IDs (`core/src/context/world_state/mod.rs:359-367`).
- That one section's full render returns exactly one of two forms: an unmarked `MultiAgentUsageHint` or a marked clone of `MultiAgentRoleInstructions` (`core/src/context/world_state/multi_agent_usage_hint.rs:32-48`). Therefore the two matching arms in the changed loop are alternative recognizers for one fragment, not two fragments competing for the single `Option`.
- I enumerated current `ContextualUserFragment` implementations with `requires_separate_message()`. The other empty-marker standalone fragments, `BaseInstructionsFragment` and `GuardianPolicy`, are assembled outside `world_state.render_full()`. Extension world-state fragments use `WorldStateContextFragment`, which retains the default `requires_separate_message() == false`. Other standalone built-in world-state developer fragments use non-empty markers and continue through their dedicated or generic paths. No current fragment can overwrite the usage hint or be dropped by this `Option`.

### Baselines, diffs, resume, and full reinjection: cleared

- The change is confined to `build_initial_context_with_world_state`; `WorldState::snapshot`, `render_diff`, `render_history_diff`, and history baseline update code are untouched (`core/src/context/world_state/mod.rs:385-425`).
- On a missing reference context, the same `WorldState` is rendered and then snapshotted and installed as the baseline (`core/src/session/mod.rs:4308-4324`). On later turns, the existing incremental `update_world_state` branch remains unchanged (`core/src/session/mod.rs:4325-4335`). Resume reconstruction continues restoring its persisted baseline independently.
- All callers of the full-context builder receive only the new initial ordering. This includes the first live turn, a resumed/cleared thread that needs full reinjection, a new context window, and compaction replacement-context construction. No caller receives different fragment content or snapshot state.

### Request identity and history isolation: cleared

- Production touches neither session/thread IDs nor `prompt_cache_key`; it only moves an already-built initial item.
- The test explicitly requests `fork_turns: "none"` (`core/tests/suite/prompt_cache_key.rs:60-66`), pins equal model, base instructions, reasoning, and a non-empty ordered tool array (`core/tests/suite/prompt_cache_key.rs:168-182`), and asserts a shared session/cache key alongside distinct thread and client-request IDs (`core/tests/suite/prompt_cache_key.rs:248-279`). The comparison does not require root and child response-item IDs to be shared.
- The corrected privacy oracle requires the child's private task as exactly one `agent_message`, and rejects the parent's private prompt and spawn call ID anywhere in the child request (`core/tests/suite/prompt_cache_key.rs:228-246`). At the child request point those are the parent-history items capable of leaking, so the assertions cover the fresh-history boundary without requiring copied parent history.

### Test projection and claim boundary: cleared

- `cache_prefix_input_projection` applies only `strip_metadata_from_json` and `strip_response_item_ids_from_json` (`core/tests/suite/prompt_cache_key.rs:49-57`). The first helper removes only `internal_chat_message_metadata_passthrough`; the second removes only `id` on JSON objects recognized as Responses API items (`core/tests/common/responses.rs:143-184`). Roles, ordered content, tools, mode instructions, and dynamic state remain visible to the oracle.
- The test finds the actual first unequal input item, proves the preceding slices are exactly equal, requires stable developer and global instructions within that prefix, and pins the unequal pair to the configured root/child developer-role fragments (`core/tests/suite/prompt_cache_key.rs:184-227`). Unique fixture strings and the non-empty-tools assertion prevent a vacuous pass.
- The source comment expressly limits the assertion to the local semantic request projection and says it does not assert backend treatment (`core/tests/suite/prompt_cache_key.rs:49-51`). Neither the test nor its assertions claim a backend cache hit or cached-token outcome.

### Size and rebase cost: cleared

- Diff size is 145 insertions and 15 deletions across two files; production is 16 insertions and 8 deletions. There is no new API, helper type, serialization field, persistence format, or dependency. The production edit is contained in one existing match/emission block, which is easy to carry or drop during an upstream rebase.
- `git diff --check -- codex-rs/core/src/session/mod.rs codex-rs/core/tests/suite/prompt_cache_key.rs` passed.

## Regression evidence

- Proper RED: `/private/tmp/codex-cache-run/lifecycle-prefix-and-timing-2.log:205` reaches the intended assertion after model, base-instruction, reasoning, and ordered-tool equality checks. It fails because the old ordering's shared prefix contains the common developer/permissions message but not `GLOBAL_INSTRUCTIONS`; this directly discriminates the reorder.
- Post-production evidence: `/private/tmp/codex-cache-run/prefix-and-lifecycle-green.log` ran the production reorder and reached the later child-task assertion, so the prefix, role-boundary, request-field, and global-instruction assertions all passed. That run's remaining failure came from treating the fresh task as a user-role message. The current reviewed test corrects the oracle to the actual `agent_message` wire item (`core/tests/suite/prompt_cache_key.rs:228-237`).

## Limitations

- The current corrected test source has not yet been executed with the production patch. The latest post-patch run passed through the target prefix assertions but predates the small `agent_message` oracle correction and therefore is not a final green for the exact reviewed tree. This verifier was explicitly prohibited from running builds or tests.
- The test proves equality of client-constructed request fields after the two documented semantic exclusions. It intentionally does not prove how a remote backend fingerprints those fields, a cache hit, or cached-token accounting.
- Real sessions may truthfully diverge before the role hint when dynamic state differs, including token-budget window/agent identifiers, environment state, or extension contributions. The controlled fixture disables token-budget context and proves the stable-prefix invariant only when those earlier request inputs are equal.
- Codebase-memory `trace_path` found 48 inbound paths within depth two for the initial-context builder, including first-turn, new-window, compaction, resume tests, and prompt-debug flows. `check_index_coverage` returned `no_recorded_issue` with matching metadata for all 15 relied-on paths. This graph signal is best-effort; every conclusion above was also checked in worktree source.

## Cleared files

- `codex-rs/core/src/session/mod.rs` — cleared; no severity finding.
- `codex-rs/core/tests/suite/prompt_cache_key.rs` — cleared; no severity finding.

