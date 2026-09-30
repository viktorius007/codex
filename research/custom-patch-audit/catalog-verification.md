PASS
Commit: `d5f8ef066a86255fc1e84c1b2a9759c222aa9f4d` plus uncommitted two-file diff, SHA-256 `4d30f33527a6b73a3ebf7040c555fc95bdcafc530605b5f4b58c4caad31026a6`.
Scope: read-only review of `codex-rs/ext/skills/src/tools/mod.rs` and `codex-rs/ext/skills/tests/plugin_skill_locators.rs`; no production or test edit, commit, or full-suite rerun.
Command: `CARGO_TARGET_DIR=/private/tmp/codex-patch-audit/catalog/codex-rs/target CARGO_BUILD_JOBS=32 just test -p codex-skills-extension -E 'test(disabled_plugin_skill_does_not_advertise_read_tool)'` → `1 test run: 1 passed, 175 skipped`.
Command: `git diff --check` → exit 0. Fixer reports the full crate suite at `176 passed, 0 skipped`; I did not rerun it.

| Criterion | Proof |
| --- | --- |
| Disabled-only and absent-plugin states should advertise the same tools | Test at `plugin_skill_locators.rs:435-487` constructs both states, compares empty rendered host catalogs, serializes each returned `ToolSpec`, and compares the JSON arrays. The pre-fix replay recorded `skills.read` versus `[]` at line 487 (`report.md`); my post-fix targeted run passed. |
| Enabled host plugin still exposes its reader | Test at `plugin_skill_locators.rs:414-433` asserts one `skills` namespace; `tools/mod.rs:86-121` retains the enabled plugin branch and pushes `ReadTool`. Existing package-read tests at `plugin_skill_locators.rs:270-391,493-623` remain in the reported green crate suite. |
| Orchestrator and executor tools retain their availability | `tools/mod.rs:84-95,115-121` leaves `orchestrator_available`, `executor_query`, and `list_available` intact; `extension.rs:290-327` still supplies the same per-turn inputs. |
| Provider catalog no longer changes solely for a disabled plugin | `provider/host.rs:195-232` retains the disabled entry, `catalog.rs:261-263` and `render.rs:499,548-558` omit it from the visible host prompt, and `tools/read.rs:109-125` rejects it at read lookup. The changed predicate at `tools/mod.rs:91` now agrees with that lookup. `tools/tool_executor.rs:113-115` gives this executor direct exposure, and `core/tools/spec_plan.rs:533-567,1407-1436` registers and selects directly exposed extension specs for a model request. |

Findings: none in the assigned diff.

Checked and found correct: the production edit is a one-site predicate with no new function, type, comment, indirection, or API change; it stays inside the existing 57-line `skill_tools` function. The test's new helper is test-only, uses fixture-created temporary files, and the test observes the public extension tool contribution rather than adding production visibility. Disabled and enabled are distinct config values, and the absent-plugin state independently fixes the expected empty spec. The red-to-green discriminator is the disabled config at `plugin_skill_locators.rs:435-455`; the recorded baseline red was an extra serialized `read` spec, and the green was independently run above.

Limits: the test serializes extension `ToolSpec`s, not the complete provider `/responses` payload, and does not measure provider cache hits. The direct-exposure/provider-plan source trace supports the claimed request-catalog effect under normal tool settings, but provider merging and runtime settings can affect the final request. The worktree is uncommitted, so no commit gate or mutation verdict exists for this diff. Codebase-memory coverage reported `no_recorded_issue` and matching metadata for all nine cited Rust paths, with partial Rust call semantics; source reads were used for the call chain and the two changed files were reviewed fully.
