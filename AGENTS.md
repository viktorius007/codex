# Repository working principles

- Index every generated document and report in root `AGENTS.md` with a direct link and a clear, simple, single-line description of its contents; update the index when files are added, moved, or removed.

- The user is a layperson, not a software engineer. Explain technical decisions, consequences, and risks in plain language, and do not assume software-development knowledge.
- The user's computer is a MacBook Pro with an M3 Max, 16 CPU cores, 48 GB of memory, and a 1 TB NVMe drive. CPU and memory are rarely constraints, but disk space must be checked during work. Clean up agent-created build artifacts and temporary files proactively, and always perform a cleanup check before ending a session while preserving unknown or user-created files.
- Treat every rule, preference, approach, expression of intent, and other instruction from the user as durable across sessions unless the user explicitly marks it as session-scoped. Persist each durable instruction in this root `AGENTS.md` so it is available at the start of future sessions.
- Once the user's intent and scope are established, work independently within them. Do not assume either when they are unclear; clarify before starting and clarify again if material ambiguity or unexpected information would materially change the planned work.
- Before planning or choosing an approach, inspect the local repository constraints that govern the task, including this `AGENTS.md`, branch and worktree state, uncommitted changes, available disk space, and relevant local tools and configuration. Derive decisions from that evidence instead of generic assumptions.
- Optimize repository work for safe concurrent agents. Keep the root checkout for coordination and integration; give every file-writing agent its own local Git worktree and dedicated branch with non-overlapping ownership, integrate through small commits into the branch required by the rules below, and remove finished worktrees and obsolete branches after verifying integration.
- Optimize local build and test wall time for concurrent worktrees from measured local evidence. Keep per-worktree `target/` directories isolated, keep Cargo at 32 build jobs with development incremental compilation enabled on this 16-core Mac, and adopt a compiler cache only after cold and warm benchmarks demonstrate Rust hits across different absolute worktree paths and a net wall-time benefit. Rebenchmark after changes to the CPU count, open-file limit, Rust/Cargo toolchain, or cache implementation; use the [local build experiment](docs/experiments/local-build-pipeline.md) as the baseline.
- Keep this clone current with upstream and organize local changes so upstream alignment remains quick and low-friction. Keep `main` as a fast-forward-only mirror of `origin/main`; base `local/customizations` on the current stable `rust-v*` release tag rather than `origin/main`, and rebase it onto each newer stable release after that release is selected for local use.
- Give locally patched builds the stable base version plus SemVer build metadata in the form `<stable-version>+local.<revision>`, incrementing the local revision when the patch set changes. Keep `codex-rs/Cargo.toml` and `codex-rs/Cargo.lock` aligned with that version so the binary is distinguishable from the upstream release artifact.
- Building `codex-code-mode-host` (or anything pulling the `v8` crate with the `v8_enable_sandbox` feature) fails on this machine with a 404: the build script asks the upstream `denoland/rusty_v8` release for a `ptrcomp_sandbox` prebuilt archive that upstream does not publish. The matching archive and Rust binding are published under the `openai/codex` release tag `rusty-v8-v150.4.0` and are cached, checksum-verified, in `~/.cache/rusty-v8-codex/`. Build with `RUSTY_V8_ARCHIVE=~/.cache/rusty-v8-codex/librusty_v8_ptrcomp_sandbox_release_aarch64-apple-darwin.a.gz RUSTY_V8_SRC_BINDING_PATH=~/.cache/rusty-v8-codex/src_binding_ptrcomp_sandbox_release_aarch64-apple-darwin.rs` (absolute paths). When the pinned `v8` version changes, download the same-named files from the new `rusty-v8-v<version>` tag of `openai/codex` and verify them against the published `.sha256` before use.
- Prioritize fixing bugs locally and quickly over preparing changes for upstream submission. Use a dedicated `work/issue-<number>` branch for each GitHub issue and record work as small, coherent commits that can be cherry-picked independently; do not add pull-request ceremony unless the user is considering upstream submission.
- When preparing an upstream submission, create a clean branch from `origin/main` and cherry-pick only the relevant issue commits so private local policy and unrelated work are excluded.
- Take responsibility for repository housekeeping, including cleaning up worktrees, branches, and temporary files. Remove artifacts created by agents and branches or worktrees that are merged or demonstrably obsolete; preserve unknown uncommitted work and user-created artifacts.
- Apply the Pareto principle to all work in this repository: identify and pursue the most direct, surgical approach that delivers the greatest value for the effort. Multiple options may be considered, but prefer the highest-value option.
- Continuing work beyond the Pareto line requires the user's approval.
- For prompt-cache work, prioritize privacy-safe request-fingerprint diagnostics with bounded individual records and memory first, core prefix construction second, session compaction third, fresh-context subagents fourth, and tool stability fifth. Prioritize cache reuse for fresh-context subagents over forked-history subagents; forked-history behavior belongs later because it is less common and avoidable.
- Treat completed prompt-cache issue discovery and issue reads as durable research. Refresh incrementally: inspect only newly found or newly linked issues, and re-read an existing issue only when its recorded update time or state has changed or implementation work needs a specific detail that the ledger does not contain.
- For orchestrated work, subagent terminal status must wake or continue the parent with one bounded completion event; do not require fixed-interval `wait_agent` timeouts that cause repeated model requests merely to discover completion. Prevent duplicate completion when an active wait already consumed the same status.
- Always create subagents with fresh context. Never create them by forking existing context.
- Launch the Codex harness with an inherited open-file soft limit of at least 4096 and verify it before concurrent work. Raising a child shell limit does not raise the already-running host limit. Investigate recurring exhaustion instead of relying on repeated retries.
- Preserve cache diagnostic evidence indefinitely: no disk cap, age limit, or automatic deletion. Cleanup applies to agent-created temporary/build artifacts, not diagnostic evidence.
- For harness reliability audits, use local session records under `~/.codex/` as read-only incident evidence when relevant; keep reports sanitized, preserve original records, and surface significant adjacent issues to the coordinator.
- For cache repair work, adapt priorities to new evidence within the agreed Pareto scope; treat GitHub proposals as clues and choose the simplest locally justified root-cause fix with regression protection. Keep a durable run ledger and small independently applicable commits.
- Use Sol for load-bearing investigation, architecture, implementation decisions, and verification; use Terra only for basic, tightly scoped work. Delegate independent work with fresh context; the coordinator owns small corrections, integration, and cleanup.
- Use only Luna for live testing because it is the cheapest model. This overrides other model-selection guidance for live tests.
- Assert the mechanism, not the clock: regression tests should observe exact causal events and outcomes; use timeouts only as hung-test guards, not as substitutes for synchronization or correctness assertions.
- For Goal and asynchronous-tool workflow assessments, map the complete control flow before ranking fixes. Judge behavior from the consuming agent's perspective: timely useful results, no empty polling turns, no unwanted interruption, and no lost or duplicate completions. Prefer the smallest correct local patches with minimal internal API changes so upstream rebases remain simple; keep investigation outputs bounded.
- For Goal and asynchronous-tool implementation, keep edits within the agreed fixes and hold incidental findings for the final response.
- Cache repair full-suite tests are authorized. The deliverable is a trustworthy installed local binary; build and install validated milestones when this does not disrupt the running harness, preserving the previous binary and recording installation time/version.
- Cache repair verification must cover ordinary and post-compaction cold resumes within the provider cache lifetime, checking preserved input prefixes and unchanged effective model, instructions, settings, and ordered tools separately from provider-reported cache reuse.
- When auditing local patches for cache misses or wasted turns, focus on significant defects, require executed failing tests that assert the causal mechanism, group findings by severity, and have the coordinator choose the simplest surgical fix before delegating implementation to Sol. Retire this manual review rule when an automated gate enforces it.

