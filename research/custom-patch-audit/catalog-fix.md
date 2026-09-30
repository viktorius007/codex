STATUS: DONE — the disabled-plugin tool-catalog defect is repaired in this worktree; no commit made.
CHANGED: `codex-rs/ext/skills/src/tools/mod.rs:91` now requires `entry.enabled && entry.is_plugin_package()` before advertising `skills.read`. The existing candidate test in `codex-rs/ext/skills/tests/plugin_skill_locators.rs` was preserved unchanged.
TESTS: Focused regression 1 passed; full `codex-skills-extension` suite 176 passed; `just fmt` passed; `git diff --check` passed.
BLOCKERS: None. The worktree's Git index is outside the writable sandbox, so `git restore` was denied; the formatter's unrelated Python changes were restored from `HEAD` without touching the index.

The host catalog retains disabled plugin entries, but `skills.read` rejects them. The old availability check counted those entries and added a tool to the provider catalog even when the rendered host prompt had no skill. Requiring `enabled` aligns advertised capability with the read lookup in `ext/skills/src/tools/read.rs:115-122`. The existing test compares the serialized tool specs for disabled-only and absent-plugin snapshots, with an enabled plugin as a positive control.

Commands and results (from `codex-rs` unless noted):

- `CARGO_TARGET_DIR=/private/tmp/codex-patch-audit/catalog/codex-rs/target CARGO_BUILD_JOBS=32 just test -p codex-skills-extension -E 'test(disabled_plugin_skill_does_not_advertise_read_tool)'` → exit 0; `1 test run: 1 passed, 175 skipped`.
- `CARGO_TARGET_DIR=/private/tmp/codex-patch-audit/catalog/codex-rs/target CARGO_BUILD_JOBS=32 just test -p codex-skills-extension` → exit 0; `176 tests run: 176 passed, 0 skipped`.
- `UV_CACHE_DIR=/private/tmp/codex-patch-audit/catalog/.uv-cache just fmt` → exit 0. Three unrelated Python files changed; restored their `HEAD` contents. The task-created `.uv-cache` directory was removed.
- `git diff --check` → exit 0. Final tracked diff is the one-line production change plus the prior owner's candidate test. `findings.json` and `report.md` remain untouched. Free disk at cleanup check: 62 GiB.
