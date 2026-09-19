# `wait_agent` disablement and polling evidence

## Durable evidence package

| Evidence | Location |
|---|---|
| Reproduction instructions and data dictionary | [wait-agent-evidence/README.md](wait-agent-evidence/README.md) |
| Record-level polling calls and token attribution | [wait-agent-evidence/polling-records.jsonl](wait-agent-evidence/polling-records.jsonl) |
| Machine-checkable aggregate totals | [wait-agent-evidence/polling-summary.json](wait-agent-evidence/polling-summary.json) |
| Record-level exec-completion envelopes | [wait-agent-evidence/exec-completion-notifications.jsonl](wait-agent-evidence/exec-completion-notifications.jsonl) |
| Immutable rollout paths, hashes, sizes, timestamps, and session metadata | [wait-agent-evidence/source-manifest.json](wait-agent-evidence/source-manifest.json) |
| Original audit commands and outputs | [wait-agent-evidence/original-audit-command-log.jsonl](wait-agent-evidence/original-audit-command-log.jsonl) |
| Source blobs, excerpts, commits, and ancestry | [wait-agent-evidence/source-snapshot.json](wait-agent-evidence/source-snapshot.json) |
| Relevant configuration and full-file hash | [wait-agent-evidence/config-snapshot.json](wait-agent-evidence/config-snapshot.json) |
| Installed binaries and resumed-harness observations | [wait-agent-evidence/runtime-snapshot.json](wait-agent-evidence/runtime-snapshot.json) |
| Public GitHub issues, PRs, comments, timeline links, reviews, and commits | [wait-agent-evidence/github-snapshot.json](wait-agent-evidence/github-snapshot.json) |
| Evidence integrity hashes | [wait-agent-evidence/SHA256SUMS](wait-agent-evidence/SHA256SUMS) |

## Investigation boundary

| Field | Value |
|---|---|
| Brisbane interval | `[2026-09-19 00:00:00, 2026-09-20 00:00:00)` AEST (`UTC+10`) |
| UTC interval | `[2026-09-18T14:00:00Z, 2026-09-19T14:00:00Z)` |
| Historical rollout roots | `~/.codex/sessions`, `~/.codex/archived_sessions` |
| Incident executable | `codex-cli 0.154.0+local.3` |
| Incident source commit | `ffde91b0c52241cf0b746fe3524c52b82ddd099a` |
| Current executable | `codex-cli 0.155.1+local.4` |
| Current checkout during live probes | `7516ec8035ce707156185214b6f56821c6c68b91` |
| Current branch | `local/customizations` |
| Current branch relation | 279 commits ahead of and 57 commits behind `fork/local/customizations` |
| Free disk at final check | 382 GiB |

## User-session census

`response_item.payload.type == "custom_tool_call"`; counts below are top-level `exec` wrapper calls.

| Session | Started UTC | Working directory | `exec` calls |
|---|---:|---|---:|
| `01a0b75e-2a0d-7ca0-bddb-f59049ce1530` | 2026-09-19 01:53:16 | `/Users/viktor/Projects/github/codebase-memory-mcp` | 38 |
| `01a0b76f-9e55-7060-a4a5-74c923840d81` | 2026-09-19 02:12:20 | `/Users/viktor/Projects/github/codebase-memory-mcp` | 16 |
| `01a0b77c-8c00-76c1-be80-26d140fc209c` | 2026-09-19 02:26:27 | `/Users/viktor/Projects/github/codebase-memory-mcp` | 825 |
| `01a0b786-a3f3-7120-b40d-33196c23e7ea` | 2026-09-19 02:37:29 | `/Users/viktor/Projects/project-management` | 480 |
| `01a0b974-fc5f-7c01-8999-b2dd73c0a686` | 2026-09-19 11:37:26 | `/Users/viktor/Projects/github/codex` | 53 |
| `01a0b9bd-35e4-7f32-a73e-5acbe0a1f658` | 2026-09-19 12:56:20 | `/Users/viktor/Projects/github/codex` | 0 |
| `01a0b9bf-d50c-7bd2-a600-0960d73b43ec` | 2026-09-19 12:59:12 | `/Users/viktor/Projects/github/codex` | 67 |

## Polling measurements

Fresh tokens = input tokens − cached input tokens + output tokens.