## Syncing to a new upstream stable release

Apply this workflow only when the user requests moving `local/customizations` to a newer upstream stable `rust-v*` release tag. Do not start it for `origin/main` updates, alpha tags, routine code changes, or merely discovering a new tag.

- Before rebasing, verify the selected stable tag and current base; inspect branch, worktrees, uncommitted changes, free disk, installed version, rollback release, and the local patch set. For each patch, check whether upstream already supplies its behavior. Ask the user only when dropping a patch or resolving a significant functional tradeoff is necessary.
- Adapt the patches, then build the CLI and code-mode host early to catch cross-crate compile failures. Run focused tests for changed crates and preserved local behaviors. Group failures by cause, check for test-host contamination, and resolve them with focused reruns before the workspace suite.
- Finish generated fixtures, formatting, and scoped lint before the final suite; if any of those change code, recheck the affected scope. Once the tree is frozen, run the complete workspace `just test` once. Rerun the complete suite only when a later change invalidates that result or a failure remains unresolved. This final suite is authorized for a requested stable-release sync without separate approval.
- For a test close to its timeout, use measured isolated runs to justify a test-specific timeout change; do not weaken assertions or broadly increase timeouts.
- Let long-running commands report completion instead of polling logs for progress. Read bounded failure summaries when a command finishes, and update the user at meaningful milestones or blockers rather than per-test counts.
- Build the final binaries from the verified tree. Check version, checksums, architecture, and CLI/host launch commands; preserve the previous installation, atomically switch to the versioned release only after candidate checks, then verify the installed commands and record provenance. Complete the cleanup check without deleting diagnostic evidence or unknown files.
- After a verified rebase, compare the previous fork branch's local commits with the rebased patch series, and preserve its tip in a local backup ref until the new branch is published and checked. Publish `local/customizations` to `fork` with an explicit `--force-with-lease` bound to the remote tip observed immediately before pushing, then verify the remote tip equals the local tip. A normal push cannot advance the old release-based history, and `git pull` would merge that old history into the new stable base.

Retire a manual step above when a repository command enforces that step mechanically.

# Local exec-completion customization

This checkout carries a local patch derived from `tekacs/codex@9ffcf8db9078eae43d4111ff94259795c1e962c9`. Preserve the behavior below when rebasing onto a newer stable release, adapting the implementation when upstream code changes.

- When `exec_command` returns a live session, process exit must inject one bounded `ExecCompletion` and wake or continue the model without passive polling.
- Commands that finish inside the initial `exec_command` call must not emit a duplicate completion.
- Keep completion output capped at 1,000 tokens.
- Keep the `exec_command`, `write_stdin`, and bundled model guidance aligned: do not poll only to detect completion; use `write_stdin` for interaction, required intermediate output, or diagnosis.
- Treat changes to `codex-rs/core/src/context/exec_completion.rs`, `codex-rs/core/src/session/inject.rs`, `codex-rs/core/src/unified_exec/`, `codex-rs/core/src/tools/handlers/shell_spec.rs`, or `codex-rs/models-manager/models.json` as requiring preservation review.
- After an upstream update affecting those paths or turn admission, run `just test -p codex-core -E 'test(background_exec_completion_starts_a_follow_up_turn_without_polling)'` from `codex-rs` before rebuilding `~/.local/bin/codex`.

# Rust/codex-rs

In the codex-rs folder where the rust code lives:

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- When you cannot make that API change and still need a small positional-literal callsite in Rust, follow the `argument_comment_lint` convention:
  - Use an exact `/*param_name*/` comment before opaque literal arguments such as `None`, booleans, and numeric literals when passing them by position.
  - A method's sole non-self argument is exempt when the method and parameter names match, such as `.enabled(false)` for `fn enabled(&self, enabled: bool)`.
  - Do not add these comments for string or char literals unless the comment adds real clarity; those literals are intentionally exempt from the lint.
  - The parameter name in the comment must exactly match the callee signature.
  - You can run `just argument-comment-lint` to run the lint check locally. This is powered by Bazel, so running it the first time can be slow if Bazel is not warmed up, though incremental invocations should take <15s. Most of the time, it is best to update the PR and let CI take responsibility for checking this (or run it asynchronously in the background after submitting the PR). Note CI checks all three platforms, which the local run does not.
