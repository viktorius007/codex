STATUS: DONE
COMMIT: none
CHANGED: 0 files
OPEN: 5
BLOCKERS: none
---

# Prefix and fresh-child source scout

Scope: one read-only pass over stage 2 prefix construction and stage 4 fresh-child/completion behavior at `d691087d0d43d31b681774db2da3ab12990af103`. The saved research ledger was used as a lead, then every verdict below was checked against this worktree. No GitHub refresh was needed because the questions were answerable from current source. The codebase-memory graph was used for discovery; `check_index_coverage` reported no recorded issue and matching metadata for every cited path, including the two protocol files added late in the pass. Because that signal is best effort and indexes the root checkout, all claims were verified directly in the assigned worktree.

## Verdicts

| Area | Classification | Current evidence and consequence |
|---|---|---|
| Shared host aliases | Already correct, but incomplete | `ext/skills/src/host_aliases.rs:8-40` chooses shared roots deterministically and `render.rs:1048-1074` builds aliases from authority-homogeneous entries. This shortens the catalog and removes most machine-specific path text. |
| Plugin cache version in the visible locator | **Evidenced defect** | `host_aliases.rs:56-67` deliberately retains `<plugin>/<version>`. `render.rs:1076-1084` renders Host entries from their path. `render_tests.rs:807-879` locks in `r0/github/hash123/...` and `r0/slack/hash456/...`. A curated-cache refresh can therefore change the early prompt even when the skill text or selected plugin set did not change. |
| Stable plugin identity/resolution | Partly present; model-facing support unsupported | The loader already records `plugin_id` and `remote_plugin_id` (`loader/host.rs:340-387`), and the shared plugin types retain identity plus plugin root (`utils/plugins/src/lib.rs`). The Host catalog mapping discards that identity and uses the absolute SKILL.md path as package/resource ID (`provider/host.rs:89-151`). `skills.read` only searches Orchestrator and Executor catalogs (`tools/read.rs:82-108`; `tools/mod.rs:206-289`), so simply printing a virtual stable locator would make it unreadable. |
| Fresh child isolation | Already correct | A spawn with no fork history takes the `InitialHistory::New` path (`core/src/agent/control/spawn.rs:603-714`), while the child config inherits the parent's effective model/provider/reasoning and base instructions (`tools/handlers/multi_agents_common.rs:170-220`). Existing coverage also proves the fresh child omits the parent's seed prompt and receives creation-time global instructions once (`tests/suite/agents_md.rs:1560-1601`). |
| Cache routing identity | Already correct | A fresh child gets its own thread ID but shares the root session ID (`session/session.rs:761-817`). The Responses client uses that session ID as the default JSON `prompt_cache_key` (`client.rs:504-516`). `tests/suite/prompt_cache_key.rs:110-159` proves root and child have different thread/client-request IDs and the same session ID/cache key. Distinct thread IDs are intentional transport identity and must remain distinct. |
| Full parent/fresh-child prefix reuse | **Unsupported, not a demonstrated defect** | No test compares the ordered cache-relevant prefix of the complete parent and fresh MultiAgentV2 child requests. Initial context is assembled from stable developer/thread/turn contributors followed by child-specific identity/world-state material (`session/mod.rs:3932-4162`; `session/world_state.rs:122-139,210-248,296-325`). Items are stamped with per-turn metadata; whether the backend excludes all such internal metadata from cache matching is not established by local tests (`protocol/src/models.rs:928-975,1297-1309`). Same cache key is necessary, not proof of cached-token reuse. |
| V2 active wait | Already correct | The V2 wait subscribes to mailbox activity and returns on notification (`tools/handlers/multi_agents_v2/wait.rs:67-96`); it does not poll child state on a fixed interval. The input queue wakes that subscription when mail arrives (`session/input_queue.rs:124-149`). Existing integration coverage proves the active wait wakes and completion forwarding is one message per child turn (`tools/handlers/multi_agents_tests.rs:1757-1910` and the nearby mailbox-wake test). |
| V2 completion while the parent is idle | **Evidenced defect** | Child terminal turns are forwarded to the direct parent (`session/mod.rs:2151-2298`), but the completion is created with `trigger_turn: false` at 2276-2282. The receiver schedules work only for trigger-turn mail or an outstanding durable sleep (`session/handlers.rs:79-95`). Tests explicitly deliver the queued completion only after a later user turn (`tests/suite/subagent_notifications.rs:928-955`) and preserve queue-only mail after an answer boundary (`session/tests.rs:11974-12019`). This violates the repository rule that terminal status must wake or continue the parent without timeout polling. |
| Completion size | **Evidenced defect** | The stated 1,000-token cap exists, but only error text is truncated. A successful `AgentStatus::Completed(Some(message))` is cloned in full (`core/src/session_prefix.rs:9-35`), then the completion fragment renders the full payload (`context/inter_agent_completion_message.rs:13-45`). The only cap test covers an error (`session_prefix_tests.rs:9-20`). |
| Legacy V1 completion + wait | **Evidenced duplicate-delivery seam** | The detached watcher subscribes to final status and injects a `SubagentNotification` (`agent/control.rs:620-712`). The V1 wait independently subscribes to the same final status and returns it in its tool output (`tools/handlers/multi_agents/wait.rs:120-204`). There is no shared delivery receipt, so an active wait can make the same completion visible through both paths; an idle parent still gets no scheduled continuation. The exact race lacks an end-to-end regression, so the implementation fix must first pin it down with that test. |

## Prioritized closed worklist

### 1. Make plugin-backed Host locators independent of the cache directory version

