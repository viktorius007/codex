# Prompt-cache prefix stability investigation

Updated: 2026-09-14

Source snapshot: `local/customizations` at `ffabed54df`.

Repository: `openai/codex`.

Objective: locate, reproduce, and fix every preventable unintended cache bust, starting with core prefix construction. A mechanism leaves scope only after it is fixed, proven backend-only, or shown to be an intentional context change.

## Evidence boundary

- The current verdicts combine direct GitHub issue-body/comment reads with inspection of the source snapshot above. Every user-supplied issue and every additional issue retained in this ledger was opened; refresh long threads before implementation because later or non-material comments may not be represented here.
- No controlled live cache experiment has been run from this checkout. “Confirmed present” means the request-affecting code path is present, not that cached-token behavior has already been measured locally.
- GitHub's issue-search quota was exhausted before the final supplemental keyword sweep. Preserve the completed 62-issue pool and its direct reads. Future discovery is incremental only: search new terms, follow newly linked issues, and query changes since this ledger's update date. Do not repeat unchanged searches or re-read unchanged issues.
- Refresh current source before implementation. Refresh an issue only when its recorded state or update time changed, or when implementation needs a specific detail absent from this ledger. Notably, #37351 remains open despite its reported ordering mechanism appearing addressed; #25320, #36665, and #37448 are closed but remain evidence for live mechanisms or regression scenarios.
- Reported `cached_tokens` establish billing/accounting behavior, not physical KV-cache reuse. Record both request differences and reported usage.

## Working rules

- Track mechanisms, not issue numbers. Attach duplicate and symptom reports to one mechanism.
- Prioritize a mechanism only when current source still permits it and Codex can prevent it.
- Keep GitHub state separate from the current-source verdict. An open issue can already be addressed in code; a closed issue can still describe a live mechanism.
- Exclude deliberate cache changes: user-selected model or reasoning-effort changes, expiry after the documented cache lifetime, and genuinely changed tool capabilities.
- Treat a changed cache key, request prefix, tool schema/order, or hidden routing identity as a cache-bust candidate.
- Treat high token use with a stable, highly cached prefix as amplification, not a cache bust.
- Before changing code, capture two otherwise-identical requests and identify the first differing cache-relevant field.

## Local rollout baseline

The privacy-safe scanner is `scripts/audit_prompt_cache.py`. It reads rollout JSONL locally and prints token counts, timing, structural event names, tool names, and changed field names. It never prints prompt text, model output, tool arguments or output, working directories, or world-state values.

Run the summary with:

```sh
python3 scripts/audit_prompt_cache.py ~/.codex/sessions --limit 0
```

