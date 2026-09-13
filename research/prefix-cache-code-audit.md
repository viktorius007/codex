# Prefix-cache code audit

Assessed 2026-09-14 at `b2da63e68447be521bc9b68471d5faa861c09d62` on `local/customizations`.

Follow-up: [live compaction and cold-resume validation](compaction-resume-cache-validation.md) establishes provider cache reuse across ordinary and post-compaction cold resume, confirms unchanged tools/base instructions across compaction, and measures the relocation of stable contextual instructions. The static-only limitations below describe the initial audit; the follow-up contains the new executed evidence.

The prefix can lose reuse. This review establishes client-side mechanisms that change it; it does not establish the cause of a particular provider cache miss. No new unintended prefix-mutation defect was established in the inspected paths. Production source, installed binaries, and existing investigation files were not changed.

## Three different kinds of cache

1. The provider stores computed prompt prefixes. Codex supplies instructions, ordered input, tools, request settings, and a cache key; the provider owns matching and availability.
2. `ModelClientSession` retains a WebSocket connection, the last request, and the completed response. It can send only a suffix with `previous_response_id`. Losing this continuation means sending the full request, not necessarily losing the provider's cached prefix.
3. MCP caches tool catalogs locally. This does not store computed model tokens, but a change in the catalog can change the tools supplied to the model.