**Smallest direct mechanism:** preserve the loader's plugin identity and the SKILL.md path relative to its plugin root when constructing a Host catalog entry. Render a stable package/resource identifier from those semantic fields for plugin-backed entries while leaving ordinary filesystem skills unchanged. Extend the existing Host provider/read path and the `skills.read` selector just enough to resolve that stable identifier through the turn's active immutable Host snapshot. Keep the versioned absolute path private as the actual read target. Do not merely strip the version from the printed path: the resulting locator would not resolve. Do not change cache retention or deletion.

**Regression seam:** construct the same selected plugin under two cache-version directories. Assert byte-identical rendered catalog/root lines, then refresh the Host snapshot and prove the same stable package reads the active version's SKILL.md. Also assert two different marketplaces/plugins with the same skill name do not collide and a normal local filesystem skill keeps its filesystem locator.

**Ownership/dependencies:** one `codex-skills-extension` commit, principally `catalog`, Host loader/provider, renderer, tool context/read, and their existing sibling/integration tests. It is independent of core completion work. The final whole-request cache check in item 4 depends on it. Avoid a `core-plugins` cache-layout migration unless the stable snapshot resolver proves impossible; current types already carry the needed identity.

### 2. Enforce the 1,000-token completion cap for success as well as failure

**Smallest direct mechanism:** truncate the successful payload inside the shared `format_inter_agent_completion_message` path, reserving space for the envelope exactly as the error path does. Route any legacy completion fragment through the same bounded representation or apply the identical cap before `SubagentNotification` is injected. This is an independent safety/cost fix and should land before scheduling can automatically start more parent work.

**Regression seam:** use a long successful final answer, assert the complete rendered envelope remains below 1,000 approximate tokens, retains an explicit truncation marker, and is delivered once. Keep the existing long-error check.

**Ownership/dependencies:** one small `codex-core` commit in `session_prefix.rs`, its sibling tests, and the narrow legacy call site if needed. No dependency on plugin work.

### 3. Let one V2 terminal completion continue an idle parent, without duplicating an active wait

**Smallest direct mechanism:** send the existing completion mailbox item through the existing trigger-turn scheduler. The likely local change is a completion-specific scheduling flag or `trigger_turn: true`; retain one mailbox item as the single source of truth. Do not add a timer or a polling loop. Preserve the current event-driven active-wait wakeup and coalescing behavior.

**Regression seams:**

- Child completes after the parent's wait timed out or after the parent answer boundary: without another user request, exactly one new parent sampling request starts and contains exactly one completion.
- Child completes while a V2 wait is active: the wait returns early and there is exactly one continuation/delivery, with no second automatic turn.
- Two children finish together: one scheduled parent continuation may contain both bounded completion items, each once.

**Ownership/dependencies:** one `codex-core` V2 session/input-queue commit plus integration tests beside `subagent_notifications.rs` and existing multi-agent tests. Depend on item 2 so auto-continuation cannot inject an unbounded success. Coordinate with the compaction scout for post-compaction delivery assertions, but do not pull compaction changes into this commit.

### 4. Add a deterministic fresh-child prefix invariant before changing construction order

**Smallest direct mechanism:** add an integration test that captures the root Responses request and a `fork_turns: "none"` MultiAgentV2 child request under the same model, reasoning settings, tools, skills, and selected plugins. Compare `instructions`, session/cache key, ordered tool definitions, and an explicit cache-relevant projection of ordered input. Assert a long stable common prefix through the startup catalogs/instructions and identify the first deliberate child-only boundary. Assert parent conversation history is absent from the child. Run the same assertion for sequential and concurrent fresh children.

Only if this discriminator finds an early client-controlled mismatch should production code move child-specific developer/world-state fragments after the stable catalog. Preserve truthful differences: root/subagent usage hints, agent path/token budget, explicitly requested role/model/reasoning/tool changes, child task input, thread ID, and client request ID. A paced real-provider validation should separately record cached-token counts; that backend result cannot be made a deterministic local unit test.

**Ownership/dependencies:** one `codex-core` integration-test commit first. It depends on item 1 for stable plugin locators and on the diagnostic sidecar work for useful first-mismatch output. It should consume, rather than duplicate, the other scout's canonical tool-inventory projection. Any production reorder is a separate follow-up commit whose scope is determined by the failed assertion.

### 5. Give legacy V1 wait and watcher a single completion receipt

**Smallest direct mechanism:** introduce one receipt keyed by child plus terminal turn/generation. The V1 wait and detached watcher atomically claim that receipt. If the wait claims it, the tool result is the delivery; if no active wait claims it before completion, the watcher queues the bounded completion and schedules one continuation. Use the same receipt for the V2 terminal watcher fallback if a shutdown path can otherwise repeat a per-turn completion. Avoid checking “is a wait active” without an atomic claim because that leaves the race intact.

**Regression seams:** cover completion before subscription, during active wait, just after wait timeout, and on detached watcher fallback. Each case must produce one model-visible completion and, when idle, one parent continuation. Preserve the intentional behavior that a follow-up child turn can produce a new completion and that an interrupted nonterminal child does not report a final answer.

**Ownership/dependencies:** separate legacy `codex-core` commit in `agent/control`, V1 wait, and integration tests. Depend on item 2's bounded formatter. Keep it after the normal V2 path because V2 is the active mechanism and has the higher payoff.

## Validation handoff

No builds or tests were run in this scout. For implementation, the narrow lanes are `just test -p codex-skills-extension` and focused `codex-core` integration tests, followed by `just test -p codex-core`, `just fmt`, and scoped `just fix -p ...` under the repository rules. The coordinator owns the authorized full suite, installed-binary validation, durable diagnostic run ledger, and cleanup. This pass created only this report; it did not touch the source checkout or diagnostic-retention policy.