- When possible, make `match` statements exhaustive and avoid wildcard arms.
- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Discourage both `#[async_trait]` and `#[allow(async_fn_in_trait)]` in Rust traits.
  - Prefer native RPITIT trait methods with explicit `Send` bounds on the returned future, as in `3c7f013f9735` / `#16630`.
  - Preferred trait shape:
    `fn foo(&self, ...) -> impl std::future::Future<Output = T> + Send;`
  - Implementations may still use `async fn foo(&self, ...) -> T` when they satisfy that contract.
  - Do not use `#[allow(async_fn_in_trait)]` as a shortcut around spelling the future contract explicitly.
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- Do not add tests for values that are statically defined.
- Do not add negative tests for logic that was removed.
- Do not add general product or user-facing documentation to the `docs/` folder. The official Codex documentation lives elsewhere. The exception is app-server API documentation, which is covered by the app-server guidance below.
- Prefer private modules and explicitly exported public crate API.
- If you change `ConfigToml` or nested config types, run `just write-config-schema` to update `codex-rs/core/config.schema.json`.
- When working with MCP tool calls, prefer using `codex-rs/codex-mcp/src/mcp_connection_manager.rs` to handle mutation of tools and tool calls. Aim to minimize the footprint of changes and leverage existing abstractions rather than plumbing code through multiple levels of function calls.
- Do not call `reset_client_session` unnecessarily; let the incremental check logic decide whether to reuse the previous request.
- If you change Rust dependencies (`Cargo.toml` or `Cargo.lock`), run `just bazel-lock-update` from the
  repo root to refresh `MODULE.bazel.lock`, and include that lockfile update in the same change. CI
  verifies lockfile drift.
- Bazel does not automatically make source-tree files available to compile-time Rust file access. If
  you add `include_str!`, `include_bytes!`, `sqlx::migrate!`, or similar build-time file or
  directory reads, update the crate's `BUILD.bazel` (`compile_data`, `build_script_data`, or test
  data) or Bazel may fail even when Cargo passes.
- Do not create small helper methods that are referenced only once.
- For tracing async work, instrument the function or method definition with
  `#[tracing::instrument(...)]` instead of attaching spans to futures with
  `.instrument(...)` at call sites. Before adding instrumentation, check whether the callee—or
  the implementation method it immediately delegates to—is already instrumented.
- Avoid large modules:
  - Prefer adding new modules instead of growing existing ones.
  - Target Rust modules under 500 LoC, excluding tests.
  - If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending
    the existing file unless there is a strong documented reason not to.
  - This rule applies especially to high-touch files that already attract unrelated changes, such
    as `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/bottom_pane/chat_composer.rs`,
    `codex-rs/tui/src/bottom_pane/footer.rs`, `codex-rs/tui/src/chatwidget.rs`,
    `codex-rs/tui/src/bottom_pane/mod.rs`, and similarly central orchestration modules.
  - When extracting code from a large module, move the related tests and module/type docs toward
    the new implementation so the invariants stay close to the code that owns them.
  - Avoid adding new standalone methods to `codex-rs/tui/src/chatwidget.rs` unless the change is
    trivial; prefer new modules/files and keep `chatwidget.rs` focused on orchestration.
- When running Rust commands (e.g. `just fix` or `just test`) be patient with the command and never try to kill them using the PID. Rust lock can make the execution slow, this is expected.

Run `just fmt` (in the `codex-rs` directory) automatically after you have finished making code changes anywhere in this repository; do not ask for approval to run it. Additionally, run the tests:

1. Do not run `cargo test` directly. Use `just test` so test execution follows the repo defaults.
2. Run the test for the specific project that was changed. For example, if changes were made in `codex-rs/tui`, run `just test -p codex-tui`.
3. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `just test`. Avoid `--all-features` for routine local runs because it expands the build matrix and can significantly increase `target/` disk usage; use it only when you specifically need full feature coverage. Project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite except for the stable-release sync workflow above.

Before finalizing a large change to `codex-rs`, run `just fix -p <project>` (in `codex-rs` directory) to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Do not re-run tests after running `fix` or `fmt` unless either command changed code covered by those tests. For a stable-release sync, use the staged order above.

## The `codex-core` crate

Over time, the `codex-core` crate (defined in `codex-rs/core/`) has become bloated because it is the largest crate, so it is often easier to add something new to `codex-core` rather than refactor out the library code you need so your new code neither takes a dependency on, nor contributes to the size of, `codex-core`.

To that end: **resist adding code to codex-core**!

Particularly when introducing a new concept/feature/API, before adding to `codex-core`, consider whether:

- There is an existing crate other than `codex-core` that is an appropriate place for your new code to live.
- It is time to introduce a new crate to the Cargo workspace for your new functionality. Refactor existing code as necessary to make this happen.

Likewise, when reviewing code, do not hesitate to push back on PRs that would unnecessarily add code to `codex-core`.

## Code Review Rules

### Crate API surface

Keep crate API surfaces as small as possible. Avoid proliferating test-only helpers.

### Model visible context

Codex maintains a context (history of messages) that is sent to the model in inference requests.

1. No history rewrite - the context must be built up incrementally.
2. Avoid frequent changes to context that cause cache misses.
3. No unbounded items - everything injected in the model context must have a bounded size and a hard cap.
4. No items larger than 10K tokens.
5. Highlight new individual items that can cross >1k tokens as P0. These need an additional manual review.
6. All injected fragments must be defined as structs in `core/context` and implement ContextualUserFragment trait

### Breaking changes

Search for breaking changes in external integration surfaces:

- app-server APIs
- raw response item events (`rawResponseItem/*`), even while experimental
- CLI parameters
- configuration loading
- resuming sessions from existing rollouts

### Test authoring guidance

For agent changes prefer integration tests over unit tests. Integration tests are under `core/suite` and use `test_codex` to set up a test instance of codex.

Features that change the agent logic MUST add an integration test:

- Provide a list of major logic changes and user-facing behaviors that need to be tested.

If unit tests are needed, put them in a dedicated test file (\*\_tests.rs).
Avoid test-only functions in the main implementation.

Check whether there are existing helpers to make tests more streamlined and readable.

### Change size guidance (800 lines)

Unless the change is mechanical the total number of changed lines should not exceed 800 lines.
For complex logic changes the size should be under 500 lines.

If the change is larger, explore whether it can be split into reviewable stages and identify the smallest coherent stage to land first.
Base the staging suggestion on the actual diff, dependencies, and affected call sites.

## TUI style conventions

See `codex-rs/tui/styles.md`.

## TUI code conventions

- Use concise styling helpers from ratatui’s Stylize trait.
  - Basic spans: use "text".into()
  - Styled spans: use "text".red(), "text".green(), "text".magenta(), "text".dim(), etc.
  - Prefer these over constructing styles with `Span::styled` and `Style` directly.
  - Example: patch summary file lines
    - Desired: vec!["  └ ".into(), "M".red(), " ".dim(), "tui/src/app.rs".dim()]

### TUI Styling (ratatui)

- Prefer Stylize helpers: use "text".dim(), .bold(), .cyan(), .italic(), .underlined() instead of manual Style where possible.
- Prefer simple conversions: use "text".into() for spans and vec![…].into() for lines; when inference is ambiguous (e.g., Paragraph::new/Cell::from), use Line::from(spans) or Span::from(text).
- Computed styles: if the Style is computed at runtime, using `Span::styled` is OK (`Span::from(text).set_style(style)` is also acceptable).
- Avoid hardcoded white: do not use `.white()`; prefer the default foreground (no color).
- Chaining: combine helpers by chaining for readability (e.g., url.cyan().underlined()).
- Single items: prefer "text".into(); use Line::from(text) or Span::from(text) only when the target type isn’t obvious from context, or when using .into() would require extra type annotations.
- Building lines: use vec![…].into() to construct a Line when the target type is obvious and no extra type annotations are needed; otherwise use Line::from(vec![…]).
- Avoid churn: don’t refactor between equivalent forms (Span::styled ↔ set_style, Line::from ↔ .into()) without a clear readability or functional gain; follow file‑local conventions and do not introduce type annotations solely to satisfy .into().
- Compactness: prefer the form that stays on one line after rustfmt; if only one of Line::from(vec![…]) or vec![…].into() avoids wrapping, choose that. If both wrap, pick the one with fewer wrapped lines.

### Text wrapping

- Always use textwrap::wrap to wrap plain strings.
- If you have a ratatui Line and you want to wrap it, use the helpers in tui/src/wrapping.rs, e.g. word_wrap_lines / word_wrap_line.
- If you need to indent wrapped lines, use the initial_indent / subsequent_indent options from RtOptions if you can, rather than writing custom logic.
- If you have a list of lines and you need to prefix them all with some prefix (optionally different on the first vs subsequent lines), use the `prefix_lines` helper from line_utils.

## Tests

### Test module organization

- When adding a new test module, define its contents in a separate sibling file rather than inline in the implementation file.
- Use an explicit `#[path = "..._tests.rs"]` attribute so the test filename is descriptive and easy to locate:

  ```rust
  #[cfg(test)]
  #[path = "parser_tests.rs"]
  mod tests;
  ```

- This applies only when introducing a new test module. Do not move or rewrite existing inline `#[cfg(test)] mod tests { ... }` modules solely to follow this convention.

### Snapshot tests

This repo uses snapshot tests (via `insta`), especially in `codex-rs/tui`, to validate rendered output.

**Requirement:** any change that affects user-visible UI (including adding new UI) must include
corresponding `insta` snapshot coverage (add a new snapshot test if one doesn't exist yet, or
update the existing snapshot). Review and accept snapshot updates as part of the PR so UI impact
is easy to review and future diffs stay visual.

When UI or text output changes intentionally, update the snapshots as follows:

- Run tests to generate any updated snapshots:
  - `just test -p codex-tui`
- Check what’s pending:
  - `cargo insta pending-snapshots -p codex-tui`
- Review changes by reading the generated `*.snap.new` files directly in the repo, or preview a specific file:
  - `cargo insta show -p codex-tui path/to/file.snap.new`
- Only if you intend to accept all new snapshots in this crate, run:
  - `cargo insta accept -p codex-tui`

If you don’t have the tool:

- `cargo install --locked cargo-insta`

### Benchmarks

cargo benchmarks can be run with `just bench`, use the divan crate to write new ones.

Use `just bench-smoke` to dry-run the benchmark for a single iteration to ensure it works.

### Test assertions

- Tests should use pretty_assertions::assert_eq for clearer diffs. Import this at the top of the test module if it isn't already.
- Prefer deep equals comparisons whenever possible. Perform `assert_eq!()` on entire objects, rather than individual fields.
- Avoid mutating process environment in tests; prefer passing environment-derived flags or dependencies from above.

### Spawning workspace binaries in tests (Cargo vs Bazel)

- Prefer `codex_utils_cargo_bin::cargo_bin("...")` over `assert_cmd::Command::cargo_bin(...)` or `escargot` when tests need to spawn first-party binaries.
  - Under Bazel, binaries and resources may live under runfiles; use `codex_utils_cargo_bin::cargo_bin` to resolve absolute paths that remain stable after `chdir`.
- When locating fixture files or test resources under Bazel, avoid `env!("CARGO_MANIFEST_DIR")`. Prefer `codex_utils_cargo_bin::find_resource!` so paths resolve correctly under both Cargo and Bazel runfiles.