Provider behavior is model-dependent. A matching prefix still requires an eligible available cache entry; a stable cache key does not guarantee a hit. See [official OpenAI prompt-caching documentation](https://developers.openai.com/api/docs/guides/prompt-caching), fetched for this review. Public API documentation does not prove the private Codex backend's treatment of every metadata field.

## Construction and maintenance map

| Boundary | Current source | Result |
|---|---|---|
| Initial developer/contextual messages | `core/src/session/mod.rs:3935-4190` | Aggregates developer instructions, extension fragments, plugin recommendations and world state. Shared startup context precedes agent-specific role/mode messages. |
| World-state inputs | `core/src/session/world_state.rs:44-336` | Model/configuration, environment, permissions, agent state and tool exposure determine rendered context. |
| Ordinary context updates | `core/src/session/mod.rs:4290-4380` | With an existing reference, records diffs as new conversation items. It does not rewrite the original startup message on every turn. |
| Tool plan and serialization | `core/src/tools/spec_plan.rs:125-194,531-568,926-941`; `tools/src/tool_spec.rs:145-149` | Uses current model/configuration and captured bindings; serializes the resulting ordered tool list. |
| MCP normalization and startup cache | `codex-mcp/src/tools.rs:121-213`; `codex-mcp/src/connection_manager/tool_catalog.rs:171-304`; `codex-mcp/src/tool_catalog_cache.rs:32-46,117-207,216-393` | Deterministic normal tool ordering; optional servers may start from cached catalogs or be omitted until ready. Cache identity includes configuration/environment and has a 30-minute TTL. This TTL is not the provider prompt-cache lifetime. |
| Logical request construction | `core/src/client.rs:917-1023`; `core/src/client_common.rs:56-65`; `codex-api/src/common.rs:277-307` | Builds model, instructions, input, tools, reasoning, text controls, service tier and cache key. Responses Lite puts tools and base instructions at the front of input. |
| Cache-key selection | `core/src/client.rs:516-528`; `core/src/guardian/review_session.rs:289-302` | Ordinary requests use session identity; internal/reviewer sessions can use explicitly scoped keys. |
| WebSocket continuation | `core/src/client.rs:319-389,530-580,1300-1437,1509-1545,1880-2006` | Reuses only compatible request properties and an exact prior input/output prefix, ignoring internal message metadata. Connection/endpoint changes clear continuation state. |
| Wire preparation | `core/src/client.rs:1025-1034,1620-1677,1790-2006`; `codex-api/src/endpoint/responses.rs:140-237`; `codex-api/src/endpoint/responses_websocket.rs:923-948` | Removes unsupported IDs/content annotations, adds transport metadata, and sends full HTTP or full/delta WebSocket requests. These changes must not all be equated with model-visible token changes. |
| History normalization | `core/src/context_manager/history.rs:398-454,783-795`; `core/src/context_manager/normalize.rs:18-155` | Truncates recorded tool output and repairs call/output pairing; unsupported media is removed for the selected model. Synthetic repair IDs are deterministic. |
| Compaction | `core/src/compact.rs:568-610,685-775`; `core/src/compact_remote.rs:317-403`; `core/src/compact_remote_v2.rs:489-590`; `core/src/context_manager/history.rs:551-568` | Installs replacement history assembled from retained content and summary/compaction output. |
| Resume and rollback | `core/src/session/rollout_reconstruction.rs:176-210,353-435`; `core/src/session/handlers.rs:254-348`; `core/src/context_manager/history.rs:587-671`; `history/src/retained_context.rs` | Current-format checkpoints preserve saved replacement history; rollback retains the surviving boundary. Legacy checkpoints can require reconstruction. |
| Diagnostics | `core/src/cache_diagnostics.rs:94-220`; `cache-diagnostics/src/manifest.rs:35-85,100-253` | Records logical components, ordered item/tool fingerprints, exact prepared wire observations, transport identity and returned usage. It observes requests rather than constructing a stable prefix for them. |

Paths in this table are relative to `codex-rs/`.

## What can reduce reuse

**Tools or base instructions change.** The builder serializes the current complete tool array and base instructions on each request. Adding/removing a tool, changing its description/schema, or changing base instructions can alter the early rendered prefix. In Responses Lite, the tool item is explicitly input position zero. This can sacrifice substantially more reuse than changing a late conversation message. Optional MCP server readiness is one concrete trigger, even without a user editing a prompt.

**Compaction replaces history.** It changes the sequence from its first differing position. Unchanged base instructions, tools, and any retained input prefix may still be reusable; compaction does not imply zero cached tokens. The old full conversation cannot remain an exact prefix once it has been replaced by a shorter representation.

**A new context window or fresh session reconstructs startup state.** Changed skills, recommendations, permissions, configuration, environment or extension content may then differ from another window/session. Ordinary updates in an established window are appended as diffs, so a date or environment change alone is not evidence that an already-sent early prefix was rewritten.

**Model or request controls change.** A model switch changes the computation being requested. Reasoning, text/output-schema and other request-property changes explicitly prevent the local WebSocket continuation from being reused. Their exact provider cache effect depends on provider rendering; the local compatibility check is not a specification of the provider cache key.

**History is repaired or reconstructed under different constraints.** An unmatched tool call/output, unsupported modality, changed truncation policy, or old-format compaction checkpoint can cause a different input sequence. Current-format resume and ordinary repeated normalization have deterministic protections. Rollback shortens history; a retained shorter prefix can still be reusable.

**Cache availability or identity changes outside the prompt.** Different scoped keys or provider/organization/region changes can change the reuse domain. Expiry and routing are provider-owned. No local source inspection can demonstrate that a matching server cache entry existed for a specific request.

## Protections and limits

- The fresh-subagent test checks equal model, instructions, ordered tools, reasoning, and shared startup input through the deliberate role boundary (`core/tests/suite/prompt_cache_key.rs:169-220`). Its input projection strips item IDs and internal metadata (`:52-57`). This is useful visible-prefix assurance, not proof of identical wire requests or server cache hits.
- Responses Lite generates prefix IDs from thread identity and payload, rather than fresh random IDs on every request (`core/src/client.rs:929-959`; existing test at `core/src/client_tests.rs:291-353`). Different threads intentionally receive different IDs. Their provider cache significance remains unproven here.
- MCP tools and namespace members have deterministic ordering. A malformed MCP server returning duplicate raw identities with different schemas could expose first-winner ordering (`codex-mcp/src/tools.rs:121-150`); this is a conditional malformed-input edge, not an established ordinary cache defect.
- Existing WebSocket tests exercise append reuse and full-request fallback after changed input/instructions (`core/tests/suite/client_websockets.rs:1842-1943,2080-2265`). These were read, not rerun.
- Exact request fingerprints include IDs, metadata and serialization details. A changed fingerprint alone does not establish a changed token prefix, and ordinary appended suffixes legitimately change whole-request fingerprints.

## Extension-producer closure

The production `ContextContributor` implementations were enumerated from source. The shared contract and registry are in `ext/extension-api/src/contributors.rs:78-126` and `ext/extension-api/src/registry.rs:211-214`. App-server installs the contributors in fixed order at `app-server/src/extensions.rs:75-133`; the CLI prompt builder uses git attribution and skills at `cli/src/main.rs:2348-2365`.

| Producer | Source relative to `codex-rs/` | Cache-relevant behavior |
|---|---|---|
| Skills | `ext/skills/src/extension.rs:185-268`; `ext/skills/src/world_state_catalogs.rs:139-168,251-334` | Initial thread catalog plus current catalog world state. Later catalog context updates append diffs. Explicit skill/plugin injections append turn input (`core/src/session/turn.rs:276-316`). |
| Memories | `ext/memories/src/extension.rs:51-76,92-101` | Thread-only initial policy; no turn/world-state override. A live config change can leave the earlier instruction stale until replacement context. This is a possible stale-context limitation, not evidence of cache busting. |
| History notes | `ext/history-notes/src/extension.rs:97-151` | Bounded remote thread hint during initial/full reconstruction. Backend state or failure can change a newly established window's hint. Existing prefix is not rewritten. |
| Git attribution | `ext/git-attribution/src/lib.rs:32-93`; `ext/git-attribution/src/world_state.rs:30-64` | World-state-only contribution, unchanged-state suppression, explicit enabled/disabled updates. Appends changes. |

These append guarantees apply to conversation input. A changed top-level tool array still changes the separately supplied request tools; it must not be confused with the appended world-state description of those tools.

## Method and boundaries

Used codebase-memory project `Users-viktor-Projects-github-codex`: symbol search, relationship tracing, snippets, and explicit file coverage checks. The recorded index generation was `2026-09-13T21:11:11Z`; relied-on source paths reported matching metadata and no recorded gaps. Graph results are best effort: source was read directly, and no claim of provider implementation coverage is made.

Two fresh-context Sol agents reviewed history and prefix/catalog construction; the coordinator reviewed request/transport construction and diagnostics. Historical research supplied leads only. No GitHub issues were re-fetched, no provider inference requests were made, and no builds or tests were run. This is a static source audit, not a measured cache-hit improvement or a reproduction of a historical incident.
