# Stage 5 tool-stability final audit

## Verdict

**PASS.** Complete serialized tool ordering is protected and GREEN. The integrated MCP correction preserves sampled execution authority, lazy cache startup, and established approval behavior while failing closed for changed endpoint, permission, or schema. After the full workspace run exposed process-specific description drift, the bounded presentation correction passed the expanded focused gate: **322/322 tests GREEN** in `/private/tmp/codex-cache-run/lazy-presentation-green.log:131-164,363`. The final addendum records that closure.

Generic ignored `tools/list_changed` handling remains a separate stale-capability feature. A real published schema change should change the request prefix, so it is not an unchanged-inventory cache-bust cause and does not reopen the ordering disposition.

## Historical defect that required the final correction

The superseded implementation of `Session::prepare_mcp_call` refreshed dirty MCP state, then required pointer identity between the step binding's config and the current publication. `latest_mcp_desired_state` and `build_mcp_runtime_input` allocate a fresh `Arc<McpConfig>` for every claimed refresh (`mcp_runtime.rs:330-390`), and `McpRuntime::publish` stores that fresh Arc in every new publication (`codex-rs/codex-mcp/src/runtime.rs:282-319`). Pointer inequality therefore proved only that a publication occurred; it did not prove that execution authority changed.

This conflicts with the existing reconciliation design. `McpConnectionSet::new` deliberately reuses a live server connection when `McpServerConnectionIdentity` still matches (`codex-rs/codex-mcp/src/connection_manager.rs:92-137,371-470`). The identity includes transport, resolved environment handle, referenced environment values, runtime auth and token, OAuth settings and credentials, client extensions, and agent-plugin status (`codex-rs/codex-mcp/src/server.rs:97-257`). Tool policy and presentation metadata are publication state and intentionally do not force reconnection.

The new `sampled_mcp_call_executes_after_equivalent_runtime_republish` actual-handler test clones the fixture's unchanged runtime config after the request has advertised the MCP tool, forces the refresh to publish, then releases the sampled function call. It is RED twice in `/private/tmp/codex-cache-run/step-binding-equivalent-red.log`: neither endpoint receives the call and the model gets the unavailable-tool result solely because the new but semantically equal config Arc fails pointer identity.

HOME-isolated Guardian runs show the production consequence. A child request advertises and calls `tool_server/echo_tool` or `node_repl/echo_tool`; the item then fails with `MCP tool ... is not available to the model`, while the next step immediately reaches Guardian review for that tool (`/private/tmp/codex-cache-run/final-mechanism-and-workspace-fixtures.log`, first examples around lines 169-243 and 2206-2277). This rules out a genuinely absent tool. The exact dirty setter is not observable in the current warning-level log. Source inspection substantially rules out environment attachment promotion because children inherit the parent's ready exact `Environment` Arc, and rules out effective-plugin startup invalidation because `TestAppServerBuilder` disables plugin startup tasks by default. Auth/prewarm or a queued refresh remains possible, but the deterministic equivalent-republication RED independently establishes the guard defect.

## Correction design subsequently integrated

Keep `refresh_mcp_if_dirty()` before admission, obtain both the sampled prepared call and the current prepared call for the same server/tool, compare their execution authority, and return the **sampled** call only when compatible. Never return or execute the current call.

The compatibility predicate should live beside `PreparedMcpCall`, where its private execution fields are available, and require:

- the current binding still exposes and permits the same server/tool;
- pointer identity of the underlying `RmcpClient` and its tool catalog;
- the same captured catalog revision; and
- semantic equality of the two `McpConfig` values.

The initial implementation proposal was to add derived `PartialEq` at these definitions:

- `codex-rs/codex-mcp/src/mcp/mod.rs:123`: `McpConfig`;
- `codex-rs/codex-mcp/src/catalog.rs:508-509`: `ResolvedMcpCatalog`.

Final review found that derived catalog equality would compare insertion-order-sensitive `CatalogAction` history. The integrated correction instead gives `ResolvedMcpCatalog` semantic equality over its winning servers and disabled-name veto, as detailed in the superseding review below; `CatalogAction` does not derive equality.

