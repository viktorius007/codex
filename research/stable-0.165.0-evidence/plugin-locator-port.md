# Port: stable locators for host plugin skills onto rust-v0.165.0

Worktree `/home/agent/src/codex-wt/probe-0.165.0`, branch `probe/0.165.0`, commit `df658d8ce2` "Use stable locators for host plugin skills" (single commit on top of the probe test adaptation). Reference: `fe77d81f19` + `bfd549618b` on `local/customizations`.

## Files changed (codex-rs/ext/skills/src)

| File | Change |
|---|---|
| `catalog.rs` | `SkillCatalogEntry::is_plugin_package()` (Host authority and `plugin_id` is Some). |
| `provider/host.rs` | `plugin_skill_locator()`; plugin entries built with the stable package and `with_plugin_id`, with no display path and no alias root; plugin-package branch in `read()`. |
| `render.rs` | Plugin entries render `entry.id` with locator kind `plugin package` (both `SkillLine::new` and `render_skill_locator_with_aliases`); `prompt_kind_for_entries()` picks `HostPluginPackages` when any entry is a plugin package. |
| `catalog_prompt.rs` | `HostPluginPackages` kind with its intro and usage text (the same text as the old patch); the `skills.read` package hint also appears for `(plugin package: ` lines. |
| `step_state.rs` | New `SkillsStepState::host_snapshot: Option<Arc<HostSkillsSnapshot>>`. |
| `preparation.rs` | Fills `host_snapshot` from the turn store, using the same provider gate as `discover_host_catalog`. |
| `tools/mod.rs` | `skill_tools` registers `skills.read` alone when `tools_enabled` is false but the host catalog has enabled plugin packages. Order is otherwise unchanged: list, read, then installations (v2 only). |
| `tools/read.rs` | Fallback plugin-package lookup; `output_authority` is now `Option<SkillToolAuthoritySelector>` (None = host plugin package). |
| `invocation_tests.rs`, `shadow_selection_experiment/experiment_tests.rs` | Upstream tests: `host_snapshot: None` added to their `SkillsStepState` literals (see below). |

10 files, +242/-48.

## Design choice per behaviour

- **Stable package id, main prompt and catalog rendering.** `plugin_skill_locator` returns `skill://<plugin_id>`, or `skill://<plugin_id>/<dir>` where `<dir>` is the parent of the loader-retained logical discovery path (`skill_discovery_path_for_path`) relative to `skill_root_for_path`. Windows separators are turned into `/`. The main prompt is `<package>/SKILL.md`. In `catalog_from_outcome`, plugin entries skip `with_display_path` and `with_alias_root`. They therefore add nothing to `shared_host_alias_roots` and produce no `rN` root lines, so no cache revision path can reach the rendered catalog. Rendering is a `SkillSourceKind::Host if entry.is_plugin_package()` arm added to the two existing locator matches and to the locator-kind match. This is the same shape as the old patch, placed in 0.165's code.
- **Symlinks.** The package name comes from the logical discovery path, as in `bfd549618b`. Reads use `skill.path_to_skills_md` (the canonical target) as the package root, canonicalize the requested path through the skill's filesystem (`file_system_for_skill`, falling back to `LOCAL_FS`), and require `starts_with(package_root)`.
- **`skills.read` through the active snapshot.** 0.165 builds tools from `SkillsStepState`, so the old `SkillToolCatalogInputs` plumbing in `extension.rs` was dropped. The step state now captures the host snapshot during `prepare_for_step`, next to the `host_catalog` it already held. Reads first use upstream's `matching_packages` (Cloud and executor). If nothing matches and no `source` filter is given, the read looks up an enabled plugin-package entry in `step_state.host_catalog` and reads with `SkillReadContext::Host { host_snapshot: step_state.host_snapshot }`. A package that existed in an older cache revision has the same id, so it resolves to the active step snapshot's files.
- **Containment.** The host provider rejects any resource that is not under the package prefix (`SkillPackageId::relative_resource_path`), that contains non-`Normal` path components, or whose canonical target falls outside the package directory. All three cases reach the model as "failed to read skill resource".
- **Response shape and paging.** Upstream's paging and metadata code is reused. With v1 the legacy metadata has no `skill_root`; with v2, installations are empty because `InstallationLocation::from_source` returns None for Host. Output goes through the plain (non-external-context) JSON path via `unwrap_or(Executor)`. Plugin reads do not use the executor read-snapshot cache, because `access_key` is None. Analytics records an implicit invocation for a main-prompt read, as the old patch did.
- **Ordinary host skills.** These are unchanged: the same `catalog_entry_from_skill` values (now in the `else` branch), the same display path and alias root, upstream's `PathUri`-based direct prompt read in the host provider, and the same `rN` aliases. `plugin_id` is set only on plugin-package entries; upstream never set it on host entries.
- **Prompt intro.** 0.165's `SkillPromptKind` gains one variant, `HostPluginPackages`, which has no alias instructions. `CatalogLines::unaliased` and `CatalogLines::aliased` route their existing kind through `prompt_kind_for_entries`, so the variant applies only when plugin packages are present. With no plugin packages every upstream kind (including `ResourceAliasesV2`) is chosen exactly as before, so the rendered text for every other case is byte-identical.

## Tests

`cargo nextest run -p codex-skills-extension --no-fail-fast` (from `codex-rs`, after `source ~/.profile`), after fmt and the clippy fix:

```
     Summary [   1.160s] 216 tests run: 216 passed, 0 skipped
```

All three `plugin_skill_locators` tests pass:

```
PASS codex-skills-extension::plugin_skill_locators ordinary_host_skills_keep_filesystem_locators_and_direct_prompt_reads
PASS codex-skills-extension::plugin_skill_locators plugin_package_subresources_follow_the_active_snapshot_and_stay_within_the_package
PASS codex-skills-extension::plugin_skill_locators plugin_packages_stay_stable_across_cache_revisions_and_resolve_the_active_snapshot
```

No upstream snapshot changed and no `.snap.new` files were produced: no upstream skills test renders plugin-backed host skills.

## fmt and clippy

- `cargo fmt -p codex-skills-extension`: applied. The only output is the stable-toolchain notice that `imports_granularity` is unavailable.
- `cargo clippy -p codex-skills-extension --tests`: initially reported one `needless_borrows_for_generic_args` (`.ok_or_else(&outside_package)` in `provider/host.rs`), which I fixed. It now finishes with no warnings.

## Upstream tests changed

- `src/invocation_tests.rs` and `src/shadow_selection_experiment/experiment_tests.rs` build `SkillsStepState` with struct literals. I added `host_snapshot: None,` to each literal because the struct gained a field. Assertions are unchanged.

## Notes for the coordinator

- `codex-rs/Cargo.lock` was already modified in this worktree before I started: workspace member versions read `0.0.0` -> `0.165.0`. I left it out of the commit and did not modify it. `PROBE-REPORT.md` and this report are untracked.
- Incidental finding, not changed (the old patch has the same behaviour): when a plugin skill is mentioned explicitly through the extension's turn-input path, `extension.rs` records `injected_host_skill_prompts.insert_path(entry.main_prompt.as_str())`. For plugin packages that value is now `skill://.../SKILL.md` rather than the filesystem path, so core's path-based `InjectedHostSkillPrompts::contains_path` check may not recognise the prompt as already injected. That path only runs when the host catalog is not in world state. It needs a focused check if duplicate injection matters.
- Symlinked plugin-directory identity (`bfd549618b`) is implemented, but this test file does not exercise it.
