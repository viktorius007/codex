# `wait_agent` evidence bundle

## Reproduction

```sh
cd /Users/viktor/Projects/github/codex
CODEX_DATA_DIR=/Users/viktor/.codex python3 research/wait-agent-evidence/extract_evidence.py
python3 research/wait-agent-evidence/capture_github_evidence.py
cd research/wait-agent-evidence
shasum -a 256 -c SHA256SUMS
python3 verify_evidence.py
python3 verify_evidence.py --check-external
```

## Inputs

| Input | Binding |
|---|---|
| Seven Brisbane-day user rollouts | Absolute path, SHA-256, byte size, nanosecond mtime, first/last record timestamp, session metadata in `source-manifest.json` |
| Three polling-bearing rollouts | Listed separately in `source-manifest.json`; every derived polling record carries source path, source SHA-256, and source line |
| Original investigation rollout | Fixed timestamp interval and exact call/output pairs in `original-audit-command-log.jsonl` |
| Two live Luna probe rollouts | Absolute path, SHA-256, byte size, timestamps, metadata, tool counts in `source-manifest.json` |
| `~/.codex/config.toml` | Full-file SHA-256 and relevant parsed subsection in `config-snapshot.json` |
| Repository source | HEAD, branch, file SHA-256, Git blob ID, selected line-numbered excerpts, commit metadata, and ancestry results in `source-snapshot.json` |
| Public GitHub evidence | Full issue/PR bodies, comments, selected timeline links, reviews, commits, state, and timestamps in `github-snapshot.json` |

## Outputs

| File | Records/content | Private-content rule |
|---|---:|---|
| `polling-records.jsonl` | 386 polling calls | No prompts; no raw tool output; identifiers, timestamps, classifications, and usage only |
| `polling-summary.json` | Five aggregate groups | Derived exclusively from `polling-records.jsonl` |
| `exec-completion-notifications.jsonl` | 76 completion envelopes | No command or command output; call/process IDs, exit code, timestamp, source line only |
| `source-manifest.json` | Seven user rollouts; three polling inputs; two live probes | Metadata and hashes only |
| `original-audit-command-log.jsonl` | 56 original audit command/output pairs | Audit commands and their terminal outputs; no affected-session prompts copied |
| `source-snapshot.json` | Source excerpts and commit provenance | Repository source only |
| `config-snapshot.json` | Relevant config subsection | Full config is not copied; unrelated settings are excluded |
| `runtime-snapshot.json` | Installed binary and resumed-harness observations | No prompt or user data |
| `github-snapshot.json` | Public issues and PRs | Public GitHub material only |
| `SHA256SUMS` | Evidence and reproduction-file integrity list | Excludes only `SHA256SUMS` itself |

## Polling record schema

| Field | Definition |
|---|---|
| `kind` | `empty_write_stdin`, `code_wait`, or `wait_agent` |
| `outcome` | `script_running`, `script_completed`, `timed_out`, `interrupted`, `activity`, or `other` |
| `session_id`, `turn_id`, `call_id` | Persisted rollout identifiers |
| `call_timestamp`, `output_timestamp`, `usage_timestamp` | Persisted UTC timestamps |
| `source_path`, `source_sha256`, `source_line`, `output_line`, `usage_line` | Exact source locator |
| `input_tokens` | `token_count.info.last_token_usage.input_tokens` |
| `cached_input_tokens` | `token_count.info.last_token_usage.cached_input_tokens` |
| `output_tokens` | `token_count.info.last_token_usage.output_tokens` |
| `total_tokens` | `token_count.info.last_token_usage.total_tokens` |
| `fresh_tokens` | `input_tokens - cached_input_tokens + output_tokens` |
| `usage_attribution_rule` | First `token_count.info.last_token_usage` after the matching tool output |

## Hard assertions in `extract_evidence.py`

| Group | Records | Total tokens | Fresh tokens | Turns |
|---|---:|---:|---:|---:|
| Empty `write_stdin` | 127 | 16,368,566 | 378,550 | 36 |
| Code-mode `wait` | 43 | 3,803,025 | 186,001 | 8 |
| All `wait_agent` | 216 | 27,309,194 | 283,274 | 51 |
| Timed-out `wait_agent` | 148 | 18,471,910 | 217,702 | 38 |
| Conservative waste union | 318 | 38,643,501 | 782,253 | 74 |
| `<exec-command-completed>` envelopes | 76 | Not applicable | Not applicable | Not applicable |

## Classification rules

| Classification | Rule |
|---|---|
| Empty `write_stdin` | Custom `exec` input contains `tools.write_stdin` and an explicitly empty `chars` string |
| Code-mode `wait` | Function call name is exactly `wait` |
| `wait_agent` | Function call name is exactly `wait_agent` |
| Timed-out `wait_agent` | Matching function output contains `timed_out: true` |
| Conservative waste | All empty `write_stdin` + all code-mode `wait` + timed-out `wait_agent` |
| Parent-turn count | Distinct `(session_id, turn_id)` pairs in the conservative-waste set |
| Completion envelope | Message content contains `<exec-command-completed ...>` within the UTC evidence window |

## Failure behavior

| Condition | Result |
|---|---|
| Missing input rollout | Generator exits nonzero |
| Invalid JSONL line | Generator exits nonzero with path and line |
| Unpaired polling call/output/usage | Generator exits nonzero |
| Any published count or token total changes | Generator exits nonzero |
| Completion-envelope count differs from 76 | Generator exits nonzero |
| Evidence file changes after generation | `shasum -a 256 -c SHA256SUMS` fails |

## Offline verification

| Command | Checks |
|---|---|
| `python3 verify_evidence.py` | Integrity hashes, record-derived aggregates, 74-turn union, 76 completion records, 56 original audit commands, 23 GitHub issues, 3 GitHub PRs, seven user rollouts, and two live probes |
| `python3 verify_evidence.py --check-external` | All offline checks plus existence and SHA-256 identity of the original historical and live-probe rollout files |

## Non-reproducible-by-design observation

| Observation | Durable evidence |
|---|---|
| Root collaboration tool catalogue after resumed harness contained `spawn_agent`, `send_message`, `followup_task`, `interrupt_agent`, and `list_agents`, but not `wait_agent` | `runtime-snapshot.json`; quick live-probe final result and hashed rollout in `source-manifest.json`; source registration gate in `source-snapshot.json` |

## Test status

| Test class | Status |
|---|---|
| Live low-effort Luna config/tool-presence probe | Executed |
| Live high-effort Luna completion-path probe | Executed |
| One `list_agents` status snapshot | Executed |
| One mid-flight `followup_task` | Executed and acknowledged |
| Intermediate `send_message` probe | Not executed by the Luna agent; no delivery conclusion |
| Idle-parent Rust integration test | Source and local-patch provenance captured; not rerun during concurrent-writer investigation |
| Tool-registration Rust tests | Source captured; not rerun during concurrent-writer investigation |
