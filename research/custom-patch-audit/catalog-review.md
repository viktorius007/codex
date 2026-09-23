STATUS: PROVEN DEFECT
COMMIT: d5f8ef066a86255fc1e84c1b2a9759c222aa9f4d
CHANGED: Test-only candidate in `codex-rs/ext/skills/tests/plugin_skill_locators.rs`; no production edits or commits.
OPEN: Coordinator to choose and verify the production repair; failing test stays in this worktree.
BLOCKERS: Repository `just fmt` failed only in Python formatters because the sandbox denied writes to `~/.cache/uv`; `cargo fmt -p codex-skills-extension -- --check` passed.
FILES_REVIEWED: `codex-mcp/src/{binding,catalog,connection_manager,connection_manager/tool_catalog,tools}.rs`; `core/src/{mcp_tool_exposure,session/mcp_runtime,session/mod,session/turn_context,tools/handlers/mcp,tools/router,tools/spec_plan,agent/role}.rs`; `ext/skills/src/{catalog,catalog_prompt,extension,host_service,provider/host,render,state,tools/mod,tools/read,world_state,world_state_catalogs}.rs`; related MCP, core, and skills tests; `models-manager/src/manager.rs`.
PROVEN_DEFECTS: 1 (disabled plugin advertises an unusable model tool)

---

## Finding: disabled plugin changes the tool catalog without changing skill capability

- **Severity:** P2, significant unnecessary provider cache invalidation when a disabled-only plugin is installed or removed during a thread. The extra `skills.read` schema also occupies prompt space while it cannot read that package.
- **Mechanism:** `ext/skills/src/provider/host.rs:192-232` retains disabled plugin entries in the host catalog. `ext/skills/src/tools/mod.rs:86-95` treats any plugin entry as sufficient to register `skills.read`, without checking `enabled`. Actual read lookup at `ext/skills/src/tools/read.rs:109-125` requires `entry.enabled` and refuses the package. The returned extension tool has direct exposure (`tools/src/tool_executor.rs:113-115`) and is registered into the model tool plan by `core/src/tools/spec_plan.rs:1407-1436`; that plan serializes direct tools into provider requests at `core/src/tools/spec_plan.rs:533-567`.
- **Production reachability:** `ext/skills/src/world_state_catalogs.rs:205-238` stores a host catalog for each turn, and `ext/skills/src/extension.rs:295-326` passes it to `skill_tools`. The upstream `rust-v0.156.1` `skill_tools` returned no tools without executor or orchestrator skills; the local plugin-locator patch introduced this condition.
- **Named test and command:** `disabled_plugin_skill_does_not_advertise_read_tool`, `CARGO_TARGET_DIR=/private/tmp/codex-patch-audit/catalog/codex-rs/target CARGO_BUILD_JOBS=32 just test -p codex-skills-extension -E 'test(disabled_plugin_skill_does_not_advertise_read_tool)'` from `codex-rs`.
- **Exact red:** Nextest reported `1 test run: 0 passed, 1 failed, 175 skipped`; panic at `ext/skills/tests/plugin_skill_locators.rs:487:5`: disabled-only serialized tool specs were `[Namespace { name: "skills", tools: [read] }]`, while no-plugin specs were `[]`.
- **Oracle:** The test uses one plugin root under a temporary home in three states. Enabled plugin is the positive control and produces one `skills` namespace spec. Disabled-only and no-plugin both render no host skill prompt, and no-plugin produces zero specs. The final assertion compares their independently serialized tool specs; it fails only because the disabled-only state adds `read`. Test diff: `ext/skills/tests/plugin_skill_locators.rs:179-219,394-487`.

## Checked and found correct

- MCP tool normalization sorts by raw tool identity (`codex-mcp/src/tools.rs:197`), so map iteration in connection capture does not reorder the advertised MCP tools. Sampled-call validation preserves the original client and catalog when a live prepared call exists (`codex-mcp/src/binding.rs:101-135,221-231`); core uses that sampled binding at dispatch (`core/src/tools/handlers/mcp.rs:178-189`).
- Model presets are sampled after the session startup model refresh (`core/src/session/mod.rs:638-662`) and reused within the thread (`core/src/session/turn_context.rs:921-931`). Role descriptions are similarly cached by the configured role map (`core/src/session/turn_context.rs:938-958`).
- Plugin package locators omit cache-revision paths, and the existing integration tests cover stable presentation, active-snapshot reads, and package boundary checks (`ext/skills/tests/plugin_skill_locators.rs:260-393,491-590`).

## Verification limits

- The new test proves the exact tool-spec output from the extension; source traces show how those specs enter the provider request. It does not measure provider cache usage or run a live provider.
- Codebase-memory `check_index_coverage` returned `no_recorded_issue` and `metadata_match` for the 28 relied-on Rust paths, with `semantic_partial` Rust call/implementation coverage. Call-chain conclusions above were checked against source because the graph may omit Rust edges.
- `just fmt` hit the local UV-cache sandbox error described above; scoped Rust formatting and `git diff --check` passed.