### Integration tests

#### codex_core integration testing

- Prefer the utilities in `core_test_support::responses` when writing end-to-end Codex tests.
- Use `TestCodexBuilder::build_with_auto_env()` by default to ensure that new tests work with
  foreign app/exec OSes. See $remote-tests for details.
- All `mount_sse*` helpers return a `ResponseMock`; hold onto it so you can assert against outbound `/responses` POST bodies.
- Use `ResponseMock::single_request()` when a test should only issue one POST, or `ResponseMock::requests()` to inspect every captured `ResponsesRequest`.
- `ResponsesRequest` exposes helpers (`body_json`, `input`, `function_call_output`, `custom_tool_call_output`, `call_output`, `header`, `path`, `query_param`) so assertions can target structured payloads instead of manual JSON digging.
- Build SSE payloads with the provided `ev_*` constructors and the `sse(...)`.
- Prefer `wait_for_event` over `wait_for_event_with_timeout`.
- Prefer `mount_sse_once` over `mount_sse_once_match` or `mount_sse_sequence`

- Typical pattern:

  ```rust
  let mock = responses::mount_sse_once(&server, responses::sse(vec![
      responses::ev_response_created("resp-1"),
      responses::ev_function_call(call_id, "shell", &serde_json::to_string(&args)?),
      responses::ev_completed("resp-1"),
  ])).await;

  codex.submit(Op::UserTurn { ... }).await?;

  // Assert request body if needed.
  let request = mock.single_request();
  // assert using request.function_call_output(call_id) or request.json_body() or other helpers.
  ```

#### app-server integration testing

- Tests should exercise app-server's public JSON-RPC API.
- Use similar server mocking as for core integration tests.
- Use `TestAppServer::builder().build()` and `TestAppServer::send_thread_start_request_with_auto_env()`
  by default to ensure that new tests work with foreign app/exec OSes. See `$remote-tests` for
  details.

## App-server API Development Best Practices

These guidelines apply to app-server protocol work in `codex-rs`, especially:

- `app-server-protocol/src/protocol/common.rs`
- `app-server-protocol/src/protocol/v2.rs`

### Core Rules

- All active API development should happen in app-server v2. Do not add new API surface area to v1.
- Follow payload naming consistently:
  `*Params` for request payloads, `*Response` for responses, and `*Notification` for notifications.
- Expose RPC methods as `<resource>/<method>` and keep `<resource>` singular (for example, `thread/read`, `app/list`).
- Always expose fields as camelCase on the wire with `#[serde(rename_all = "camelCase")]` unless a tagged union or explicit compatibility requirement needs a targeted rename.
- Always expose string enum values as camelCase on the wire with matching serde and TS `rename_all = "camelCase"` annotations unless an explicit compatibility requirement needs targeted renames.
- Exception: config RPC payloads are expected to use snake_case to mirror config.toml keys (see the config read/write/list APIs in `app-server-protocol/src/protocol/v2.rs`).
- Always set `#[ts(export_to = "v2/")]` on v2 request/response/notification types so generated TypeScript lands in the correct namespace.
- Never use `#[serde(skip_serializing_if = "Option::is_none")]` for v2 API payload fields.
  Exception: client->server requests that intentionally have no params may use:
  `params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>`.
- Keep Rust and TS wire renames aligned. If a field or variant uses `#[serde(rename = "...")]`, add matching `#[ts(rename = "...")]`.
- For discriminated unions, use explicit tagging in both serializers:
  `#[serde(tag = "type", ...)]` and `#[ts(tag = "type", ...)]`.
- Prefer plain `String` IDs at the API boundary (do UUID parsing/conversion internally if needed).
- Timestamps should be integer Unix seconds (`i64`) and named `*_at` (for example, `created_at`, `updated_at`, `resets_at`).
- For experimental API surface area:
  use `#[experimental("method/or/field")]`, derive `ExperimentalApi` when field-level gating is needed, and use `inspect_params: true` in `common.rs` when only some fields of a method are experimental.

### Client->server request payloads (`*Params`)

- Every optional field must be annotated with `#[ts(optional = nullable)]`. Do not use `#[ts(optional = nullable)]` outside client->server request payloads (`*Params`).
- Optional collection fields (for example `Vec`, `HashMap`) must use `Option<...>` + `#[ts(optional = nullable)]`. Do not use `#[serde(default)]` to model optional collections, and do not use `skip_serializing_if` on v2 payload fields.
- When you want omission to mean `false` for boolean fields, use `#[serde(default, skip_serializing_if = "std::ops::Not::not")] pub field: bool` over `Option<bool>`.
- For new list methods, implement cursor pagination by default:
  request fields `pub cursor: Option<String>` and `pub limit: Option<u32>`,
  response fields `pub data: Vec<...>` and `pub next_cursor: Option<String>`.

### Development Workflow

- Regenerate schema fixtures when API shapes change:
  `just write-app-server-schema`
  (and `just write-app-server-schema --experimental` when experimental API fixtures are affected).
- Validate with `just test -p codex-app-server-protocol`.
- Avoid boilerplate tests that only assert experimental field markers for individual
  request fields in `common.rs`; rely on schema generation/tests and behavioral coverage instead.

## Python Development Best Practices

### Ignore Python 2 compatibility

This project uses Python 3+. You should not use the `__future__` module.

If you need to worry about feature compatibility between different 3.xx point releases, check the
closest `pyproject.toml`'s `requires-python` field to see what minimum runtime version is supported.

## Platform Support

Tests and features must support Linux, macOS and Windows unless feature is explicitly OS-specific.

Codex supports running connected app-server and exec-server on different operating systems. See the
`$remote-tests` skill for details about integration testing these configurations.

## Generated document index

