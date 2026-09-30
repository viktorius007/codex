#!/usr/bin/env python3

import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
from typing import Optional
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import audit_cache_diagnostics


WARM_CACHED_TOKENS = 16_384
PRIVATE_CANARY = "PRIVATE-DIAGNOSTIC-VALUE"


def fingerprint(label: str, byte_count: int = 16) -> dict[str, object]:
    digest = hashlib.sha256(label.encode()).digest()
    hmac = base64.urlsafe_b64encode(digest).decode().rstrip("=")
    return {"hmac": hmac, "bytes": byte_count}


def missing() -> dict[str, str]:
    return {"status": "missing"}


def unavailable() -> dict[str, str]:
    return {"status": "unavailable"}


def list_manifest(label: str, items: list[str]) -> dict[str, object]:
    return {
        "body": fingerprint(f"{label}-body"),
        "count": len(items),
        "retained": [fingerprint(item) for item in items],
        "omitted": {"count": 0, "hmac": fingerprint("empty-tail")["hmac"]},
    }


def input_manifest_with_bytes(
    labels_and_bytes: list[tuple[str, int]],
) -> dict[str, object]:
    """Build a `logical.input` manifest with explicit per-entry byte counts."""
    labels = [label for label, _ in labels_and_bytes]
    return {
        "body": fingerprint(f"input-body-{'-'.join(labels)}"),
        "count": len(labels_and_bytes),
        "retained": [
            fingerprint(label, byte_count=byte_count)
            for label, byte_count in labels_and_bytes
        ],
        "omitted": {"count": 0, "hmac": fingerprint("empty-tail")["hmac"]},
    }


def tools_detail_manifest(
    tools: list[tuple[str, int, int]],
) -> dict[str, object]:
    """Build a `logical.toolsDetail` manifest.

    `tools` is a list of (name, description_bytes, parameters_bytes) tuples.
    Tool `name` is plaintext; descriptor/description/parameters remain
    fingerprints (hmac, bytes).
    """
    retained = [
        {
            "name": name,
            "descriptor": fingerprint(f"{name}-descriptor"),
            "description": fingerprint(f"{name}-description", byte_count=desc_bytes),
            "parameters": fingerprint(f"{name}-parameters", byte_count=params_bytes),
        }
        for name, desc_bytes, params_bytes in tools
    ]
    return {
        "count": len(retained),
        "retained": retained,
        "omitted": {"count": 0, "hmac": fingerprint("empty-tail")["hmac"]},
    }


def transport_identity(
    *, routing_hint: str = "routing-a", previous_response: bool = False
) -> dict[str, object]:
    return {
        "sessionId": fingerprint("transport-session"),
        "threadId": fingerprint("transport-thread"),
        "clientRequestId": fingerprint("transport-client-request"),
        "subagent": missing(),
        "routingHint": fingerprint(routing_hint),
        "responsesLite": missing(),
        "previousResponseId": (
            fingerprint("transport-previous-response")
            if previous_response
            else missing()
        ),
        "originator": fingerprint("transport-originator"),
        "userAgent": fingerprint("transport-user-agent"),
    }


def observed_wire(
    label: str,
    *,
    websocket: bool = False,
    transport_available: bool = True,
    connection_reused: bool = False,
    incremental: bool = False,
    routing_hint: str = "routing-a",
    previous_response: bool = False,
) -> dict[str, object]:
    transport: object = unavailable()
    if transport_available:
        transport = {
            "endpoint": "responses",
            "compression": unavailable() if websocket else "none",
            "connectionReused": connection_reused if websocket else unavailable(),
            "incremental": incremental if websocket else unavailable(),
            "identity": transport_identity(
                routing_hint=routing_hint,
                previous_response=previous_response,
            ),
        }
    return {
        "kind": "websocket" if websocket else "http",
        "body": fingerprint(label),
        "transport": transport,
    }


def logical_manifest(
    *,
    instructions: str = "instructions-a",
    input_items: tuple[str, ...] = ("input-0",),
    tool_items: tuple[str, ...] = ("tool-0",),
    model: str = "model-a",
    reasoning: str = "effort-medium",
    input_manifest: Optional[dict[str, object]] = None,
    tools_detail: Optional[dict[str, object]] = None,
) -> dict[str, object]:
    resolved_input_manifest = input_manifest or list_manifest(
        "input", list(input_items)
    )
    tools_manifest = list_manifest("tools", list(tool_items))
    manifest = {
        "body": fingerprint(
            f"body-{model}-{reasoning}-{instructions}-"
            f"{'-'.join(input_items)}-{'-'.join(tool_items)}"
        ),
        "model": fingerprint(model),
        "instructions": fingerprint(instructions),
        "input": resolved_input_manifest,
        "tools": tools_manifest,
        "toolChoice": fingerprint("tool-choice"),
        "parallelToolCalls": fingerprint("parallel-tool-calls"),
        "reasoning": fingerprint(reasoning),
        "store": fingerprint("store"),
        "stream": fingerprint("stream"),
        "streamOptions": fingerprint("stream-options"),
        "include": fingerprint("include"),
        "serviceTier": missing(),
        "promptCacheKey": fingerprint("cache-key"),
        "text": missing(),
        "clientMetadata": missing(),
        "accessPrograms": missing(),
    }
    if tools_detail is not None:
        manifest["toolsDetail"] = tools_detail
    return manifest