The remaining constituent types already support equality, including `McpServerConfig`, `PermissionProfile`, `ConfigLayerStack`, `ConnectorSnapshot`, approval settings, and client elicitation capability. `McpConfig` contains semantic configuration only: it excludes submit/event handles, cancellation, runtime context and HTTP client, caches, auth/auth-manager, reviewer/lifecycle handles, startup policy, plugin-ready state, and selected-environment Arc maps. Whole-config equality is conservative because startup-only fields can cause rejection, but it safely admits an identical republication without a hand-picked security field list.

The predicate belongs in `codex-rs/codex-mcp/src/binding.rs:171-181` on `PreparedMcpCall`. The comparable client fields are private in `codex-rs/codex-mcp/src/rmcp_client.rs:116-126` on `ManagedClient`: `client: Arc<RmcpClient>` and `tool_catalog: Arc<ClientToolCatalog>`. It should compare the server and raw tool names, pointer identity for both those Arcs, catalog revision, and full `McpConfig` equality.

Current tool presence plus static server-config equality alone is insufficient. Identical TOML can hide a changed OAuth credential, bearer/header environment value, runtime account token, or resolved environment handle. Underlying client identity is the existing proof that connection reconciliation considered those inputs reusable. The current-call presence rejects an allowlist removal; whole-config equality rejects permission, approval, sandbox, layer, or server-policy changes; changed endpoint/auth/environment replaces the client; changed schema changes the catalog revision. The returned sampled call retains `run_with_revision` across irreversible preparation and dispatch, so no path silently selects N+1.

## Regression gate

The existing actual-handler fixture in `codex-rs/core/tests/suite/mcp_tool_cache.rs:275-478` already separates old and replacement JSON-RPC endpoints and pauses the model stream after advertisement. The equivalent-republish discriminator uses the same fixture and should require successful output, exactly one call to the sampled endpoint, and zero calls to the replacement endpoint after `refresh_runtime_config(fixture.config.clone())` and `wait_for_mcp_server`.

Keep the three existing controls GREEN:

- unchanged authority executes on N;
- endpoint replacement executes on neither N nor N+1;
- narrowed allowlist executes on neither N nor N+1.

Those three controls are GREEN in `/private/tmp/codex-cache-run/final-mechanism-and-workspace-fixtures.log:89-92`; the mixed complete-array ordering test is also GREEN there. The final Stage 5 verdict requires the equivalent-republication test plus all three controls to pass on the corrected integrated tree, followed by the HOME-isolated Guardian subset that exposed normal child false rejection.

## Complete serialized ordering

`mixed_tool_order_is_stable_across_mcp_insertion_orders` builds independent sessions with reversed `alpha`/`zeta` MCP insertion history and compares the complete serialized `tools` arrays for exact equality (`codex-rs/core/tests/suite/rmcp_client.rs:605-706`). Non-vacuity checks require canonical MCP namespace order, hosted `web_search`, and non-MCP `exec_command`. It is GREEN in `/private/tmp/codex-cache-run/step-binding-red-ordering-green-2.log:37` and the final focused log.

This audit ran no Cargo command and made no repository source changes. Codebase-memory coverage previously reported `no_recorded_issue` and `metadata_match` for the relied-on Core paths; direct reads of `/private/tmp/codex-cache-integration` are authoritative for the dirty integrated tree.

---

## Superseding review after the semantic-authority correction

**CONDITIONAL PASS.** This section supersedes the earlier FAIL verdict and proposed correction above. The integrated source now implements that correction without comparing order-sensitive catalog action history. Final Stage 5 PASS still requires the current integrated tree to run all four `sampled_mcp_call_*` tests and the HOME-isolated Guardian subset GREEN; the latest combined attempts stopped at unrelated `codex-memories-write` test-compilation errors before executing those gates.

`Session::prepare_mcp_call` retains the dirty refresh, prepares the requested server/tool from both the sampled step binding and the current publication, and returns only the sampled call when their execution authority matches (`codex-rs/core/src/session/mcp_runtime.rs:61-76`). It never substitutes the current N+1 call.

