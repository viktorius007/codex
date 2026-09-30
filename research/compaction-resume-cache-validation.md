# Compaction and cold-resume cache validation

Assessed 2026-09-14. Source `b2da63e68447be521bc9b68471d5faa861c09d62`; live executable `0.154.0+local.2`, installed from `4b0a1a1194d1393680c9dca1d75cb9b8ecb17de7`. `git diff --quiet 4b0a1a1194 b2da63e684 -- codex-rs` exited zero: the inspected Rust source matches the installed source.

## Decision

Modern cold resume preserves the previous visible input prefix and can reuse the provider cache, including after compaction. This is now supported by live provider usage and independently checked request captures, rather than source inspection alone. No resume cache repair was indicated by the controlled runs.

Compaction preserves the tools and base instructions, but it also moves other contextual instructions behind retained conversation items. This forfeits some stable-context prefix reuse beyond the necessary replacement of conversation history. The measured drop is consistent with that changed layout; it is not an unexplained cache bust. Improving the contextual-prefix placement is an optimization candidate, not a proved behavior-preserving correction. No production patch was made.

## Live experiment

The installed app-server ran a synthetic task with fixed `gpt-6-astra`, medium reasoning, explicit base/developer reference text, a fixed working directory, disabled external MCP servers/plugins/hooks, and no tool calls. Six ordinary requests surrounded one explicit remote-v2 compaction. App-server was fully stopped and restarted before both resumes. The controlled rerun supplied the same base and developer overrides on every resume.

| Request | Input tokens | Cached input tokens | Transport |
|---|---:|---:|---|
| Initial | 23,859 | 0 | Full |
| Warm before resume | 29,218 | 23,680 | WebSocket delta |
| Cold resume before compaction | 34,510 | 29,056 | Full |
| First after compaction | 24,518 | 8,576 | Full |
| Warm after compaction | 24,532 | 24,320 | WebSocket delta |
| Cold resume after compaction | 29,824 | 24,320 | Full |

The cold-resume request starts were 5.798 seconds and 3.393 seconds after their respective preceding inference starts. These exercise immediate resume within cache lifetime, not the provider's expiry edge. The local replay mechanism has no cache-TTL branch; a provider hit cannot be guaranteed merely because elapsed time is below a nominal TTL.

All six ordinary turns completed successfully, with no tool calls. The server reported zero cache-write tokens even on cold input, so this report uses returned cached-input accounting and does not infer cache-write billing from those zeros. The compaction trace itself lacks a usage payload in this experiment; the table measures ordinary requests on both sides.

## Exact request evidence

The model uses Responses Lite: tools and base instructions are the first two `input` items rather than top-level `tools` and `instructions` fields. Their complete serialized item hashes, including IDs, were identical across all six requests. All nine present non-null prompt configuration fields also stayed equal: `include`, `model`, `parallel_tool_calls`, `prompt_cache_key`, `reasoning`, `store`, `stream`, `text`, and `tool_choice`. Absent top-level fields are not counted as evidence.

Before compaction the controlled developer reference occurs at input position 2. After compaction it occurs at position 6, behind three retained user messages and an opaque compaction item. Its 18,861-byte reference section remains identical, but it is in a rebuilt developer message. The complete enclosing message changes from 42,713 to 43,097 text bytes because other context in that message changes. The first 18,914 text bytes of those enclosing messages are equal. Stable material therefore exists beyond the preserved tools/base prefix, but it has moved behind a changed preceding sequence.

The pre/post compaction inputs agree through positions 0 and 1 and differ at position 2. The first post-compaction request reports 8,576 cached tokens. This supports preservation of an early cached prefix; it does not independently map each cached token to one request field or quantify how many tokens a different layout would recover.

For ordinary cold resume, all 11 preceding logical input/output items remain the resumed request's prefix after removing only `internal_chat_message_metadata_passthrough`. For post-compaction cold resume, the same holds for all 14 preceding input/output items. IDs and visible contents are preserved. Full-object differences occur only in assistant-message internal metadata. The provider still returns substantial cached tokens across both transitions.

The immediate ordinary request after the first post-compaction request is also an exact structured append: all 12 preceding input/output items remain equal without normalization. This closes the live same-session append evidence gap left by the original fixture review.

Large context updates are appended around some turns and resumes. They increase new input without modifying the earlier prefix, explaining why the resumed input count can increase while the existing cached-token count is preserved. Cached/input percentage alone would misclassify this as reduced prefix preservation.

## Why the compaction layout exists

Source paths below are relative to `codex-rs/`.

