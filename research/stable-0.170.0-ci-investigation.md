# Stable 0.170.0 CI and release investigation

The release workflow succeeded. Local test failures on the unchanged tag do not prove upstream CI failed or that maintainers knew of these failures. The public workflow graph does establish that the release pipeline has no explicit dependency on the complete Rust runtime test suite.

## Verified evidence

- Stable release: https://github.com/openai/codex/releases/tag/rust-v0.170.0, published 2026-11-18 03:40:48 UTC. Annotated tag object 72f97c309fb0f78b6f35cce455adf416e6a51571 peels to commit 29eb79e9866b3262a777fdf8195c1b9e63684c91. The earlier baseline report stores the tag object as upstream_commit; it identifies the same source, not a different revision.
- Release run https://github.com/openai/codex/actions/runs/44223519321 completed successfully. Public check-runs on the release commit are build, packaging, signing, publication and artifact-verification jobs, with successful or intentionally skipped conclusions.
- `.github/workflows/blocking-ci.yml` names the intended merge-blocking checks. Bazel is explicitly advisory while a BuildBuddy key is revoked and is omitted from the required dependency list. The repository comment describes intended ruleset requirements; actual server-side branch protections were not established by this audit.
- `.github/workflows/rust-ci.yml` required aggregate covers format/general checks, dependency checks and lint-package checks; the Bazel-backed argument lint is advisory. This is not the complete Cargo runtime suite.
- `.github/workflows/postmerge-ci.yml` runs the broader rust-ci-full and V8 canary after pushes to main, explicitly outside merge blocking.
- `.github/workflows/rust-release.yml` is triggered by release tags. Its publication job requires tag validation, builds, final macOS artifacts, Windows builds, lint release assets, npm staging and the applicable macOS candidate result. It does not depend on postmerge-ci, rust-ci-full, or a check-suite-success lookup. Successful publication therefore does not itself certify the full runtime suite passed.
- The public actions query for the exact release commit returned the release workflow only. Its immediate parent query returned no workflow runs/checks. Empty results do not establish that no relevant testing happened on other revisions or PR merge candidates, or internally. Do not infer a knowingly ignored failed run.

## Interpretation

A CI badge means the configured checks passed for their tested environment and revision. Releasing software does not logically imply every test in every environment passed. A strong release policy should explicitly require the appropriate complete test result for the exact candidate (or an established equivalent revision). The published workflow graph has a real enforcement gap in that respect, but this audit has not shown maintainers released in the face of these exact seventeen failures.

Our local failures still require individual causal investigation: sandbox and process restrictions, inherited configuration and test assumptions can differ from CI; some failures may also be genuine upstream defects. Reproduction on upstream isolates them from the private patches but does not identify those causes. A separate investigator is examining the preserved logs and source.

## Evidence preservation

The adjacent `stable-0.170.0-ci-evidence/` directory contains public GitHub API responses for the release, workflow runs, check-runs, legacy status and immediate-parent queries, plus a SHA-256 manifest. No secrets or private session contents were collected. No product source or installed binaries were changed.