| Polling mechanism | Calls inspected | Conservatively wasted model rounds | Raw processed tokens | Fresh tokens | Classification |
|---|---:|---:|---:|---:|---|
| Empty `write_stdin` completion polls | 127 | 127 | 16,368,566 | 378,550 | No interaction; no required intermediate output; completion discovery only |
| Code-mode `wait` calls | 43 | 43 | 3,803,025 | 186,001 | Passive completion/status waiting |
| MultiAgentV2 `wait_agent` | 216 total | 148 timed-out calls | 18,471,910 | 217,702 | Successful/activity-bearing calls excluded from the wasted-round count |
| **Total** | **386 inspected** | **318** | **38,643,501** | **782,253** | Conservative lower bound |

## Polling ranking

| Rank | Metric | Mechanism | Value |
|---:|---|---|---:|
| 1 | Wasted model rounds | `wait_agent` | 148 |
| 1 | Raw processed tokens | `wait_agent` | 18,471,910 |
| 1 | Fresh tokens | empty `write_stdin` | 378,550 |
| 2 | Wasted model rounds | empty `write_stdin` | 127 |
| 3 | Wasted model rounds | code-mode `wait` | 43 |

## Incident-binary patch provenance

| Behavior | Local commit | Ancestor of incident source `ffde91b0c522...` | Active mechanism |
|---|---|---:|---|
| Background `exec_command` completion wake | `3e3b41a7a2712420b149088905eff86eb44c67a1` | Yes | Process exit injects bounded `ExecCompletion`; commands completed during initial call do not emit a duplicate |
| Terminal MultiAgentV2 child completion wake | `8c5e8401e8f24f366c882b1864d0ef41e45ac9f9` | Yes | Terminal child result wakes or continues its direct parent |

| Historical runtime signal | Count |
|---|---:|
| Persisted `<exec-command-completed>` notifications in the Brisbane-day evidence window | 76 |
| Conclusion supported by notification presence | Background completion notification code executed in the incident runtime |

| Evidence | Location |
|---|---|
| Incident installation provenance | [installation-local3.json](cache-repair-reports/installation-local3.json) |
| Local patch ledger | [cache-repair-run.md](cache-repair-run.md) |
| Current preservation rules | [AGENTS.md](../AGENTS.md) |

## Configuration state

```toml
[features.multi_agent_v2]
wait_agent_enabled = false
```

| Fact | Evidence |
|---|---|
| Setting file | `~/.codex/config.toml` |
| Setting observed after harness resume | `false` |
| Default | `true` |
| Schema description | “Expose the multi-agent v2 `wait_agent` tool.” |
| Scope | MultiAgentV2 tool registration and default usage-hint generation |
| Legacy MultiAgentV1 effect | None; the flag gates the V2 registration branch |
| Terminal-completion effect | None; terminal forwarding does not read `wait_agent_enabled` |
| Background-exec effect | None |

## Configuration and registration source map