- [harness-audit/mcp-review.md](research/harness-audit/mcp-review.md) — Records the idle MCP cancellation audit; later fixture correction and delivery results are tracked in the run ledger.
- [harness-audit/mcp-findings.json](research/harness-audit/mcp-findings.json) — Preserves the initial structured MCP recovery finding and test evidence.
- [harness-audit/mcp-red.txt](research/harness-audit/mcp-red.txt) — Preserves the initial coordinator MCP failure replay before the documented presentation-fixture correction.
- [harness-audit/residency-review.md](research/harness-audit/residency-review.md) — Records the proven eviction accounting defect and rejected queue-only mailbox claim.
- [harness-audit/residency-findings.json](research/harness-audit/residency-findings.json) — Stores the structured residency finding with its final causal test.
- [harness-audit/residency-red.txt](research/harness-audit/residency-red.txt) — Records the coordinator's failing capacity and cancellation replay.
- [harness-audit/retry-delivery/builder-report.md](research/harness-audit/retry-delivery/builder-report.md) — Records the retry repair and passing API verification.
- [harness-audit/retry-delivery/verifier-report.md](research/harness-audit/retry-delivery/verifier-report.md) — Records independent acceptance of the retry repair.
- [harness-audit/ordering-delivery/report.md](research/harness-audit/ordering-delivery/report.md) — Records the malformed-search finding and dismissed batching premise.
- [harness-audit/ordering-delivery/builder-report.md](research/harness-audit/ordering-delivery/builder-report.md) — Records the tool-search output repair and affected tests.
- [harness-audit/ordering-delivery/verifier-report.md](research/harness-audit/ordering-delivery/verifier-report.md) — Records independent acceptance of the tool-search repair.

- [harness-audit/storage-review.md](research/harness-audit/storage-review.md) — Records transcript persistence review and the archive crash recovery finding.
- [harness-audit/storage-findings.json](research/harness-audit/storage-findings.json) — Stores the structured archive crash finding and causal regression.
- [harness-audit/storage-red.patch](research/harness-audit/storage-red.patch) — Preserves the durable crash-state regression for paginated archive recovery.
- [harness-audit/storage-red.txt](research/harness-audit/storage-red.txt) — Records the coordinator's independent failing archive recovery replay.

- [harness-audit/retry-review.md](research/harness-audit/retry-review.md) — Records terminal-response parsing review and the quota-error classification defect.
- [harness-audit/retry-findings.json](research/harness-audit/retry-findings.json) — Stores the structured retry finding with causal proof and source trace.
- [harness-audit/retry-red.patch](research/harness-audit/retry-red.patch) — Preserves the regression that exposes terminal quota errors overwritten by transport errors.
- [harness-audit/retry-red.txt](research/harness-audit/retry-red.txt) — Preserves the investigator's executed failing retry regression log.

- [harness-audit/replay-review.md](research/harness-audit/replay-review.md) — Records the session replay and rollback source audit, clean result, and unproven history-scaling limit.

- [harness-audit/session-evidence.md](research/harness-audit/session-evidence.md) — Summarizes sanitized local-session observations, incident leads, and running-version limits.
- [harness-audit/session-evidence.json](research/harness-audit/session-evidence.json) — Preserves content-free session counts and source-manifest hashes from the coordinator replay.
- [harness-audit/scan_sessions.py](research/harness-audit/scan_sessions.py) — Reproduces the bounded read-only session and runtime-log evidence scan.

- [harness-audit/brief.md](research/harness-audit/brief.md) — Defines the six harness audit scopes, causal proof requirements, private-session handling, and agent handoff contract.
- [harness-audit/run-ledger.md](research/harness-audit/run-ledger.md) — Tracks parallel harness investigations, findings, validation, integration, and cleanup.

- [next-audit-targets.md](research/next-audit-targets.md) — Ranks further audit targets using current open GitHub reports, local patch deduplication, and proposed causal tests.