def request_record(
    attempt: str,
    *,
    sequence: int,
    timestamp_ms: Optional[int],
    run_id: str = "run-a",
    key_scope: Optional[str] = "key-scope-a",
    thread: str = "thread-a",
    logical: Optional[dict[str, object]] = None,
    wire: Optional[str] = None,
    websocket: bool = False,
    previous_response: bool = False,
    transport_available: bool = True,
    connection_reused: bool = False,
    incremental: bool = False,
    routing_hint: str = "routing-a",
    request_kind: str = "turn",
    schema_version: int = 1,
    continuation: Optional[dict[str, object]] = None,
    tool_provenance: Optional[dict[str, object]] = None,
) -> dict[str, object]:
    if wire is None:
        wire_manifest: dict[str, object] = missing()
    else:
        wire_manifest = observed_wire(
            wire,
            websocket=websocket,
            transport_available=transport_available,
            connection_reused=connection_reused,
            incremental=incremental,
            routing_hint=routing_hint,
            previous_response=previous_response,
        )
    record = {
        "schemaVersion": schema_version,
        "event": "request",
        "runId": run_id,
        "sequence": sequence,
        "timestampUnixMs": timestamp_ms,
        "requestKind": request_kind,
        "retryOrdinal": 0,
        "lineage": {
            "threadId": fingerprint(thread),
            "sessionId": fingerprint("session-a"),
            "turnId": fingerprint(f"turn-{attempt}"),
            "parentId": missing(),
            "affinityId": fingerprint("affinity-a"),
            "previousResponseId": (
                fingerprint("previous-response") if previous_response else missing()
            ),
        },
        "logical": logical or logical_manifest(),
        "wire": wire_manifest,
    }
    if key_scope is not None:
        record["keyScope"] = fingerprint(key_scope, byte_count=0)
    if continuation is not None:
        record["continuation"] = continuation
    if tool_provenance is not None:
        record["toolProvenance"] = tool_provenance
    record["attemptId"] = attempt
    return record


def outcome_record(
    attempt: str,
    *,
    sequence: int,
    timestamp_ms: Optional[int],
    cached_input: int,
    input_tokens: int = 20_000,
    run_id: str = "run-a",
    key_scope: Optional[str] = "key-scope-a",
    terminal: str = "completed",
    usage_present: bool = True,
    response_present: Optional[bool] = None,
) -> dict[str, object]:
    usage = None
    if terminal == "completed" and usage_present:
        usage = {
            "input": input_tokens,
            "cachedInput": cached_input,
            "cacheWrite": 0,
            "output": 200,
            "reasoning": 50,
            "total": input_tokens + 200,
        }
    if response_present is None:
        response_present = terminal == "completed"
    record = {
        "schemaVersion": 1,
        "event": "outcome",
        "runId": run_id,
        "sequence": sequence,
        "timestampUnixMs": timestamp_ms,
        "terminal": terminal,
        "responseId": (
            fingerprint(f"response-{attempt}") if response_present else missing()
        ),
        "usage": usage,
    }
    if key_scope is not None:
        record["keyScope"] = fingerprint(key_scope, byte_count=0)
    record["attemptId"] = attempt
    return record


def completed_attempt_records(
    attempt: str,
    *,
    sequence: int,
    timestamp_ms: int,
    cached_input: int,
    run_id: str,
    logical: Optional[dict[str, object]] = None,
    wire: str,
    thread: str = "thread-a",
    key_scope: Optional[str] = "key-scope-a",
) -> list[dict[str, object]]:
    return [
        request_record(
            attempt,
            sequence=sequence,
            timestamp_ms=timestamp_ms,
            run_id=run_id,
            key_scope=key_scope,
            thread=thread,
            logical=logical,
            wire=wire,
        ),
        outcome_record(
            attempt,
            sequence=sequence + 1,
            timestamp_ms=timestamp_ms + 1_000,
            run_id=run_id,
            key_scope=key_scope,
            cached_input=cached_input,
        ),
    ]