Use `--json --limit 1000` for machine-readable incident details, or pass an individual rollout path for a focused scan. The defaults flag a pair only when it is at most 30 minutes apart, the prior request had at least 1,024 cached tokens, at least 1,024 expected cached tokens were lost, and the new cached count is at most half of `min(previous cached tokens, current input tokens)`. Model and reasoning-effort changes are excluded. The 30-minute default is the current documented minimum cache lifetime for GPT-5.6+ `prompt_cache_options.ttl`; it is a conservative eligibility window, not an assertion that every cache entry expires at 30 minutes. [Official Responses API reference](https://developers.openai.com/api/reference/cli/resources/responses/methods/create)

Baseline run on 2026-09-14, while the archive was live:

| Classification | Candidates | Estimated lost cached tokens |
|---|---:|---:|
| No cache-relevant rollout change visible | 440 | 29,261,067 |
| Forked-history subagent start | 170 | 3,061,756 |
| Post-compaction request | 141 | 2,918,467 |
| World-state field changed | 6 | 287,818 |
| Turn-context field changed | 5 | 348,467 |
| Thread settings actually changed | 2 | 89,700 |
| **Fresh-context subagent start** | **2** | **45,972** |
| Local compaction request itself | 1 | 215,808 |
| Other related-thread start | 1 | 28,611 |
| **Total** | **768** | **36,257,666** |

The run scanned 3,762 rollouts (3.13 GiB) and 109,522 model-usage samples. It excluded 65 model/effort changes and 98 comparisons outside the warm window. Counts can increase as active rollouts are appended.

Two fresh-context subagent witnesses deserve early reproduction:

- At `2026-09-09T00:34:07.309Z`, a child began 0.419 seconds after a related warm request. Cached input fell from 26,112 to zero on a 22,728-token request. Every startup component visible to the scanner had the same redacted fingerprint.
- At `2026-09-09T14:21:28.899Z`, a child began 10.656 seconds after a related warm request. Cached input fell from 44,928 to zero on a 23,244-token request. Visible startup differences included `current_date`, environment/host-skill state, and multi-agent guidance. This is a concrete date/dynamic-context lead, not yet proof that any one field was the first differing provider-request byte.

The largest same-thread witness fell from 237,440 cached tokens to zero after 33.802 seconds with no visible rollout change. These invisible cases are the strongest evidence for M0: current rollouts do not persist the complete serialized request, tool catalog, cache key, or transport identity needed to deterministically name the first differing field. The scanner narrows incidents and proves their timing; the M0 request-fingerprint work must supply the missing discriminator.

Subagent history classification uses persisted inheritance markers (`forked_from_id`, `history_base`, or `subagent_history_start_ordinal`). A subagent without those markers is classified as fresh. This keeps the 170 avoidable forked-history candidates out of the two high-priority fresh-context witnesses.

## Sort order

Work the ledger in this order:

1. **Core prefix construction.** Establish one deterministic representation and comparison surface for instructions, world state, skills, tools, request settings, and cache/routing identities. Every later investigation depends on this.
2. **Session compaction.** Fix compact-request parity, trigger accounting, replacement-history stability, and resume parity because every long-running session crosses this boundary.
3. **Fresh-context subagents.** Eliminate timeout-driven orchestrator polling, then prove that a child without inherited conversation history reuses the parent's still-warm stable startup prefix. This is the normal high-volume workflow.
4. **Tool stability.** Preserve deterministic ordering and freeze one complete catalog revision at each sampling boundary.
5. **Forked-history subagents.** Keep below fresh-context behavior because this mode is rarer and avoidable.
6. **Backend-dependent, amplification, and excluded reports.** Separate these from client-controlled prefix fixes.

Within each priority, handle confirmed current-source defects before suspected or backend-dependent behavior. Then sort by blast radius, expected wasted uncached tokens, reproducibility, and independence of the fix.

## Repair queue

| Priority | Mechanism | Current-source verdict | Primary issues | Next gate |
|---:|---|---|---|---|
| 1.1 | Complete prefix construction and first-difference diagnostics | Confirmed observability gap | #35706, #32479, #43615 | Capture redacted instructions, input, tools, request settings, cache key, and transport identities in serialized order. |
| 1.2 | Plugin skills expose mutable cache-version paths in the startup prefix | Confirmed design risk; stale-path handling partly improved | #25609, #24390, #25285 | Render stable semantic locators or aliases and compare equivalent startup prefixes across marketplace refreshes. |
| 1.3 | Core prefix fields can change without a single explicit fingerprint/invariant | Test gap | #30425, #35925 | Define and test a cache-relevant prefix fingerprint across consecutive turns and warm resume. |
| 2.1 | Local compaction request omits the active tool specification | Confirmed present | #37305 | Prove compact and normal requests share the maximum eligible prefix; then pass active tools and parallel-call settings into local compaction. |
| 2.2 | Compaction is triggered from stale token accounting and discards the current operational tail | Confirmed/strongly evidenced | #32888, #35935, #36665, #36721, #29319, #34017 | Fix the provider-usage/history-boundary invariant before changing retained-history design. |
| 3.1 | Completion-driven parent wake-up without timeout polling | Active waits are event-driven; idle/expired waits can still require repeated model turns | #37299, #41875 | Inject one bounded completion and wake or continue the parent when a child becomes terminal; suppress a duplicate when an active wait consumed it. |
| 3.2 | Fresh-context subagent reuse of the warm parent startup prefix | Identity handling present; prefix reuse unproven | #39808; related #44716 | Compare parent and child byte prefixes and cached tokens while omitting parent conversation history. |
| 4.1 | MCP inventory depends on asynchronous startup and ignores later `tools/list_changed` | Confirmed present | #43642, #33266, #37417, #35583, #10105, #19155, #20605 | Freeze an atomic catalog revision at a turn boundary and refresh deterministically for the next boundary. |
| 4.2 | MCP ordering across complete serialized tool arrays | Reported defect appears addressed; regression gap remains | #37351 | Compare the complete tool array across fresh processes, including namespace, hosted, and non-MCP tools. |
| 5.1 | Forked-history subagent cache lineage | Client-side identity handling appears improved; backend reuse unproven | #24704, #44716 | Run only after fresh-context subagent reuse is established. |
| 6.1 | Transport `session-id` influences cache affinity independently of `prompt_cache_key` | Backend-dependent and unresolved | #44716, #30425, #33821, #20301, #21756 | Run paced identity controls before changing identity semantics. |
| 6.2 | Independent sessions lack deliberate shared affinity and providers lack an explicit breakpoint | Product/provider gaps | #21796, #29377, #26283, #35300 | Specify safe cache affinity separately from conversation identity. |
| 6.3 | Goal steering includes changing usage counters | Present, cache effect unproven | #25320 | Verify whether updates append after the cached prefix or replace earlier content. |

## Delivery stages

1. **Core prefix construction:** add the comparison harness, establish a prefix fingerprint/invariant, and remove volatile skill locators.
2. **Session compaction:** fix local request parity, then accounting, then retained-tail continuity.
3. **Fresh-context subagents:** add completion-driven parent wake-up without timeout polling, then prove warm parent-prefix reuse and stable identities without copying parent conversation history.
4. **Tool stability:** verify complete-array ordering, then make startup and notification-driven catalog changes atomic at turn boundaries.
5. **Forked-history subagents:** verify lineage only after the common fresh-context path is stable.
6. **Backend and efficiency work:** investigate hidden affinity, optional breakpoints, goal counters, and residual wait amplification separately.

The Pareto line is after stage 4. Obtain user approval before forked-history, backend/capability, or non-bust efficiency work.

## Mechanism records

### M0 — Core prefix construction

- **Verdict:** the request builder has stable components, but no single regression proves that the complete cache-relevant prefix is unchanged across an ordinary next turn, warm resume, or fresh-context subagent.
- **Construction surface:** base instructions, model-visible input, tool schemas and order, parallel-tool setting, reasoning settings, service tier, text settings, JSON `prompt_cache_key`, and transport session/thread identities.
- **Known stable behavior:** running world-state changes are appended as diffs; environment maps are ordered; MCP tools and tools inside namespaces are explicitly sorted; warm root resume restores the thread-derived session ID; fresh subagents share the root session ID while retaining distinct thread IDs.
- **Known instability:** plugin skill locators can expose mutable cache-version paths; asynchronously captured tools can change the complete tool array; backend affinity can depend on transport identity even when the JSON key is stable.
- **Required invariant:** for any two requests expected to reuse cache, emit a redacted fingerprint of every cache-relevant field and identify the first serialized difference.
- **Exit tests:** ordinary next turn, warm same-day resume, date rollover in a running session, unchanged and changed environment state, plugin refresh, fresh-context subagent, process restart with the same MCP catalog, HTTP/WebSocket reconnect, and local compaction.
- **Issues:** #35706, #25609, #30425, #32479, #35300, #35925, #43615.

### M1 — Local compaction request parity

- **Verdict:** confirmed present in `codex-rs/core/src/compact.rs`.
- **Evidence:** the local compaction `Prompt` supplies history and base instructions but uses defaults for tools and parallel tool calls. Normal turns supply the active tool list.
- **Why it matters:** a nearly full history is sent in the compaction call, so losing reuse here can rebill the largest request in the session.
- **Required invariant:** for the same context window, the compact request must retain every unchanged cache-relevant field from the immediately preceding normal request.
- **Exit test:** capture consecutive normal and compact requests; after removing the compact instruction/suffix, their eligible stable prefix and tool serialization match byte-for-byte.
- **Issues:** #37305; related cost evidence #35925.

### M2 — Compaction accounting and operational continuity

- **Verdict:** stale pre-turn/post-tool accounting remains visible in current source; loss of operational state is strongly evidenced by production reports.
- **Evidence:** the pre-turn path still notes that incoming context and user input are not included in the compaction decision. Current accounting uses the latest server usage rather than a trustworthy usage sample paired atomically with its exact history boundary.
- **Fix stage A:** pair each trustworthy provider usage sample with the exact covered history boundary. A response without numeric usage must not advance that boundary.
- **Fix stage B:** estimate pending world-state changes, user input, and tool output before the next request.
- **Fix stage C:** retain a hard-capped raw tail made of complete user/assistant/tool groups, plus a structured checkpoint for completed work, failed attempts, subagent results, tests, and the next action.
- **Required invariant:** compaction never leaves an unpaired tool call/result and live, persisted, and resumed replacement histories are identical.
- **Exit tests:** threshold crossing after a large tool result; usage-less response; resume/fork reconstruction; rollback/history replacement; bounded current-turn tail; no repeat of an explicitly recorded failed action.
- **Issues:** #32888, #29319, #34017, #35935, #36665, #36721, #37448, #13279, #14425, #23589, #25900, #25394.
- **Historical/protocol reports:** #19400, #27005, #27423.

### M3 — MCP catalog lifecycle

- **Verdict:** confirmed present.
- **Evidence:** the active handler for `tools/list_changed` only logs the notification. Initial capture can also proceed after optional startup grace with an incomplete inventory.
- **Constraint:** refreshing tools is legitimate, but the refresh must happen at a defined turn boundary. A mid-request mutation would create both cache instability and tool-call/schema races.
- **Required invariant:** identical server readiness and schemas produce identical serialized tools; one catalog revision is frozen for a complete sampling/tool-call step.
- **Exit tests:** delayed optional server, notification after startup, same-name schema replacement, reconnect, config refresh, and stable ordering across fresh processes.
- **Issues:** #43642, #33266, #37417, #35583, #10105, #19155, #20605.

### M4 — Plugin and skill locator stability

- **Verdict:** model-visible locators can still contain marketplace cache versions. Cache invalidation callbacks have improved, but a new version can still change startup text and older persisted paths can become stale.
- **Required invariant:** plugin content changes may change a content revision, but installation directory names and unrelated marketplace updates must not change the model-visible locator for an unchanged skill.
- **Preferred direction:** stable semantic identifiers such as `skill://<source>/<plugin>/<skill>`, resolved through the active registry. A stable `current` path is an interim compatibility mechanism.
- **Exit tests:** unrelated plugin update, same plugin update, warm running thread, cold resume, and new equivalent session. Compare both skill availability and complete rendered startup context.
- **Issues:** #25609, #24390, #25285.

### M5 — Session/cache affinity and hidden backend routing

- **Verdict:** unresolved and partly backend-controlled.
- **Current client behavior:** warm root resume reuses the thread ID as session ID. Resumed subagents restore the root session identity. Internal subagents derive their JSON cache key from their source and parent thread.
- **Remaining risk:** controlled reports show the transport `session-id` can affect reported cache reuse independently of the JSON cache key, while unchanged controls can still miss intermittently.
- **Do not do:** do not reuse a parent transport identity across unrelated threads without a backend contract; that could conflate authorization, telemetry, or hidden state.
- **Exit test:** paced matrix covering warm resume, ordinary user fork, internal subagent, fresh identical session, concurrent identical requests, HTTP, and WebSocket. Record complete redacted request hashes, identity headers, response IDs, and cached tokens.
- **Issues:** #44716, #24704, #30425, #33821, #20301, #21756.

### M6 — Explicit cross-session affinity and cache breakpoints

- **Verdict:** missing capabilities rather than regressions.
- **Use case:** ephemeral or independently started sessions with identical stable startup context cannot safely declare shared affinity; custom providers cannot receive an explicit stable-prefix breakpoint.
- **Required invariant:** cache identity is separate from conversation identity and is scoped to the same authorization, provider, model family, and compatible prompt schema.
- **Issues:** #21796, #29377, #26283, #35300.

### M7 — Dynamic context such as dates, times, environment, and goal counters

- **Verdict:** ordinary world-state changes are appended as bounded diff messages; they do not rewrite the established history prefix. Environment maps use deterministic ordering.
- **Date behavior:** the initial context includes the current date and timezone. A date change in a running session is appended as a diff. A genuinely new session on a different day has different truthful context and should not be treated as an accidental bust.
- **Time behavior:** current time is not continuously rewritten into the prefix; reminder behavior is feature/mode dependent.
- **Open question:** goal steering renders changing token/time counters. Verify placement and cadence before changing it.
- **Issues:** #25320. JSONL repetition in #10403 is persistence bloat, not proof of a model-request cache bust.

### M8 — Fresh-context subagent prefix reuse

- **Verdict:** identity handling is present, but cache reuse is not yet proven.
- **Current behavior:** a fresh-context child omits parent conversation history, inherits the parent's creation-time instructions, shares the root `session-id` and JSON cache key, and uses its own `thread-id`.
- **Risk:** inherited instructions, skills, tools, role text, or child-only developer fragments can diverge before the reusable boundary. Existing tests assert identity fields and absence of parent history, but do not compare the complete parent/child prefix or cached-token behavior.
- **Required invariant:** a fresh child reuses the maximum eligible still-warm startup prefix without inheriting parent task history or conflating the child thread identity.
- **Exit test:** seed a long stable startup prefix in the parent, spawn multiple fresh-context children sequentially and concurrently, compare complete redacted prefix fingerprints, and verify warm cached-token reuse for every child.
- **Issues:** no issue isolates this exact path. #39808 establishes the fixed-context cost; #44716 establishes transport-identity sensitivity. Keep #24704 with forked-history behavior.

### M9 — Deterministic MCP ordering

- **Verdict:** apparently addressed in current source, despite the issue remaining open.
- **Evidence:** normalized MCP candidates are sorted by stable raw tool identity, and tools inside a namespace are sorted by name.
- **Remaining gate:** add or locate a process-restart regression that compares the complete serialized tool array, including namespace order and non-MCP/hosted tools.
- **Issue:** #37351.

### M10 — Subagent completion wake-up and residual polling cost

- **Priority:** high within fresh-context subagent work because every unnecessary timeout causes another model request over the orchestrator's accumulated context.
- **Verdict:** event-driven waiting is implemented only while a wait call remains active; residual timeout polling remains possible and is amplification rather than a prefix mutation.
- **Current V1 behavior:** `wait_agent` subscribes to each child's status channel and returns immediately when any target reaches a final state.
- **Current V2 behavior:** `wait_agent` subscribes to parent mailbox activity. Child completion queues a mailbox message, which wakes an active wait immediately.
- **Distinction from background exec:** subagent completion uses status/mailbox watch channels. It does not use the local `ExecCompletion` injection mechanism.
- **Residual risk:** the default wait is 30 seconds. If a child runs longer and the model repeatedly calls `wait_agent` after timeouts, each timeout can create another model request over the parent's context.
- **Required behavior:** when a child reaches terminal status, inject one hard-capped `AgentCompletion`-equivalent item and wake or continue the parent without a polling request. A completion already returned through an active `wait_agent` call must not be injected twice.
- **Required measurement:** run a child longer than 30 seconds without repeated waits and assert exactly one completion-driven parent continuation. Also cover completion during an active wait, completion after a wait timeout, multiple simultaneous completions, parent cancellation, and parent compaction.
- **Compaction interaction:** parent compaction can discard subagent findings or completion state, causing repeated delegation. This belongs to M2.
- **Fixed context:** each agent receives substantial instructions/tools/skills. Reduce or defer that context independently of prefix-stability work.
- **Issues:** #35935, #37299, #39808, #41875.
- **Excluded workaround:** banning fork-style subagents does not address parent compaction, polling, fixed-context overhead, tool lifecycle, or plugin locator changes.

## Backend-only or provider-specific evidence

Do not mix these reports into a client patch without a discriminator showing a request difference:

- #30425 — intermittent zero hits despite stable key/prefix.
- #33821 — identical concurrent request bodies split between hit and zero.
- #20301 — low GPT-5.5 cache rate.
- #21756 — cache drops during short continuous sessions.
- #25604 — Azure eviction/retention behavior; ordinary expiry is out of scope.

Historical backend fixes useful as regression scenarios:

- #32613 — first image invalidation.
- #37674 — Bedrock web-search tool invalidation.
- #2610 — large MCP tool sets and cache behavior.
- #5556 — elevated misses later reported fixed.

## Amplification and observability, not direct busts

- #4764 — early report quantifying prompt-cache cost impact.
- #35925 — quantified uncached-cost concentration and silent context shrinkage.
- #37299 — parent polling repeatedly meters a large cached context.
- #39808 — subagent fixed-context multiplication.
- #32479 — cache-write telemetry.
- #43615 — cache visibility.
- #35706 — full prompt snapshot for comparison.
- #10403 — repeated rollout persistence, not repeated model-prefix insertion.

## Excluded from the repair queue

- User-selected reasoning/model changes: #35416, #32533, #42996.
- Cache-lifetime/idle-expiry policies: #40924, #41875 when considered only as TTL behavior.
- Desktop binary/backend packaging: #13709, #13715, #30988, #34583, #40752, #43640.
- Old protocol/model compatibility: #19400, #27005, #27423 unless reproduced on the current protocol.

## Per-mechanism workflow

For each queue item:

1. Query linked issues for state and update-time changes. Read only new or changed material, plus any specific detail the ledger does not already capture.
2. Pin the source revision used for analysis.
3. Capture the complete redacted request representation before provider-specific transmission.
4. Reproduce with two requests that should share a prefix.
5. Identify the first cache-relevant difference; if none exists, classify the report as backend-dependent.
6. Add a failing discriminator test for that difference.
7. Implement one surgical fix on `work/issue-<number>`.
8. Verify byte-stable serialization and cached-token behavior separately; cached-token counters alone do not prove physical cache reuse.
9. Update this ledger with the commit, test, remaining uncertainty, and issue status.
10. Do not begin the next mechanism until the current fix is integrated or explicitly blocked.

## Completion definition

A mechanism is complete only when:

- the current source no longer permits the unintended difference;
- a regression test detects reintroduction;
- warm resume and post-compaction behavior are covered where applicable;
- tool arrays and model-visible context have deterministic serialized comparisons;
- the linked GitHub reports are updated or their remaining backend dependency is recorded;
- no unknown request difference is dismissed solely because `prompt_cache_key` stayed constant.