- [custom-patch-audit.md](research/custom-patch-audit.md) — Tracks the significant cache and wasted-turn patch audit, regression evidence, repairs, and verification limits.
- [findings.json](reviews/findings.json) — Stores the two proven custom-patch defects, complete regression patches, replay evidence, and repair status.
- [ledger.json](reviews/ledger.json) — Records the audited commit and scope for primary source paths reviewed for significant cache and turn defects.
- [prefix-review.md](research/custom-patch-audit/prefix-review.md) — Records source review of local prefix, compaction, and resume patches with no proven significant defect.
- [diagnostics-review.md](research/custom-patch-audit/diagnostics-review.md) — Records source review of diagnostic observers and transport continuation with no proven significant defect.
- [catalog-review.md](research/custom-patch-audit/catalog-review.md) — Proves unnecessary tool-catalog changes from disabled plugin skills with a failing regression.
- [catalog-fix.md](research/custom-patch-audit/catalog-fix.md) — Records the one-condition catalog repair and 176 passing skills-extension tests.
- [catalog-verification.md](research/custom-patch-audit/catalog-verification.md) — Records independent review of the disabled-plugin repair and its evidence limits.
- [async-review.md](research/custom-patch-audit/async-review.md) — Proves duplicate model-visible exec completion after a terminal write_stdin result and records rejected candidates.
- [async-fix.md](research/custom-patch-audit/async-fix.md) — Records the terminal-result disarming repair, focused green tests, and isolated-target suite failures.
- [async-verification.md](research/custom-patch-audit/async-verification.md) — Records independent review and passing causal tests for terminal completion delivery.
- [validation-diagnosis.md](research/custom-patch-audit/validation-diagnosis.md) — Separates load-sensitive core test failures from three reproducible fixture/policy mismatches.
- [validation-evidence.tar.gz](research/custom-patch-audit/validation-evidence.tar.gz) — Preserves raw audit reports, red replays, validation logs, baseline comparison, and build-cleanup records.
- [validation-manifest.json](research/custom-patch-audit/validation-manifest.json) — Lists SHA-256 hashes for every archived audit and validation evidence file.
- [installation-local7.json](research/custom-patch-audit/installation-local7.json) — Records the user-approved scoped installation of local.7, binary checksums, validation limits, installation time, and local.6 rollback release.
- [catalog-red-replay.log](research/custom-patch-audit/catalog-red-replay.log) — Preserves the coordinator's independent failing replay of the disabled-plugin regression.
- [async-red-replay.log](research/custom-patch-audit/async-red-replay.log) — Preserves the coordinator's independent failing replay of all three terminal-completion regression cases.
- [goal-async-workflow-analysis.md](research/goal-async-workflow-analysis.md) — Assesses Goal continuation and asynchronous result delivery, explains the empty-turn breaker, and ranks compatible repair layers with their evidence and verification limits.
- [wait-agent-disablement-evidence.md](research/wait-agent-disablement-evidence.md) — Records the polling measurements, runtime flag behavior, source paths, live Luna probes, workflow effects, and public issue/PR evidence for disabling MultiAgentV2 `wait_agent`.
- [wait-agent-evidence/README.md](research/wait-agent-evidence/README.md) — Defines the immutable inputs, reproduction commands, schemas, assertions, and files for the self-verifying `wait_agent` evidence bundle.
- [wait-agent-evidence/polling-records.jsonl](research/wait-agent-evidence/polling-records.jsonl) — Stores every sanitized polling call with exact rollout, line, turn, call, outcome, timestamp, and token attribution.
- [wait-agent-evidence/polling-summary.json](research/wait-agent-evidence/polling-summary.json) — Stores machine-checkable per-mechanism counts, outcomes, sessions, turns, and token totals.
- [wait-agent-evidence/exec-completion-notifications.jsonl](research/wait-agent-evidence/exec-completion-notifications.jsonl) — Stores all 76 sanitized completion-envelope records proving the incident runtime emitted exec completion events.
- [wait-agent-evidence/source-manifest.json](research/wait-agent-evidence/source-manifest.json) — Binds all rollout inputs and live probes to absolute paths, SHA-256 hashes, sizes, timestamps, metadata, and tool counts.
- [wait-agent-evidence/original-audit-command-log.jsonl](research/wait-agent-evidence/original-audit-command-log.jsonl) — Preserves the original audit commands and terminal outputs from the investigation rollout.
- [wait-agent-evidence/source-snapshot.json](research/wait-agent-evidence/source-snapshot.json) — Preserves source excerpts, file and blob hashes, commit metadata, and patch-ancestry results.
- [wait-agent-evidence/config-snapshot.json](research/wait-agent-evidence/config-snapshot.json) — Preserves the relevant runtime configuration subsection and binds it to the full settings file by hash.
- [wait-agent-evidence/runtime-snapshot.json](research/wait-agent-evidence/runtime-snapshot.json) — Preserves installed-binary hashes, versions, active tool inventory, and live-probe outcomes.
- [wait-agent-evidence/github-snapshot.json](research/wait-agent-evidence/github-snapshot.json) — Preserves the public GitHub issue and PR material used by the investigation.
- [wait-agent-evidence/SHA256SUMS](research/wait-agent-evidence/SHA256SUMS) — Provides integrity hashes for every generated evidence data file.
- [goal-continuation-usage-incident.md](research/goal-continuation-usage-incident.md) — Confirms the 19 September Goal usage incident and maps public reports, fixes, source paths, and root-cause tests.
- [cache-diagnostics-guide.md](research/cache-diagnostics-guide.md) — Explains how to collect and interpret private cache diagnostics.
- [cache-repair-run.md](research/cache-repair-run.md) — Records the cache repair work, decisions, validation, and delivery milestones.
- [cache-repair-recovery.md](research/cache-repair-recovery.md) — Preserves historical recovery checkpoints from the interrupted repair run.
- [cache-repair-issue-disposition.md](research/cache-repair-issue-disposition.md) — Tracks investigated issues, completed repairs, and remaining obligations.
- [prompt-cache-prefix-stability.md](research/prompt-cache-prefix-stability.md) — Records the original prefix-stability investigation and issue research.
- [prefix-cache-code-audit.md](research/prefix-cache-code-audit.md) — Maps the code that constructs and changes cache-relevant request prefixes.
- [compaction-resume-cache-validation.md](research/compaction-resume-cache-validation.md) — Reports live cache reuse across compaction and cold session resumes.
- [scan.json](research/cache-baselines/2026-09-14-pre-repair/scan.json) — Stores the pre-repair diagnostic baseline measurements.
- [provenance.json](research/cache-baselines/2026-09-14-pre-repair/provenance.json) — Records the origin and scope of the pre-repair baseline.
- [compaction-resume-probe.json](research/cache-repair-reports/compaction-resume-probe.json) — Stores sanitized request comparisons and usage from the compaction/resume tests.
- [runtime-cache-scan-20260914.json](research/cache-repair-reports/runtime-cache-scan-20260914.json) — Classifies recorded cache drops after installation of the local binary.
- [installation-local2.json](research/cache-repair-reports/installation-local2.json) — Records the installed binary version, checksums, installation time, and rollback location.
- [final-delivery-audit.md](research/cache-repair-reports/final-delivery-audit.md) — Reviews the installed milestone and states its limits against the original objective.
- [issues-30425-35925-diagnosis.md](research/cache-repair-reports/issues-30425-35925-diagnosis.md) — Investigates the causes reported in issues 30425 and 35925.
- [build-readiness.md](research/cache-repair-reports/build-readiness.md) — Records the initial build environment and prerequisites.
- [compact-parity-builder.md](research/cache-repair-reports/compact-parity-builder.md) — Describes the repair that keeps compaction tools aligned with the active turn.
- [compact-parity-tests.md](research/cache-repair-reports/compact-parity-tests.md) — Describes regression tests for matching ordinary and compaction tool catalogs.
- [compaction-tools-inventory.md](research/cache-repair-reports/compaction-tools-inventory.md) — Maps compaction request construction and tool-related repair candidates.
- [compaction-verifier.md](research/cache-repair-reports/compaction-verifier.md) — Records independent verification of the compaction tool-catalog repair.
- [diagnostics-design-initial.md](research/cache-repair-reports/diagnostics-design-initial.md) — Preserves the initial diagnostic design proposal and its tradeoffs.
- [diagnostics-interface.md](research/cache-repair-reports/diagnostics-interface.md) — Describes the proposed interfaces between diagnostic records and request handling.
- [records-verifier.md](research/cache-repair-reports/records-verifier.md) — Records the historical review of bounded diagnostic records and outstanding checks.
- [analysis-verifier-followup.md](research/cache-repair-reports/analysis-verifier-followup.md) — Records verification of the offline cache-diagnostics analyzer.
- [runtime-diagnostics-verifier-followup.md](research/cache-repair-reports/runtime-diagnostics-verifier-followup.md) — Reviews diagnostic coverage of warm-ups, compaction, retries, and fallback requests.
- [runtime-diagnostics-memory-assessment.md](research/cache-repair-reports/runtime-diagnostics-memory-assessment.md) — Reviews diagnostic coverage of background memory requests.
- [prefix-subagents-inventory.md](research/cache-repair-reports/prefix-subagents-inventory.md) — Maps prefix construction for subagents and identifies stability candidates.
- [fresh-prefix-verifier.md](research/cache-repair-reports/fresh-prefix-verifier.md) — Records verification of stable initial context ordering for fresh subagents.
- [parent-completion-tests.md](research/cache-repair-reports/parent-completion-tests.md) — Describes tests for parent notification when subagents complete.
- [grandchild-context-flake-diagnosis.md](research/cache-repair-reports/grandchild-context-flake-diagnosis.md) — Explains a synchronization race in the grandchild context test.
- [locator-verifier-followup.md](research/cache-repair-reports/locator-verifier-followup.md) — Reviews stable plugin skill locations and their regression coverage.
- [tool-stability-final-audit.md](research/cache-repair-reports/tool-stability-final-audit.md) — Records the final review and test results for tool ordering and presentation stability.
- [pending-projection-verifier.md](research/cache-repair-reports/pending-projection-verifier.md) — Records the initial review of pending-input context accounting.
- [pending-projection-fullsuite-review.md](research/cache-repair-reports/pending-projection-fullsuite-review.md) — Records follow-up verification of pending-input accounting and corrected fixtures.
- [usage-boundary-tests.md](research/cache-repair-reports/usage-boundary-tests.md) — Describes regression tests for usage accounting across history changes and resume.
- [usage-verifier.md](research/cache-repair-reports/usage-verifier.md) — Records independent verification of restored usage and active-context estimates.
- [README.md](docs/experiments/README.md) — Provides the entry point for local engineering experiments.
- [local-build-pipeline.md](docs/experiments/local-build-pipeline.md) — Records measured local build settings and compiler-cache experiments.
- [Goal/async local.5 installation record](/Users/viktor/.local/lib/codex-local/0.155.1+local.5-9d738e79eeb1-84c1f583e9dd/provenance.json) — Records the installed CLI and code-mode host, source commit, checksums, installation time, build log, and rollback directory.
- [Stable 0.156.1 local.6 installation record](/Users/viktor/.local/lib/codex-local/0.156.1+local.6-2a63855bea2a-320f2b1dd788/provenance.json) — Records the installed CLI and code-mode host, source commit, checksums, tests, installation time, and rollback release.