class CacheDiagnosticsAuditTest(unittest.TestCase):
    def run_json(
        self,
        files: dict[str, list[dict[str, object]]],
        *extra_args: str,
    ) -> tuple[dict[str, object], str]:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            for name, records in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    "".join(
                        json.dumps(record, separators=(",", ":")) + "\n"
                        for record in records
                    ),
                    encoding="utf-8",
                )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = audit_cache_diagnostics.main(
                    [str(root), "--json", *extra_args]
                )
            self.assertEqual(status, 0)
            return json.loads(output.getvalue()), str(root)

    def run_text(
        self, records: list[dict[str, object]], *extra_args: str
    ) -> tuple[str, str]:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "run-a.jsonl"
            path.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = audit_cache_diagnostics.main([str(root), *extra_args])
            self.assertEqual(status, 0)
            return output.getvalue(), str(root)

    def test_interleaved_outcomes_are_correlated_by_attempt(self) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        records = [
            request_record(
                "attempt-a",
                sequence=1,
                timestamp_ms=1_000,
                logical=first,
                wire="wire-a",
            ),
            request_record(
                "attempt-b",
                sequence=2,
                timestamp_ms=61_000,
                logical=second,
                wire="wire-b",
            ),
            outcome_record(
                "attempt-b", sequence=3, timestamp_ms=62_000, cached_input=0
            ),
            outcome_record(
                "attempt-a",
                sequence=4,
                timestamp_ms=63_000,
                cached_input=WARM_CACHED_TOKENS,
            ),
        ]

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        self.assertEqual(len(report["incidents"]), 1)
        self.assertEqual(
            report["incidents"][0]["first_difference"],
            {"scope": "component", "name": "instructions"},
        )
        self.assertEqual(
            report["incidents"][0]["assessment"],
            "request_difference_candidate",
        )
        self.assertEqual(
            report["incidents"][0]["estimated_lost_cached_tokens"],
            WARM_CACHED_TOKENS,
        )
        self.assertIn("candidate", text_output.lower())
        self.assertNotIn("preventable", text_output.lower())

    def test_first_changed_input_or_tool_ordinal_is_reported(self) -> None:
        cases = {
            "input": (
                logical_manifest(input_items=("input-0", "input-old")),
                logical_manifest(input_items=("input-0", "input-new")),
            ),
            "tool": (
                logical_manifest(tool_items=("tool-0", "tool-old")),
                logical_manifest(tool_items=("tool-0", "tool-new")),
            ),
        }
        for scope, (first, second) in cases.items():
            with self.subTest(scope=scope):
                records = self.warm_drop_pair(first, second)
                report, _ = self.run_json({"run-a.jsonl": records})

                self.assertEqual(
                    report["incidents"][0]["first_difference"],
                    {"scope": scope, "ordinal": 1},
                )

    def test_append_only_input_is_distinguished_from_a_rewrite(self) -> None:
        first = logical_manifest(input_items=("input-0", "input-1"))
        appended = logical_manifest(input_items=("input-0", "input-1", "input-2"))
        rewritten = logical_manifest(input_items=("input-0", "input-rewritten"))

        append_report, _ = self.run_json(
            {"run-a.jsonl": self.warm_drop_pair(first, appended)}
        )
        rewrite_report, _ = self.run_json(
            {"run-a.jsonl": self.warm_drop_pair(first, rewritten)}
        )

        self.assertEqual(
            append_report["incidents"][0]["logical_input_relation"],
            "prior_is_prefix",
        )
        self.assertEqual(append_report["incidents"][0]["common_input_prefix_count"], 2)
        self.assertEqual(
            rewrite_report["incidents"][0]["logical_input_relation"], "rewrite"
        )
        self.assertEqual(rewrite_report["incidents"][0]["common_input_prefix_count"], 1)

    def test_websocket_wire_change_uses_the_full_logical_prefix_evidence(self) -> None:
        first = logical_manifest(input_items=("input-0", "input-1"))
        second = logical_manifest(input_items=("input-0", "input-1", "input-2"))
        records = self.warm_drop_pair(
            first,
            second,
            websocket=True,
            previous_response=True,
            incremental_second=True,
        )

        report, _ = self.run_json({"run-a.jsonl": records})

        incident = report["incidents"][0]
        self.assertEqual(incident["logical_input_relation"], "prior_is_prefix")
        self.assertEqual(incident["wire_observation"], "websocket_incremental")
        self.assertNotEqual(
            incident["first_difference"], {"scope": "wire", "name": "body"}
        )

    def test_transport_identity_change_is_localized_with_equal_request_bodies(
        self,
    ) -> None:
        logical = logical_manifest()
        records = self.warm_drop_pair(logical, logical)
        records[2]["wire"] = observed_wire("wire-a", routing_hint="routing-b")

        report, _ = self.run_json({"run-a.jsonl": records})

        incident = report["incidents"][0]
        self.assertEqual(
            incident["first_difference"],
            {"scope": "transport_identity", "name": "routingHint"},
        )
        self.assertEqual(incident["assessment"], "request_difference_candidate")

    def test_model_or_reasoning_change_is_excluded_from_confident_comparison(
        self,
    ) -> None:
        cases = {
            "model": (
                logical_manifest(model="model-a"),
                logical_manifest(model="model-b"),
            ),
            "reasoning": (
                logical_manifest(reasoning="effort-low"),
                logical_manifest(reasoning="effort-high"),
            ),
        }
        for field, (first, second) in cases.items():
            with self.subTest(field=field):
                report, _ = self.run_json(
                    {"run-a.jsonl": self.warm_drop_pair(first, second)}
                )

                self.assertEqual(report["incidents"], [])
                self.assertEqual(report["stats"]["incompatible_comparisons"], 1)

    def test_equal_observed_hashes_do_not_claim_a_backend_fault(self) -> None:
        logical = logical_manifest()
        records = self.warm_drop_pair(logical, logical)
        records[2]["wire"] = records[0]["wire"]

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        self.assertEqual(
            report["incidents"][0]["assessment"],
            "unexplained_observed_equality",
        )
        self.assertEqual(report["incidents"][0]["evidence_status"], "indeterminate")
        self.assertIsNone(report["incidents"][0]["first_difference"])
        self.assertIn("no observed request difference", text_output.lower())
        self.assertNotIn("backend fault", text_output.lower())

    def test_missing_or_unavailable_wire_evidence_is_never_confident(self) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        cases = {
            "missing_wire": missing(),
            "unavailable_transport": observed_wire("wire-b", transport_available=False),
        }
        for case, second_wire in cases.items():
            with self.subTest(case=case):
                records = self.warm_drop_pair(first, second)
                records[2]["wire"] = second_wire

                report, _ = self.run_json({"run-a.jsonl": records})

                self.assertEqual(report["incidents"][0]["evidence_status"], "partial")
                self.assertGreaterEqual(
                    report["stats"]["missing_evidence_comparisons"], 1
                )

    def test_records_without_key_scope_are_not_compared_within_or_across_runs(
        self,
    ) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        same_run = self.warm_drop_pair(first, second)
        for record in same_run:
            record.pop("keyScope")
        files = {
            "run-a.jsonl": completed_attempt_records(
                "attempt-a",
                sequence=1,
                timestamp_ms=1_000,
                cached_input=WARM_CACHED_TOKENS,
                run_id="run-a",
                key_scope=None,
                logical=first,
                wire="wire-a",
            ),
            "run-b.jsonl": completed_attempt_records(
                "attempt-b",
                sequence=1,
                timestamp_ms=61_000,
                cached_input=0,
                run_id="run-b",
                key_scope=None,
                logical=second,
                wire="wire-b",
            ),
        }

        same_run_report, _ = self.run_json({"run-same.jsonl": same_run})
        cross_run_report, _ = self.run_json(files)

        self.assertEqual(same_run_report["incidents"], [])
        self.assertEqual(same_run_report["stats"]["completed_attempts"], 0)
        self.assertGreaterEqual(
            same_run_report["stats"]["missing_key_scope_comparisons"], 1
        )
        self.assertEqual(cross_run_report["incidents"], [])
        self.assertGreaterEqual(
            cross_run_report["stats"]["missing_key_scope_comparisons"], 1
        )

    def test_matching_key_scope_allows_comparison_across_run_files(self) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        files = {
            "2026/09/13/run-a.jsonl": completed_attempt_records(
                "attempt-a",
                sequence=1,
                timestamp_ms=1_000,
                cached_input=WARM_CACHED_TOKENS,
                run_id="run-a",
                logical=first,
                wire="wire-a",
            ),
            "2026/09/14/run-b.jsonl": completed_attempt_records(
                "attempt-b",
                sequence=1,
                timestamp_ms=61_000,
                cached_input=0,
                run_id="run-b",
                logical=second,
                wire="wire-b",
            ),
        }

        report, _ = self.run_json(files)

        self.assertEqual(len(report["incidents"]), 1)
        self.assertEqual(
            report["incidents"][0]["first_difference"],
            {"scope": "component", "name": "instructions"},
        )

    def test_affinity_tie_cannot_suppress_a_same_thread_predecessor(self) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        current_run = self.warm_drop_pair(first, second, run_id="run-current")
        affinity_only_run = completed_attempt_records(
            "affinity-only",
            sequence=1,
            timestamp_ms=1_000,
            cached_input=WARM_CACHED_TOKENS,
            run_id="run-affinity",
            thread="different-thread",
            logical=logical_manifest(instructions="affinity-only"),
            wire="wire-affinity",
        )

        report, _ = self.run_json(
            {
                "run-current.jsonl": current_run,
                "run-affinity.jsonl": affinity_only_run,
            }
        )

        self.assertEqual(report["stats"]["order_ambiguous_comparisons"], 0)
        self.assertEqual(report["stats"]["eligible_warm_comparisons"], 1)
        self.assertEqual(len(report["incidents"]), 1)
        self.assertEqual(report["incidents"][0]["comparison_kind"], "same_thread")
        self.assertEqual(
            report["incidents"][0]["first_difference"],
            {"scope": "component", "name": "instructions"},
        )

    def test_request_and_outcome_key_scope_must_match(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )
        records[-1]["keyScope"] = fingerprint("different-key-scope", byte_count=0)

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(report["incidents"], [])
        self.assertEqual(report["stats"]["key_scope_mismatches"], 1)

    def test_null_timestamps_are_valid_but_ineligible_for_timed_comparison(
        self,
    ) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )
        records[2]["timestampUnixMs"] = None
        records[3]["timestampUnixMs"] = None

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(report["stats"]["malformed_lines"], 0)
        self.assertEqual(report["stats"]["request_records"], 2)
        self.assertEqual(report["stats"]["outcome_records"], 2)
        self.assertEqual(report["stats"]["eligible_warm_comparisons"], 0)
        self.assertGreaterEqual(report["stats"]["missing_evidence_comparisons"], 1)
        self.assertEqual(report["incidents"], [])

    def test_non_comparable_emitted_outcomes_are_valid_records(self) -> None:
        records = [
            request_record("no-usage", sequence=1, timestamp_ms=1_000, wire="wire-a"),
            outcome_record(
                "no-usage",
                sequence=2,
                timestamp_ms=2_000,
                cached_input=0,
                usage_present=False,
            ),
            request_record("failed", sequence=3, timestamp_ms=3_000, wire="wire-b"),
            outcome_record(
                "failed",
                sequence=4,
                timestamp_ms=4_000,
                cached_input=0,
                terminal="failed",
            ),
            request_record("signed", sequence=5, timestamp_ms=5_000, wire="wire-c"),
            outcome_record(
                "signed",
                sequence=6,
                timestamp_ms=6_000,
                cached_input=-1,
                input_tokens=-1,
            ),
        ]

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(records[3]["responseId"], missing())
        self.assertEqual(report["stats"]["malformed_lines"], 0)
        self.assertEqual(report["stats"]["outcome_records"], 3)
        self.assertEqual(report["stats"]["completed_attempts"], 0)
        self.assertEqual(report["incidents"], [])

    def test_pre_and_post_windows_report_normalized_rates_ranges_and_config(
        self,
    ) -> None:
        before = self.warm_drop_pair(
            logical_manifest(instructions="before-a"),
            logical_manifest(instructions="before-b"),
            start_ms=1_000,
            run_id="run-before",
        )
        after = self.warm_drop_pair(
            logical_manifest(instructions="after-a"),
            logical_manifest(instructions="after-b"),
            start_ms=201_000,
            run_id="run-after",
            second_cached=WARM_CACHED_TOKENS,
        )

        report, _ = self.run_json(
            {"run-before.jsonl": before, "run-after.jsonl": after},
            "--split-at-unix-ms",
            "200000",
        )
        text_output, _ = self.run_text(
            before + after,
            "--split-at-unix-ms",
            "200000",
        )

        self.assertEqual(
            report["config"],
            {
                "warm_window_minutes": 30.0,
                "min_cached_tokens": 1024,
                "min_lost_tokens": 1024,
                "drop_fraction": 0.5,
                "split_at_unix_ms": 200_000,
            },
        )
        self.assertEqual(
            report["windows"],
            [
                {
                    "name": "before",
                    "start_unix_ms": 1_000,
                    "end_unix_ms": 61_000,
                    "eligible_warm_comparisons": 1,
                    "cache_drops": 1,
                    "cache_drop_rate": 1.0,
                    "expected_cached_tokens": WARM_CACHED_TOKENS,
                    "observed_cached_tokens": 0,
                    "cached_token_retention_rate": 0.0,
                },
                {
                    "name": "after",
                    "start_unix_ms": 201_000,
                    "end_unix_ms": 261_000,
                    "eligible_warm_comparisons": 1,
                    "cache_drops": 0,
                    "cache_drop_rate": 0.0,
                    "expected_cached_tokens": WARM_CACHED_TOKENS,
                    "observed_cached_tokens": WARM_CACHED_TOKENS,
                    "cached_token_retention_rate": 1.0,
                },
            ],
        )
        self.assertRegex(text_output.lower(), r"before:.*range=1000\.\.61000")
        self.assertRegex(text_output.lower(), r"after:.*range=201000\.\.261000")

    def test_empty_pre_post_windows_use_null_rates_and_explicit_text(self) -> None:
        report, _ = self.run_json({}, "--split-at-unix-ms", "200000")
        text_output, _ = self.run_text([], "--split-at-unix-ms", "200000")

        self.assertEqual(
            [window["name"] for window in report["windows"]], ["before", "after"]
        )
        for window in report["windows"]:
            self.assertIsNone(window["start_unix_ms"])
            self.assertIsNone(window["end_unix_ms"])
            self.assertEqual(window["eligible_warm_comparisons"], 0)
            self.assertIsNone(window["cache_drop_rate"])
            self.assertEqual(window["expected_cached_tokens"], 0)
            self.assertIsNone(window["cached_token_retention_rate"])
        self.assertRegex(text_output.lower(), r"before:.*range=no observed timestamps")
        self.assertRegex(text_output.lower(), r"after:.*range=no observed timestamps")
        self.assertEqual(text_output.lower().count("cache_drop_rate=no data"), 2)
        self.assertEqual(text_output.lower().count("retention=no data"), 2)

    def test_reports_never_echo_unknown_values_or_source_paths(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )
        for record in records:
            if record["event"] == "request":
                record["unknownPrivateField"] = PRIVATE_CANARY

        json_report, root = self.run_json({"private/run-a.jsonl": records})
        text_output, text_root = self.run_text(records)
        rendered_json = json.dumps(json_report, sort_keys=True)

        self.assertNotIn(PRIVATE_CANARY, rendered_json)
        self.assertNotIn(root, rendered_json)
        self.assertNotIn(PRIVATE_CANARY, text_output)
        self.assertNotIn(text_root, text_output)
        self.assertNotIn("attempt-a", rendered_json)
        self.assertNotIn("run-a", rendered_json)

    def test_oversized_lines_and_sql_integers_are_skipped_without_aborting(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "run-a.jsonl"
            oversized_secret = PRIVATE_CANARY * 32_768
            too_large_sequence = request_record(
                "too-large-sequence",
                sequence=2**63,
                timestamp_ms=1_000,
                wire="wire-too-large",
            )
            valid_records = self.warm_drop_pair(
                logical_manifest(instructions="instructions-a"),
                logical_manifest(instructions="instructions-b"),
            )
            path.write_text(
                json.dumps({"event": "request", "value": oversized_secret})
                + "\n"
                + json.dumps(too_large_sequence, separators=(",", ":"))
                + "\n"
                + "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in valid_records
                ),
                encoding="utf-8",
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = audit_cache_diagnostics.main([str(root), "--json"])

            self.assertEqual(status, 0)
            report = json.loads(output.getvalue())
            self.assertEqual(report["stats"]["oversized_lines"], 1)
            self.assertEqual(report["stats"]["malformed_lines"], 1)
            self.assertEqual(len(report["incidents"]), 1)
            self.assertNotIn(PRIVATE_CANARY, output.getvalue())

    def test_scanning_a_dated_archive_does_not_modify_evidence(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "2026" / "09" / "14" / "run-a.jsonl"
            path.parent.mkdir(parents=True)
            original = "".join(
                json.dumps(record, separators=(",", ":")) + "\n" for record in records
            ).encode()
            path.write_bytes(original)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = audit_cache_diagnostics.main([str(root), "--json"])

            self.assertEqual(status, 0)
            self.assertEqual(path.read_bytes(), original)
            self.assertEqual(json.loads(output.getvalue())["stats"]["files"], 1)

    def test_cross_kind_comparison_is_excluded_from_headline_drops_and_listed_separately(
        self,
    ) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
            first_request_kind="warmup",
            second_request_kind="turn",
        )

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        self.assertEqual(report["incidents"], [])
        self.assertEqual(report["stats"]["cache_drops"], 0)
        self.assertEqual(report["stats"]["eligible_warm_comparisons"], 0)
        self.assertEqual(report["stats"]["cross_kind_comparisons"], 1)
        self.assertEqual(len(report["cross_kind_comparisons"]), 1)
        pair = report["cross_kind_comparisons"][0]
        self.assertEqual(pair["previous_request_kind"], "warmup")
        self.assertEqual(pair["current_request_kind"], "turn")
        self.assertIn("CROSS-KIND COMPARISONS", text_output)
        self.assertIn("warmup", text_output)

    def test_same_kind_comparison_is_unaffected_by_cross_kind_handling(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
            first_request_kind="turn",
            second_request_kind="turn",
        )

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(report["stats"]["cross_kind_comparisons"], 0)
        self.assertEqual(report["stats"]["cache_drops"], 1)
        self.assertEqual(len(report["incidents"]), 1)

    def test_incident_reports_granularity_of_first_differing_retained_input(
        self,
    ) -> None:
        first = logical_manifest(
            input_manifest=input_manifest_with_bytes(
                [("input-0", 100), ("input-1", 200), ("input-2", 300)]
            )
        )
        second = logical_manifest(
            input_manifest=input_manifest_with_bytes(
                [("input-0", 100), ("input-1-changed", 250), ("input-2", 300)]
            )
        )
        records = self.warm_drop_pair(first, second)

        report, _ = self.run_json({"run-a.jsonl": records})

        incident = report["incidents"][0]
        self.assertEqual(incident["first_differing_input_ordinal"], 1)
        self.assertEqual(incident["first_differing_input_bytes_previous"], 200)
        self.assertEqual(incident["first_differing_input_bytes_current"], 250)
        self.assertEqual(incident["identical_retained_before_first_diff"], 1)
        self.assertEqual(incident["identical_retained_after_first_diff"], 1)
        self.assertEqual(incident["input_count_change"], 0)
        rendered = json.dumps(report, sort_keys=True)
        # Only ordinals/byte counts are reported, never fingerprint/hmac values.
        self.assertNotIn("hmac", rendered)

    def test_incident_reports_input_count_change_on_append(self) -> None:
        first = logical_manifest(input_items=("input-0",))
        second = logical_manifest(input_items=("input-0", "input-1"))
        records = self.warm_drop_pair(first, second)

        report, _ = self.run_json({"run-a.jsonl": records})

        incident = report["incidents"][0]
        self.assertEqual(incident["input_count_change"], 1)

    def test_tool_catalog_drift_candidate_boundary_is_inclusive_at_64_bytes(
        self,
    ) -> None:
        first = logical_manifest(
            input_manifest=input_manifest_with_bytes([("input-0", 100)])
        )
        second = logical_manifest(
            input_manifest=input_manifest_with_bytes([("input-0", 164)])
        )
        records = self.warm_drop_pair(first, second)

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(
            report["incidents"][0]["classification"], "tool_catalog_drift_candidate"
        )

    def test_tool_catalog_drift_candidate_boundary_excludes_65_bytes(self) -> None:
        first = logical_manifest(
            input_manifest=input_manifest_with_bytes([("input-0", 100)])
        )
        second = logical_manifest(
            input_manifest=input_manifest_with_bytes([("input-0", 165)])
        )
        records = self.warm_drop_pair(first, second)

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertIsNone(report["incidents"][0]["classification"])

    def test_drift_candidate_requires_the_only_difference_to_be_ordinal_zero(
        self,
    ) -> None:
        first = logical_manifest(
            input_manifest=input_manifest_with_bytes(
                [("input-0", 100), ("input-1", 200)]
            )
        )
        second = logical_manifest(
            input_manifest=input_manifest_with_bytes(
                [("input-0", 110), ("input-1-changed", 200)]
            )
        )
        records = self.warm_drop_pair(first, second)

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertIsNone(report["incidents"][0]["classification"])
        self.assertEqual(report["incidents"][0]["first_differing_input_ordinal"], 0)

    def test_continuation_field_is_reported_when_present(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
            second_schema_version=2,
            second_continuation={
                "transport": "http",
                "decision": "full",
                "dropReason": "inputPrefixMismatch",
                "firstMismatchedInputIndex": 3,
                "divergence": {
                    "scope": "input",
                    "byteOffset": 128,
                    "previousBytes": 64,
                    "currentBytes": 96,
                },
            },
        )

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        continuation = report["incidents"][0]["continuation"]
        self.assertEqual(continuation["decision"], "full")
        self.assertEqual(continuation["dropReason"], "inputPrefixMismatch")
        self.assertEqual(continuation["firstMismatchedInputIndex"], 3)
        self.assertEqual(
            continuation["divergence"],
            {
                "scope": "input",
                "byteOffset": 128,
                "previousBytes": 64,
                "currentBytes": 96,
            },
        )
        self.assertIn("continuation:", text_output)
        self.assertIn("decision=full", text_output)

    def test_continuation_field_is_absent_when_not_present(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertIsNone(report["incidents"][0]["continuation"])

    def test_tool_provenance_field_is_reported_when_present(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
            first_schema_version=2,
            second_schema_version=2,
            first_tool_provenance={
                "modelCatalog": {
                    "presetCount": 5,
                    "lockContentionFallback": False,
                    "identity": fingerprint("catalog-identity-a"),
                },
                "roleFileReadFailures": 0,
            },
            second_tool_provenance={
                "modelCatalog": {
                    "presetCount": 7,
                    "lockContentionFallback": True,
                    "identity": fingerprint("catalog-identity-b"),
                },
                "roleFileReadFailures": 2,
            },
        )

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        provenance = report["incidents"][0]["tool_provenance"]
        self.assertEqual(provenance["presetCount"], 7)
        self.assertEqual(provenance["lockContentionFallback"], True)
        self.assertEqual(provenance["roleFileReadFailures"], 2)
        self.assertEqual(
            provenance["presetCountChanged"], {"previous": 5, "current": 7}
        )
        self.assertIn("lockContentionFallback=True", text_output)
        self.assertIn("WARNING: lockContentionFallback=true", text_output)
        self.assertIn("presetCount differs", text_output)

    def test_tool_provenance_field_is_absent_when_not_present(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertIsNone(report["incidents"][0]["tool_provenance"])

    def test_tools_detail_diff_is_reported_when_present_on_both_requests(
        self,
    ) -> None:
        first_logical = logical_manifest(
            tools_detail=tools_detail_manifest(
                [("tool_alpha", 50, 20), ("tool_beta", 30, 10)]
            )
        )
        second_logical = logical_manifest(
            tools_detail=tools_detail_manifest(
                [("tool_alpha", 90, 20), ("tool_gamma", 15, 15)]
            )
        )
        records = self.warm_drop_pair(first_logical, second_logical)

        report, _ = self.run_json({"run-a.jsonl": records})
        text_output, _ = self.run_text(records)

        diff = report["incidents"][0]["tools_detail_diff"]
        self.assertEqual(diff["added"], ["tool_gamma"])
        self.assertEqual(diff["removed"], ["tool_beta"])
        self.assertEqual(len(diff["changed"]), 1)
        changed = diff["changed"][0]
        self.assertEqual(changed["name"], "tool_alpha")
        self.assertEqual(len(changed["parts"]), 1)
        self.assertEqual(changed["parts"][0]["part"], "description")
        self.assertEqual(changed["parts"][0]["previous_bytes"], 50)
        self.assertEqual(changed["parts"][0]["current_bytes"], 90)
        self.assertEqual(changed["parts"][0]["byte_delta"], 40)
        # Tool names are plaintext by design and may be printed.
        self.assertIn("tool_alpha", text_output)
        self.assertIn("tool_gamma", text_output)
        self.assertIn("tool_beta", text_output)

    def test_tools_detail_diff_is_absent_when_missing_on_either_request(self) -> None:
        first_logical = logical_manifest(
            tools_detail=tools_detail_manifest([("tool_alpha", 50, 20)])
        )
        second_logical = logical_manifest()
        records = self.warm_drop_pair(first_logical, second_logical)

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertIsNone(report["incidents"][0]["tools_detail_diff"])

    def test_v2_schema_version_records_still_parse_through_v1_code_paths(
        self,
    ) -> None:
        first = logical_manifest(instructions="instructions-a")
        second = logical_manifest(instructions="instructions-b")
        records = self.warm_drop_pair(
            first, second, first_schema_version=2, second_schema_version=2
        )

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(report["stats"]["malformed_lines"], 0)
        self.assertEqual(len(report["incidents"]), 1)
        self.assertEqual(
            report["incidents"][0]["first_difference"],
            {"scope": "component", "name": "instructions"},
        )
        self.assertIsNone(report["incidents"][0]["continuation"])
        self.assertIsNone(report["incidents"][0]["tool_provenance"])
        self.assertIsNone(report["incidents"][0]["tools_detail_diff"])

    def test_malformed_v2_continuation_field_rejects_the_record(self) -> None:
        records = self.warm_drop_pair(
            logical_manifest(instructions="instructions-a"),
            logical_manifest(instructions="instructions-b"),
        )
        records[2]["continuation"] = {"transport": "http", "decision": "bogus"}

        report, _ = self.run_json({"run-a.jsonl": records})

        self.assertEqual(report["incidents"], [])
        self.assertGreaterEqual(report["stats"]["malformed_lines"], 1)

    @staticmethod
    def warm_drop_pair(
        first: dict[str, object],
        second: dict[str, object],
        *,
        start_ms: int = 1_000,
        run_id: str = "run-a",
        second_cached: int = 0,
        websocket: bool = False,
        previous_response: bool = False,
        incremental_second: bool = False,
        first_request_kind: str = "turn",
        second_request_kind: str = "turn",
        first_schema_version: int = 1,
        second_schema_version: int = 1,
        first_continuation: Optional[dict[str, object]] = None,
        second_continuation: Optional[dict[str, object]] = None,
        first_tool_provenance: Optional[dict[str, object]] = None,
        second_tool_provenance: Optional[dict[str, object]] = None,
    ) -> list[dict[str, object]]:
        return [
            request_record(
                "attempt-a",
                sequence=1,
                timestamp_ms=start_ms,
                run_id=run_id,
                logical=first,
                wire="wire-a",
                websocket=websocket,
                connection_reused=False,
                incremental=False,
                request_kind=first_request_kind,
                schema_version=first_schema_version,
                continuation=first_continuation,
                tool_provenance=first_tool_provenance,
            ),
            outcome_record(
                "attempt-a",
                sequence=2,
                timestamp_ms=start_ms + 1_000,
                run_id=run_id,
                cached_input=WARM_CACHED_TOKENS,
            ),
            request_record(
                "attempt-b",
                sequence=3,
                timestamp_ms=start_ms + 60_000,
                run_id=run_id,
                logical=second,
                wire="wire-b",
                websocket=websocket,
                previous_response=previous_response,
                connection_reused=incremental_second,
                incremental=incremental_second,
                request_kind=second_request_kind,
                schema_version=second_schema_version,
                continuation=second_continuation,
                tool_provenance=second_tool_provenance,
            ),
            outcome_record(
                "attempt-b",
                sequence=4,
                timestamp_ms=start_ms + 61_000,
                run_id=run_id,
                cached_input=second_cached,
            ),
        ]


if __name__ == "__main__":
    unittest.main()