| Function or type | Location | Fact |
|---|---|---|
| `MultiAgentV2ConfigToml::wait_agent_enabled` | [`codex-rs/features/src/feature_configs.rs:288`](../codex-rs/features/src/feature_configs.rs#L288) | Optional runtime configuration field |
| `MultiAgentV2Config::defaults_for_max_concurrency` | [`codex-rs/core/src/config/mod.rs:1302`](../codex-rs/core/src/config/mod.rs#L1302) | Default is `true` |
| `resolve_multi_agent_v2_config` | [`codex-rs/core/src/config/mod.rs:2754`](../codex-rs/core/src/config/mod.rs#L2754) | Configured value overrides default |
| Tool-name planning | [`codex-rs/core/src/tools/spec_plan.rs:697`](../codex-rs/core/src/tools/spec_plan.rs#L697) | Adds `wait_agent` only when enabled |
| V2 tool registration | [`codex-rs/core/src/tools/spec_plan.rs:1318`](../codex-rs/core/src/tools/spec_plan.rs#L1318) | Registers `WaitAgentHandlerV2` only when enabled |
| Config schema | [`codex-rs/core/config.schema.json:2506`](../codex-rs/core/config.schema.json#L2506) | Boolean `wait_agent_enabled` field |

## `wait_agent` behavior

| Property | Value | Evidence |
|---|---|---|
| Input | Optional `timeout_ms` | [`wait.rs:85`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L85) |
| Default timeout | Configured `default_wait_timeout_ms` | [`wait.rs:52`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L52) |
| Minimum timeout | Configured `min_wait_timeout_ms`; shorter values are clamped | [`wait.rs:50`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L50) |
| Maximum timeout | Configured `max_wait_timeout_ms`; larger values are rejected | [`wait.rs:51`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L51) |
| Wake sources | Mailbox activity; steering input; timeout | [`wait.rs:74`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L74) |
| Output | `message`, `timed_out` | [`wait.rs:91`](../codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L91) |
| Agent selection | None | `WaitArgs` contains only `timeout_ms` |
| Agent result | Not returned directly | Output contains only wait outcome |
| All-children join guarantee | None | Generic input-queue activity subscription |

## Terminal child completion path

| Stage | Location | Fact |
|---|---|---|
| Terminal event hook | [`codex-rs/core/src/session/mod.rs:2232`](../codex-rs/core/src/session/mod.rs#L2232) | Every emitted event is offered to terminal-parent notification logic |
| V2 and final-status gates | [`codex-rs/core/src/session/mod.rs:2252`](../codex-rs/core/src/session/mod.rs#L2252) | Requires MultiAgentV2, `ThreadSpawn`, agent path, and final status |
| Completion envelope | [`codex-rs/core/src/session/mod.rs:2377`](../codex-rs/core/src/session/mod.rs#L2377) | `InterAgentCommunication` is created with `trigger_turn = true` |
| Receiver enqueue | `codex-rs/core/src/session/handlers.rs:82-97` | Enqueues communication; schedules pending work when `trigger_turn` is true |
| Idle/active scheduling | `codex-rs/core/src/tasks/mod.rs:441-500` | Starts an idle parent; leaves trigger mail pending behind an active turn |
| Post-turn scheduling | `codex-rs/core/src/tasks/mod.rs:850-866` | Rechecks pending work after active task completion |
| V2 legacy-watcher exclusion | [`codex-rs/core/src/agent/control/spawn.rs:788`](../codex-rs/core/src/agent/control/spawn.rs#L788) | Detached legacy completion watcher is started only when version is not V2 |
| Legacy watcher V2-looking branch | [`codex-rs/core/src/agent/control.rs:624`](../codex-rs/core/src/agent/control.rs#L624) | Contains `trigger_turn = false`; not the active V2 spawn path |

## Remaining collaboration-tool behavior

| Tool | Delivery mode | Starts or steers target turn | Root target permitted | Observed after switch |
|---|---|---:|---:|---:|
| `spawn_agent` | New child turn | Yes | Not applicable | Yes |
| `send_message` | `QueueOnly` | No | Yes | Available |
| `followup_task` | `TriggerTurn` | Yes | No | Accepted by running Luna probe |
| `interrupt_agent` | Interrupt | No new task required | Applicable to descendants | Available |
| `list_agents` | Status snapshot | No | Not applicable | Returned root/running/completed states |
| `wait_agent` | Input-queue wait | Current parent remains sampled until activity/timeout | Not applicable | Absent |

| Source | Fact |
|---|---|
| [`send_message.rs:38`](../codex-rs/core/src/tools/handlers/multi_agents_v2/send_message.rs#L38) | Uses `MessageDeliveryMode::QueueOnly` |
| [`followup_task.rs:38`](../codex-rs/core/src/tools/handlers/multi_agents_v2/followup_task.rs#L38) | Uses `MessageDeliveryMode::TriggerTurn` |
| [`message_tool.rs:58`](../codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs#L58) | Rejects a trigger-turn follow-up targeting the root agent |
| [`input_queue.rs:124`](../codex-rs/core/src/session/input_queue.rs#L124) | Mailbox communications are queued and publish mailbox activity |

## Workflow outcomes

| Workflow | Outcome with `wait_agent_enabled = false` |
|---|---|
| Parent spawns child and continues useful work | Supported |
| Parent ends its turn before child completes | Terminal child result starts one bounded parent continuation |
| Child completes while parent is active | Result is queued/steered and delivered without `wait_agent` |
| Parent sends new work to running or idle child | `followup_task` starts or steers child |
| Parent sends non-waking information to child | `send_message` queues information |
| Child sends progress to active parent | Queue-only message can be consumed by active parent |
| Child sends a question to idle parent and stays running | No root wake from `send_message`; possible workflow stall |
| Child terminates with a blocking question | Terminal result wakes parent; parent can answer with `followup_task` |
| Parent repeatedly calls `list_agents` for completion discovery | Recreates polling behavior under a different tool name |
| Multiple children finish separately | Each terminal completion is an event capable of scheduling bounded parent work; mailbox work may coalesce around active turns |

## Prompt/tool mismatch

| Fact | Evidence |
|---|---|
| Disabled tool is absent from tool registration | [`spec_plan.rs:1318`](../codex-rs/core/src/tools/spec_plan.rs#L1318) |
| Long-wait recommendation is conditionally omitted | [`multi_agents.rs:123`](../codex-rs/core/src/session/multi_agents.rs#L123) |
| Shared guidance still names `wait_agent` among callable direct collaboration tools | [`multi_agents.rs:53`](../codex-rs/core/src/session/multi_agents.rs#L53) |
| Resumed harness developer instructions contained the stale `wait_agent` name | Observed in the active session |
| Low-effort Luna did not attempt the absent tool | Live probe rollout |
| Existing tests assert absence of the tool and conditional absence of the long-wait sentence | `config_tests.rs`, `spec_plan_tests.rs`, `spawn_agent_description.rs` |
| Existing tests do not assert that disabled default guidance contains no `wait_agent` reference anywhere | Source/test inspection |

## Live probes after harness resume

Fresh tokens = input tokens − cached input tokens + output tokens.

| Probe | Model | Reasoning | UTC interval | Duration | Assistant messages | Tool calls | Raw tokens | Fresh tokens | Result |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `/root/luna_quick_probe` | `gpt-5.6-luna` | low | 22:38:44.725–22:38:54.049 | 9.324 s | 1 | 1 `exec` | 59,833 | 30,393 | Confirmed config `false`; confirmed no `wait_agent`; completion delivered automatically |
| `/root/luna_deep_probe` | `gpt-5.6-luna` | high | 22:38:48.963–22:41:32.568 | 163.605 s | 4 | 32 `exec` | 2,750,917 | 143,813 | Traced V2 terminal wake path; received mid-flight `followup_task`; completion delivered automatically |

| Probe evidence | Path |
|---|---|
| Quick rollout | `~/.codex/sessions/2026/09/20/rollout-2026-09-20T08-38-44-01a0bbd2-6b78-70b2-bd6a-bb57be40955b.jsonl` |
| High-effort rollout | `~/.codex/sessions/2026/09/20/rollout-2026-09-20T08-38-48-01a0bbd2-7c09-7161-bc1b-533139333f24.jsonl` |
| Root polling calls during probes | 0 `wait_agent`; 1 deliberate `list_agents` snapshot |
| High-effort intermediate-message request | Not executed; rollout contains no `send_message` custom tool call |
| Intermediate-message delivery verdict | Inconclusive; no call was issued |
| Idle-parent live wake verdict | Not directly exercised in the live probe because the root turn remained active |
| Idle-parent mechanistic verdict | Covered by source path and integration test below |

## Luna probe comparison

| Metric | High / low ratio |
|---|---:|
| Duration | 17.55× |
| Tool calls | 32× |
| Raw tokens | 45.98× |
| Fresh tokens | 4.73× |

## Regression evidence

| Test | Location | Assertion |
|---|---|---|
| `multi_agent_v2_can_disable_wait_agent` | [`codex-rs/core/src/tools/spec_plan_tests.rs:2843`](../codex-rs/core/src/tools/spec_plan_tests.rs#L2843) | V2 keeps spawn/send/follow-up/interrupt/list; omits `wait_agent` |
| `multi_agent_v2_allows_disabled_wait_agent_without_sleep_tool` | [`codex-rs/core/src/config/config_tests.rs:11824`](../codex-rs/core/src/config/config_tests.rs#L11824) | Config parses independently of clock sleep state |
| `multi_agent_v2_wait_agent_tool_follows_configuration` | [`codex-rs/core/tests/suite/spawn_agent_description.rs:697`](../codex-rs/core/tests/suite/spawn_agent_description.rs#L697) | Request tool catalogue follows the setting |
| `multi_agent_v2_resume_refreshes_changed_wait_guidance` | [`codex-rs/core/tests/suite/spawn_agent_description.rs:567`](../codex-rs/core/tests/suite/spawn_agent_description.rs#L567) | Resume refreshes tool availability and conditional long-wait guidance |
| `multi_agent_v2_terminal_child_wakes_idle_parent_once` | [`codex-rs/core/tests/suite/subagent_notifications.rs:2338`](../codex-rs/core/tests/suite/subagent_notifications.rs#L2338) | Idle parent receives exactly one terminal-child completion request without polling |
| `plaintext_multi_agent_v2_completion_during_wait_is_delivered_once` | `codex-rs/core/tests/suite/subagent_notifications.rs` | An active wait consumes completion without a duplicate automatic request |
| `background_exec_completion_starts_a_follow_up_turn_without_polling` | [`codex-rs/core/tests/suite/unified_exec.rs:1221`](../codex-rs/core/tests/suite/unified_exec.rs#L1221) | Yielded process exit starts a bounded follow-up turn |

| Test execution during this report session | Value |
|---|---|
| Rust integration tests rerun | No |
| Reason | Repository was under concurrent-writer/read-only investigation constraints |
| Live behavioral probes | Two fresh-context Luna agents |

## Local commits

| Commit | Subject |
|---|---|
| `3e3b41a7a2712420b149088905eff86eb44c67a1` | `fix(core): wake agent when background exec completes` |
| `8c5e8401e8f24f366c882b1864d0ef41e45ac9f9` | `Wake idle parents for terminal subagent results` |
| `19ccaa5827a7a3f7dbadcca415b836d69d7d4018` | Current-line equivalent: `Wake idle parents for terminal subagent results` |

## Upstream commits, pull requests, and issues

| Reference | State/type | Fact |
|---|---|---|
| [PR #34887](https://github.com/openai/codex/pull/34887) / `4462b9deef211723b781b426f5e5d36a5777115f` | Merged | Adds default-on `features.multi_agent_v2.wait_agent_enabled`; independently omits V2 wait tool from the tool plan |
| [PR #35594](https://github.com/openai/codex/pull/35594) / `8a1c9414399bd8c52cd11d89f420767c41f4ae99` | Merged | Recommends longer V2 wait timeouts to reduce busy polling |
| [PR #37357](https://github.com/openai/codex/pull/37357) / `4d7e3e90d9781514f9b3b99c95169a2386868ad9` | Merged | Clamps short `wait_agent` timeouts to configured minimum |
| [Issue #37299](https://github.com/openai/codex/issues/37299) | Public diagnostic | Reports 8,744 of 11,002 model-visible calls as wait-family calls; 83% of `wait_agent` calls timed out |
| [Issue #41875](https://github.com/openai/codex/issues/41875) | Public diagnostic | Related parent polling and completion-delivery concern; local completion wake addresses only the completion-driven portion |
| [Issue #32188](https://github.com/openai/codex/issues/32188) | Open mechanism discussion | Event-driven wake when a background exec session completes |
| [Issue #45974](https://github.com/openai/codex/issues/45974) | Open mechanism discussion | Repeated model wakeups to poll deterministic jobs; runtime-owned waiting and one completion wake |
| [Issue #46085](https://github.com/openai/codex/issues/46085) | Open mechanism discussion | Event-driven wake when a local process completes |
| [Issue #28144](https://github.com/openai/codex/issues/28144) | Open mechanism discussion | Durable wait state and event/timer wake for Goals |
| [Issue #38495](https://github.com/openai/codex/issues/38495) | Public diagnostic | One long command became 90 model turns and 21.6M input tokens |

## Findings matrix

| Finding | Verdict | Evidence class |
|---|---|---|
| Incident binary lacked event-driven completion patches | False | Commit ancestry and installation provenance |
| Background exec completion patch was active in `local.3` | True | `3e3b41a7a2` is an ancestor of `ffde91b0c522...` |
| Terminal child completion patch was active in `local.3` | True | `8c5e8401e8` is an ancestor of `ffde91b0c522...` |
| Event-driven completion prevented model polling by itself | False | Historical rollouts contain explicit polling calls despite installed completion paths |
| `wait_agent` produced the most wasted model rounds | True | 148 timed-out calls versus 127 empty `write_stdin` and 43 code-mode waits |
| `wait_agent` produced the most raw processed polling tokens | True | 18,471,910 raw tokens |
| `wait_agent` produced the most fresh polling tokens | False | Empty `write_stdin` produced 378,550 fresh tokens versus 217,702 |
| Disabling V2 `wait_agent` breaks child spawning | False | Live probes and tool-plan test |
| Disabling V2 `wait_agent` breaks terminal completion delivery | False | Independent source path; terminal trigger does not consult flag |
| Disabling V2 `wait_agent` removes explicit mailbox waiting | True | Handler absent from tool catalogue |
| Disabling V2 `wait_agent` removes all polling mechanisms | False | `list_agents`, `write_stdin`, code-mode waits, and model-driven status checking remain possible |
| Disabled-tool prompt guidance is internally consistent | False | Shared usage hint still names unavailable `wait_agent` |
| Low-effort Luna was sufficient for the configuration/tool-presence probe | True | 1 tool call; correct result |
| High-effort Luna was cheaper for the source trace | False | 32 tool calls; 2,750,917 raw tokens; 163.605 seconds |

## Operational constraints

| Constraint | Recorded state |
|---|---|
| Repository writes before report request | None |
| Report creation authorization | Explicit user request |
| Production source changes | None |
| Configuration changes during report creation | None |
| Build artifacts created | None |
| Test artifacts created | None |