- [mcp/builder-report.md](research/harness-audit/mcp-delivery/builder-report.md) — Records the mcp audit builder report and its verification limits.

- [mcp/report.md](research/harness-audit/mcp-delivery/report.md) — Records the mcp audit report and its verification limits.

- [mcp/verifier-report.md](research/harness-audit/mcp-delivery/verifier-report.md) — Records the mcp audit verifier report and its verification limits.

- [residency/builder-report.md](research/harness-audit/residency-delivery/builder-report.md) — Records the residency audit builder report and its verification limits.

- [residency/report.md](research/harness-audit/residency-delivery/report.md) — Records the residency audit report and its verification limits.

- [residency/verifier-report.md](research/harness-audit/residency-delivery/verifier-report.md) — Records the residency audit verifier report and its verification limits.

- [storage/builder-report.md](research/harness-audit/storage-delivery/builder-report.md) — Records the storage audit builder report and its verification limits.

- [storage/report.md](research/harness-audit/storage-delivery/report.md) — Records the storage audit report and its verification limits.

- [storage/tester-report.md](research/harness-audit/storage-delivery/tester-report.md) — Records the storage audit tester report and its verification limits.

- [storage/verifier-initial-report.md](research/harness-audit/storage-delivery/verifier-initial-report.md) — Records the storage audit verifier initial report and its verification limits.

- [storage/verifier-report.md](research/harness-audit/storage-delivery/verifier-report.md) — Records the storage audit verifier report and its verification limits.

- [Harness audit summary](research/harness-audit/summary.md) — Summarizes five proven repairs, two clean audit slices, evidence, and delivery limits.

- [worktree-evidence.tar.gz](research/harness-audit/worktree-evidence.tar.gz) — Preserves audit worktree reports, raw test logs, patches, and integration evidence before cleanup.

- [worktree-evidence-manifest.json](research/harness-audit/worktree-evidence-manifest.json) — Lists verified SHA-256 hashes for every file in the preserved worktree evidence archive.

- [installation-local8.json](research/harness-audit/installation-local8.json) — Records installed local.8 binaries, checksums, validation limits, installation time, and local.7 rollback.

- [validation-evidence.tar.gz](research/harness-audit/validation-evidence.tar.gz) — Preserves combined build, lint, test, smoke-check and cleanup evidence for local.8.

- [validation-manifest.json](research/harness-audit/validation-manifest.json) — Lists verified SHA-256 hashes for the combined validation evidence archive.