`PreparedMcpCall::has_same_execution_authority` requires the same server and raw tool name, pointer identity for the reconciled `RmcpClient` and its tool-catalog Arc, the same catalog revision, and semantic equality of `McpConfig` (`codex-rs/codex-mcp/src/binding.rs:168-193`). Current preparation must still expose and permit the server/tool. Client identity rejects endpoint, resolved-environment, runtime-authentication, token, OAuth, and connection-capability replacement. Tool-catalog identity and revision reject republished schema. `McpConfig` equality rejects permission, approval, sandbox, config-layer, server-policy, attribution, and protocol changes. Permission or allowlist narrowing therefore executes on neither N nor N+1; an equivalent publication may execute only on the sampled N connection.

`ResolvedMcpCatalog::PartialEq` now compares `disabled_server_names` and the `BTreeMap` of winning `ResolvedMcpServer` values (`codex-rs/codex-mcp/src/catalog.rs:490-520`). Each winner contains its source and full `McpServerConfig`; the disabled-name set is the persisted name veto. This is the effective catalog authority relevant to execution. Equality deliberately ignores the `actions` registration history and derived `conflicts` diagnostics, and `CatalogAction` no longer derives `PartialEq` (`catalog.rs:274-282`). That avoids the verified false rejection when equivalent same-tier registrations arrive in a different order.

There is one bounded API caveat: `to_builder()` preserves action history (`catalog.rs:528-532`), so two catalogs equal under this implementation can retain different reconstruction or diagnostic histories. The source comment accurately defines equality as effective server authority rather than structural-history equality. A tree-wide caller search found no direct catalog equality consumer; the new equality is currently reached only through `McpConfig` in `PreparedMcpCall::has_same_execution_authority`. No current caller can confuse it with `to_builder()` identity.

Whole-value `McpConfig` equality is conservative. It may reject a publication whose non-execution settings differ, but it cannot admit changed authority. It contains no event channels, cancellation handles, runtime HTTP clients, auth-manager handles, ready-environment Arcs, or publication counters. The independent client-identity check covers runtime auth and resolved-environment inputs supplied outside `McpConfig`.

The actual-handler fixture reverses two controlled, disabled same-tier extension registrations before rebuilding the same effective catalog (`codex-rs/core/tests/suite/mcp_tool_cache.rs:285-319,481-497`). Its oracle requires successful output, exactly one call to the sampled endpoint, and zero calls to the replacement endpoint. This directly protects both the fresh-Arc and registration-order representation boundaries. Companion controls cover unchanged authority, endpoint replacement, and narrowed allowlist.

The equivalent-republish case was RED twice under whole-config Arc identity in `/private/tmp/codex-cache-run/step-binding-equivalent-red.log`. The three earlier controls were GREEN in `/private/tmp/codex-cache-run/final-mechanism-and-workspace-fixtures.log:89-92`. The latest corrected-tree combined runs compiled the production crates, then stopped before test execution on unrelated `codex-memories-write` fixture compile errors: first `PathBuf` versus `AbsolutePathBuf` in `final-authority-memory-resume.log`, then private `Prompt` fields in `final-authority-memory-resume-2.log`. These are compilation evidence only.

Complete mixed tool-array ordering remains GREEN in `/private/tmp/codex-cache-run/step-binding-red-ordering-green-2.log:37`. The remaining focused authority gate is:

```sh
just test -p codex-core -E 'test(sampled_mcp_call_)'
```

All four sampled-call tests must pass, followed by the HOME-isolated Guardian tests that originally exposed equivalent-republication rejection. This read-only review ran no Cargo command and changed no repository source. `git diff --check` passes for the reviewed production and regression files.

---

## Final integrated review after lazy-cache correction

**PASS.** This section is the authoritative final disposition and supersedes the conditional wording and ready-client-only design discussion above.

`Session::prepare_mcp_call` refreshes dirty runtime state, prepares the requested call from the current publication, and asks the sampled step binding to admit it (`codex-rs/core/src/session/mcp_runtime.rs:61-75`). `McpBinding::prepare_call_if_current` has two deliberately different paths (`codex-rs/codex-mcp/src/binding.rs:99-127`):