| Path | Replacement and subsequent layout |
|---|---|
| Local pre-turn/manual | Retained users + summary; clears the reference baseline; next ordinary turn appends canonical context and new input (`core/src/compact.rs:370-405,685-775`; `core/src/session/mod.rs:4286-4353`). |
| Remote legacy pre-turn/manual | Filters stale developer/wrapper content from returned history, installs retained output, then next ordinary turn appends canonical context (`core/src/compact_remote.rs:317-403`). |
| Remote v2 pre-turn/manual | Retains/truncates eligible messages and appends opaque compaction item; next ordinary turn appends canonical context (`core/src/compact_remote_v2.rs:315-358,489-590`). This is the live-tested path. |
| Mid-turn, all paths | Inserts canonical context before the last retained real user, or before summary/compaction fallback (`core/src/compact.rs:627-683`). With only one retained user, context can remain at the beginning; with older retained users, it moves behind them. |

The source states that mid-turn summary/compaction output must be last because of the model's training (`core/src/compact.rs:68-75`). Commit `bb0ac5be70fcef5f418abf77955e96e39add2f30`, “Fix compaction context reinjection and model baselines (#12252),” explains a real earlier bug: pre-turn compaction included incoming context updates without their new user message, and incorrect baselines caused duplicate or suppressed context. The current ordering and baseline resets address those correctness concerns.

Neither that history nor the inspected tests establishes that canonical context must specifically follow older retained user messages. Prepending an unchanged stable context block could preserve more prefix while leaving the summary last, but moving current instructions before older retained messages changes chronological scoping. New dynamic context also prevents simply treating the whole rebuilt developer bundle as immutable. A safe optimization would have to preserve the actual immutable prefix separately and maintain the existing current-context/baseline semantics. This investigation does not claim a one-line reorder is safe.

Public [OpenAI compaction guidance](https://developers.openai.com/api/docs/guides/compaction) describes canonical returned compacted windows and advises using standalone compact output as returned. It does not specify this private Codex client's reinsertion position. [Prompt-caching guidance](https://developers.openai.com/api/docs/guides/prompt-caching) explains why compaction can reduce reuse from the first changed token. These documents do not establish behavioral equivalence of an alternative client layout.

## Resume scope and regression evidence

Cold resume restores the thread/session identity (`core/src/session/session.rs:761-813`), preserves the ordinary cache key (`core/src/client.rs:516-528`), replays saved items in order, and starts a fresh transport connection. The full request after restart is expected; a new WebSocket connection does not invalidate the logical cached prefix.

Existing tests assert exact structured input-prefix equality across modern compacted-history resume (`core/tests/suite/compact_resume_fork.rs:245-265,428-444`; `core/tests/suite/compact.rs:5747-5757`). Existing tests were inspected, not rerun. The live experiment additionally compares actual present request settings and the complete ordered tool item, which those individual tests do not jointly establish.

The result is scoped to modern saved history and unchanged effective state. Legacy compact records without `replacement_history` can require reconstruction. Changed effective tools, model defaults, explicit instructions or configuration can change a request despite an unchanged visible model selector. Provider expiry, eligibility and routing remain external conditions; no local fix can guarantee every request inside a TTL gets a hit.

## Evidence, verification and cleanup

Sanitized request shapes, usage and comparisons: [compaction-resume-probe.json](cache-repair-reports/compaction-resume-probe.json). The full private archive is `/Users/viktor/.codex/cache-diagnostics/experiments/compaction-resume-20260914/`. The final controlled run is `codex-prefix-probe-p1byfypd`; its script, event log, raw trace bundles, provenance and analysis are preserved. Two earlier runs are also retained: a preliminary run had a probe lifecycle error (it submitted input after item completion but before compaction turn completion); the next completed run omitted developer overrides on resume. Neither is substituted for the controlled result.

A fresh Sol verifier independently read the raw requests/responses, reconstructed WebSocket deltas, checked present fields and the embedded Lite tools/base items, isolated metadata changes, and corroborated both cold-resume cache counts and movement of the unchanged developer section. The analyzer was refined to report only fields present in the actual requests and to compare embedded tools/base separately. Its JSON-value comparisons are structural equality, not a claim that full request wire bytes are identical.

Source and fixture reviewers used codebase-memory with direct source reads. Relied-on source paths had matching index metadata and no recorded gaps; snapshot files were read directly because they are not graph-indexed. No Rust build or test artifacts were created. Probe servers exited, raw diagnostic evidence was moved out of temporary storage into the permanent private archive, and existing user work was preserved. The user's resume-verification requirement was added to root `AGENTS.md`.