- When the sampled binding already owns a ready call, it returns that exact sampled call only if the refreshed call has the same raw server/tool, `RmcpClient` Arc, tool-catalog Arc, catalog revision, and semantic `McpConfig`.
- When the sampled binding advertised a cached tool before lazy startup, it never starts a retired sampled connection. It accepts the already-live current call only if the sampled and current configurations are equal, the server connection is the same Arc or has the same complete reusable connection identity, and the complete frozen `ToolInfo` matches the live tool after applying the cache's documented annotation redaction. Generic MCP caches omit all annotations; the Apps cache retains annotations but omits `read_only_hint` until startup. A changed schema or other model-visible metadata therefore rejects admission.

The lazy path returns the verified **live** call. This preserves the existing approval contract: generic cached annotations are deliberately temporary because annotations affect approval and parallelism and may only come from the live connection (`codex-rs/codex-mcp/src/tool_catalog_cache.rs:193-200`). MCP approval metadata and trusted-access checks consume `PreparedMcpCall::tool_info`; returning frozen redacted metadata would add an incorrect pre-call review and alter Guardian behavior. The live call can supply execution annotations only after configuration, permission, connection identity, tool presence, and frozen schema checks succeed. Permission or allowlist narrowing, endpoint/auth/environment replacement, and schema change still fail closed rather than selecting a different authority.

Regression coverage now protects both paths:

- Four actual-handler tests prove unchanged and semantically equivalent authority execute successfully, while endpoint replacement and narrowed allowlist execute on neither the old nor replacement endpoint (`codex-rs/core/tests/suite/mcp_tool_cache.rs:275-552`).
- Two connection-manager tests prove a lazy Apps tool accepts an identical live schema with live execution annotations and rejects a changed live schema (`codex-rs/codex-mcp/src/connection_manager_tests.rs:2675-2790`).
- The HOME-isolated Guardian suite exercises the generic cache path and its established live approval/elicitation lifecycle.

All four sampled-call tests pass at `/private/tmp/codex-cache-run/final-live-execution-authority.log:125-128`; both lazy-cache tests pass at `:157-158`; the complete focused run finishes **318 passed, 0 failed** at `:359`. Temporary authority diagnostics are absent, and `git diff --check` passes for the reviewed MCP production and regression files. Complete mixed serialized ordering remains GREEN at `/private/tmp/codex-cache-run/final-mechanism-and-workspace-fixtures.log:2454`; no later MCP change modifies tool ordering or serialization.

The ignored generic `tools/list_changed` behavior remains intentionally outside this Stage 5 result. A real schema publication changes the serialized request inventory and should change the cache prefix; it is not evidence of an ordering defect.

---

## Full-workspace addendum: dynamic cached descriptions

**CLOSED; PASS.** The first full workspace run reproduced `cached_mcp_startup_is_eager_for_root_and_lazy_for_subagents` twice with an unavailable-tool result (`/private/tmp/codex-cache-run/final-full-workspace.log:6942-6985`). The fixture intentionally caches an eager stdio process's server instructions and echo description, starts a new lazy process for the subagent, and requires the first cached call to execute on that new process (`codex-rs/core/tests/suite/mcp_tool_cache.rs:843-1138`). The stdio test server embeds its PID in exactly those two descriptive fields (`codex-rs/rmcp-client/src/bin/test_stdio_server.rs:53-60,168-172,542-550`).

The corrected lazy admission check normalizes only `namespace_description` and `tool.description` to their frozen values in the comparison clone, then retains the existing annotation normalization and whole-object equality (`codex-rs/codex-mcp/src/binding.rs:114-130`). This keeps input/output schemas, names, title, icons, execution metadata, provenance, file fields, permissions, connection identity, and cached Apps safety hints strict. It returns the verified live call so established approval and reporting behavior sees live metadata.

The expanded focused gate includes the entire Core MCP tool-cache suite, all `codex-mcp` tests, and all app-server Guardian tests. The formerly failing lifecycle test passes at `/private/tmp/codex-cache-run/lazy-presentation-green.log:131`; all four sampled-call controls pass at `:132-138`; both lazy-cache unit regressions pass at `:163-164`; the run finishes **322 passed, 0 failed** at `:363`. No Cargo command was run by this auditor, temporary diagnostics remain absent, and `git diff --check` passes for the final binding correction.
