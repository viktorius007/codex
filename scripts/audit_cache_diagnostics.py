#!/usr/bin/env python3
"""Analyze privacy-preserving prompt-cache diagnostic records offline.

Warm comparisons whose two requests carry different requestKind values
(e.g. warmup vs turn) are excluded from the headline cache_drops count and
rate; they are counted and listed separately under CROSS-KIND COMPARISONS.
Same-requestKind pairs keep the existing semantics.

Each detailed incident additionally reports, from byte sizes, ordinals, and
counts already present in the v1 record (never fingerprint/hmac values):
the first differing retained logical.input item, that item's byte size on
both requests, how many retained items before and after it stayed
identical, and the input count change. An incident is classified
tool_catalog_drift_candidate when the only differing retained item is
ordinal 0 with an absolute byte delta of at most 64.

Optional schemaVersion-2 request fields (continuation, toolProvenance,
logical.toolsDetail) are reported only when present on the compared
records; their absence changes nothing. Tool names in
logical.toolsDetail are plaintext by design and are printed; every other
value from these fields remains a byte count, flag, or ordinal, never a
fingerprint/hmac.
"""

import argparse
import fnmatch
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
from typing import Any, Iterable, Iterator


DEFAULT_WARM_MINUTES = 30.0
DEFAULT_MIN_CACHED_TOKENS = 1024
DEFAULT_MIN_LOST_TOKENS = 1024
DEFAULT_DROP_FRACTION = 0.5
MAX_LINE_BYTES = 256 * 1024
SQLITE_MAX_INT = (1 << 63) - 1
TOOL_CATALOG_DRIFT_MAX_BYTES = 64
SCHEMA_VERSIONS = {1, 2}
CONTINUATION_DECISIONS = {"incremental", "full"}
CONTINUATION_DROP_REASONS = {
    "noPreviousResponse",
    "noPreviousRequest",
    "propertiesMismatch",
    "inputShorterThanPrevious",
    "inputPrefixMismatch",
    "emptyPreviousResponseId",
}

# Sentinel distinguishing "field absent" (no report change) from "field
# present but malformed" (rejects the record) for optional schemaVersion-2
# request fields.
_MISSING = object()

FIXED_COMPONENTS = (
    "instructions",
    "toolChoice",
    "parallelToolCalls",
    "store",
    "stream",
    "streamOptions",
    "include",
    "serviceTier",
    "promptCacheKey",
    "text",
    "clientMetadata",
    "accessPrograms",
)
TRANSPORT_FIELDS = (
    "endpoint",
    "compression",
    "connectionReused",
    "incremental",
)
TRANSPORT_IDENTITY_FIELDS = (
    "sessionId",
    "threadId",
    "clientRequestId",
    "subagent",
    "routingHint",
    "responsesLite",
    "previousResponseId",
    "originator",
    "userAgent",
)
REQUEST_KINDS = {"turn", "warmup", "compaction", "memory"}
COMPACTION_UNAVAILABLE_COMPONENTS = {
    "toolChoice",
    "store",
    "stream",
    "streamOptions",
    "include",
    "clientMetadata",
}


def _fingerprint(value: Any) -> str | None:
    if not isinstance(value, dict):
        return None
    hmac = value.get("hmac")
    byte_count = value.get("bytes")
    if not isinstance(hmac, str) or not isinstance(byte_count, int) or byte_count < 0:
        return None
    return json.dumps([hmac, byte_count], separators=(",", ":"))


def _is_sqlite_uint(value: Any) -> bool:
    return type(value) is int and 0 <= value <= SQLITE_MAX_INT


def _observation(value: Any) -> str | None:
    fingerprint = _fingerprint(value)
    if fingerprint is not None:
        return f"fingerprint:{fingerprint}"
    if isinstance(value, dict) and value.get("status") in {"missing", "unavailable"}:
        return f"status:{value['status']}"
    return None


def _scalar_observation(value: Any, allowed: tuple[Any, ...]) -> str | None:
    if any(type(value) is type(item) and value == item for item in allowed):
        return json.dumps(value, separators=(",", ":"))
    if isinstance(value, dict) and value.get("status") in {"missing", "unavailable"}:
        return f"status:{value['status']}"
    return None


def _transport_manifest(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    if value.get("status") in {"missing", "unavailable"}:
        return {"status": value["status"]}
    identity = value.get("identity")
    if not isinstance(identity, dict):
        return None
    fields = {
        "endpoint": _scalar_observation(
            value.get("endpoint"),
            ("responses", "guardian", "guardianClassifier", "compact"),
        ),
        "compression": _scalar_observation(value.get("compression"), ("none", "zstd")),
        "connectionReused": _scalar_observation(
            value.get("connectionReused"), (False, True)
        ),
        "incremental": _scalar_observation(value.get("incremental"), (False, True)),
        "identity": {
            name: _observation(identity.get(name)) for name in TRANSPORT_IDENTITY_FIELDS
        },
    }
    if any(fields[name] is None for name in TRANSPORT_FIELDS) or any(
        item is None for item in fields["identity"].values()
    ):
        return None
    return fields


def _list_manifest(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    status = value.get("status")
    if status in {"missing", "unavailable"}:
        return {"status": status}
    body = _fingerprint(value.get("body"))
    count = value.get("count")
    retained = value.get("retained")
    omitted = value.get("omitted")
    if (
        body is None
        or not isinstance(count, int)
        or count < 0
        or not isinstance(retained, list)
        or not isinstance(omitted, dict)
    ):
        return None
    retained_fingerprints = [_fingerprint(item) for item in retained]
    omitted_count = omitted.get("count")
    omitted_hmac = omitted.get("hmac")
    if (
        any(item is None for item in retained_fingerprints)
        or len(retained) > count
        or not isinstance(omitted_count, int)
        or omitted_count < 0
        or omitted_count != count - len(retained)
        or not isinstance(omitted_hmac, str)
    ):
        return None
    return {
        "body": body,
        "count": count,
        "retained": retained_fingerprints,
        "omitted_count": omitted_count,
        "omitted_hmac": omitted_hmac,
    }


def _logical_manifest(value: Any, request_kind: str) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    body = _fingerprint(value.get("body"))
    model = _observation(value.get("model"))
    reasoning = _observation(value.get("reasoning"))
    input_manifest = _list_manifest(value.get("input"))
    tools_manifest = _list_manifest(value.get("tools"))
    components = {
        name: _observation(
            value.get(
                name,
                {"status": "unavailable"}
                if request_kind == "compaction"
                and name in COMPACTION_UNAVAILABLE_COMPONENTS
                else None,
            )
        )
        for name in FIXED_COMPONENTS
    }
    if (
        body is None
        or model is None
        or reasoning is None
        or input_manifest is None
        or tools_manifest is None
        or any(item is None for item in components.values())
    ):
        return None
    return {
        "body": body,
        "model": model,
        "reasoning": reasoning,
        "input": input_manifest,
        "tools": tools_manifest,
        "components": components,
    }


def _wire_manifest(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    status = value.get("status")
    if status in {"missing", "unavailable"}:
        return {"status": status}
    kind = value.get("kind")
    body = _fingerprint(value.get("body"))
    transport = _transport_manifest(value.get("transport", {"status": "unavailable"}))
    if kind not in {"http", "websocket"} or body is None or transport is None:
        return None
    return {"kind": kind, "body": body, "transport": transport}


def _tool_detail_item(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    name = value.get("name")
    descriptor = _fingerprint(value.get("descriptor"))
    description = _observation(value.get("description"))
    parameters = _observation(value.get("parameters"))
    if (
        not isinstance(name, str)
        or descriptor is None
        or description is None
        or parameters is None
    ):
        return None
    return {
        "name": name,
        "descriptor": descriptor,
        "description": description,
        "parameters": parameters,
    }


def _tools_detail_manifest(logical_raw: Any) -> Any:
    """Parse the optional schemaVersion-2 `logical.toolsDetail` field.

    Returns `_MISSING` when the field is absent (never reported, never
    rejects the record), `None` when present but malformed (rejects the
    record), or the parsed manifest. Tool `name` values are plaintext by
    design and are carried through verbatim; every other value is a
    fingerprint (hmac, bytes) never printed as hmac.
    """
    if not isinstance(logical_raw, dict) or "toolsDetail" not in logical_raw:
        return _MISSING
    value = logical_raw["toolsDetail"]
    if not isinstance(value, dict):
        return None
    count = value.get("count")
    retained = value.get("retained")
    omitted = value.get("omitted")
    if (
        not isinstance(count, int)
        or count < 0
        or not isinstance(retained, list)
        or not isinstance(omitted, dict)
    ):
        return None
    retained_items = [_tool_detail_item(item) for item in retained]
    if any(item is None for item in retained_items) or len(retained_items) > count:
        return None
    omitted_count = omitted.get("count")
    omitted_hmac = omitted.get("hmac")
    if (
        not isinstance(omitted_count, int)
        or omitted_count < 0
        or omitted_count != count - len(retained_items)
        or not isinstance(omitted_hmac, str)
    ):
        return None
    return {
        "count": count,
        "retained": retained_items,
        "omitted_count": omitted_count,
        "omitted_hmac": omitted_hmac,
    }


def _continuation_manifest(value: Any) -> Any:
    """Parse the optional schemaVersion-2 `continuation` request field."""
    if value is _MISSING:
        return _MISSING
    if not isinstance(value, dict):
        return None
    transport = value.get("transport")
    decision = value.get("decision")
    if not isinstance(transport, str) or decision not in CONTINUATION_DECISIONS:
        return None
    manifest: dict[str, Any] = {"transport": transport, "decision": decision}
    if "dropReason" in value:
        drop_reason = value["dropReason"]
        if drop_reason not in CONTINUATION_DROP_REASONS:
            return None
        manifest["dropReason"] = drop_reason
    if "firstMismatchedProperty" in value:
        property_name = value["firstMismatchedProperty"]
        if not isinstance(property_name, str):
            return None
        manifest["firstMismatchedProperty"] = property_name
    if "firstMismatchedInputIndex" in value:
        index = value["firstMismatchedInputIndex"]
        if not _is_sqlite_uint(index):
            return None
        manifest["firstMismatchedInputIndex"] = index
    if "divergence" in value:
        divergence = value["divergence"]
        if not isinstance(divergence, dict):
            return None
        scope = divergence.get("scope")
        byte_offset = divergence.get("byteOffset")
        previous_bytes = divergence.get("previousBytes")
        current_bytes = divergence.get("currentBytes")
        if (
            not isinstance(scope, str)
            or not _is_sqlite_uint(byte_offset)
            or not _is_sqlite_uint(previous_bytes)
            or not _is_sqlite_uint(current_bytes)
        ):
            return None
        manifest["divergence"] = {
            "scope": scope,
            "byteOffset": byte_offset,
            "previousBytes": previous_bytes,
            "currentBytes": current_bytes,
        }
    return manifest


def _tool_provenance_manifest(value: Any) -> Any:
    """Parse the optional schemaVersion-2 `toolProvenance` request field."""
    if value is _MISSING:
        return _MISSING
    if not isinstance(value, dict):
        return None
    model_catalog = value.get("modelCatalog")
    role_failures = value.get("roleFileReadFailures")
    if not isinstance(model_catalog, dict) or not _is_sqlite_uint(role_failures):
        return None
    preset_count = model_catalog.get("presetCount")
    lock_fallback = model_catalog.get("lockContentionFallback")
    identity = _fingerprint(model_catalog.get("identity"))
    if (
        not _is_sqlite_uint(preset_count)
        or not isinstance(lock_fallback, bool)
        or identity is None
    ):
        return None
    return {
        "modelCatalog": {
            "presetCount": preset_count,
            "lockContentionFallback": lock_fallback,
            "identity": identity,
        },
        "roleFileReadFailures": role_failures,
    }


def _request_fields(record: dict[str, Any]) -> tuple[Any, ...] | None:
    run_id = record.get("runId")
    attempt_id = record.get("attemptId")
    sequence = record.get("sequence")
    timestamp = record.get("timestampUnixMs")
    lineage = record.get("lineage")
    request_kind = record.get("requestKind")
    logical_raw = record.get("logical")
    logical = _logical_manifest(logical_raw, request_kind)
    wire = _wire_manifest(record.get("wire"))
    tools_detail = _tools_detail_manifest(logical_raw)
    continuation = _continuation_manifest(record.get("continuation", _MISSING))
    tool_provenance = _tool_provenance_manifest(record.get("toolProvenance", _MISSING))
    if (
        not isinstance(run_id, str)
        or not isinstance(attempt_id, str)
        or not _is_sqlite_uint(sequence)
        or not (timestamp is None or _is_sqlite_uint(timestamp))
        or not isinstance(lineage, dict)
        or request_kind not in REQUEST_KINDS
        or logical is None
        or wire is None
        or tools_detail is None
        or continuation is None
        or tool_provenance is None
    ):
        return None
    thread = _observation(lineage.get("threadId"))
    turn = _observation(lineage.get("turnId"))
    affinity = _observation(lineage.get("affinityId"))
    retry_ordinal = record.get("retryOrdinal")
    if (
        thread is None
        or turn is None
        or affinity is None
        or not _is_sqlite_uint(retry_ordinal)
    ):
        return None
    return (
        run_id,
        attempt_id,
        sequence,
        timestamp,
        _fingerprint(record.get("keyScope")),
        request_kind,
        thread,
        turn,
        affinity,
        retry_ordinal,
        json.dumps(logical, separators=(",", ":")),
        json.dumps(wire, separators=(",", ":")),
        json.dumps(
            None if tools_detail is _MISSING else tools_detail,
            separators=(",", ":"),
        ),
        json.dumps(
            None if continuation is _MISSING else continuation,
            separators=(",", ":"),
        ),
        json.dumps(
            None if tool_provenance is _MISSING else tool_provenance,
            separators=(",", ":"),
        ),
    )


def _outcome_fields(record: dict[str, Any]) -> tuple[Any, ...] | None:
    run_id = record.get("runId")
    attempt_id = record.get("attemptId")
    sequence = record.get("sequence")
    timestamp = record.get("timestampUnixMs")
    usage = record.get("usage")
    if (
        record.get("terminal") != "completed"
        or not isinstance(run_id, str)
        or not isinstance(attempt_id, str)
        or not _is_sqlite_uint(sequence)
        or not (timestamp is None or _is_sqlite_uint(timestamp))
        or not isinstance(usage, dict)
    ):
        return None
    input_tokens = usage.get("input")
    cached_input = usage.get("cachedInput")
    if not _is_sqlite_uint(input_tokens) or not _is_sqlite_uint(cached_input):
        return None
    return (
        run_id,
        attempt_id,
        sequence,
        timestamp,
        _fingerprint(record.get("keyScope")),
        input_tokens,
        cached_input,
    )


def _valid_outcome_envelope(record: dict[str, Any]) -> bool:
    timestamp = record.get("timestampUnixMs")
    return (
        record.get("terminal") in {"completed", "failed", "cancelled", "fallback"}
        and isinstance(record.get("runId"), str)
        and isinstance(record.get("attemptId"), str)
        and _is_sqlite_uint(record.get("sequence"))
        and (timestamp is None or _is_sqlite_uint(timestamp))
    )


def _bounded_lines(stream: Any) -> Iterator[tuple[bytes | None, str | None]]:
    while True:
        chunk = stream.readline(MAX_LINE_BYTES + 1)
        if not chunk:
            return
        if len(chunk) > MAX_LINE_BYTES or not chunk.endswith(b"\n"):
            oversized = len(chunk) > MAX_LINE_BYTES
            while chunk and not chunk.endswith(b"\n"):
                chunk = stream.readline(MAX_LINE_BYTES + 1)
                oversized = oversized or len(chunk) > MAX_LINE_BYTES
            yield None, "oversized" if oversized else "malformed"
            continue
        yield chunk, None


def _run_files(roots: Iterable[str]) -> Iterator[Path]:
    for root_text in roots:
        root = Path(root_text).expanduser()
        if root.is_file():
            if fnmatch.fnmatch(root.name, "run-*.jsonl"):
                yield root
            continue
        if not root.is_dir():
            continue
        for directory, _subdirectories, filenames in os.walk(root):
            for filename in filenames:
                if fnmatch.fnmatch(filename, "run-*.jsonl"):
                    yield Path(directory, filename)


def _create_database(path: str) -> sqlite3.Connection:
    connection = sqlite3.connect(path)
    connection.row_factory = sqlite3.Row
    connection.executescript(
        """
        PRAGMA journal_mode=OFF;
        PRAGMA synchronous=OFF;
        CREATE TABLE seen_files (path TEXT PRIMARY KEY);
        CREATE TABLE requests (
          run TEXT NOT NULL, attempt TEXT NOT NULL, sequence INTEGER NOT NULL,
          timestamp INTEGER, scope TEXT, request_kind TEXT NOT NULL,
          thread TEXT NOT NULL, turn TEXT NOT NULL,
          affinity TEXT NOT NULL, retry_ordinal INTEGER NOT NULL,
          logical TEXT NOT NULL, wire TEXT NOT NULL,
          tools_detail TEXT, continuation TEXT, tool_provenance TEXT,
          PRIMARY KEY (run, attempt)
        );
        CREATE TABLE outcomes (
          run TEXT NOT NULL, attempt TEXT NOT NULL, sequence INTEGER NOT NULL,
          timestamp INTEGER, scope TEXT, input_tokens INTEGER NOT NULL,
          cached_input INTEGER NOT NULL, PRIMARY KEY (run, attempt)
        );
        """
    )
    return connection


def _empty_stats() -> dict[str, int]:
    names = (
        "files bytes records request_records outcome_records completed_attempts "
        "malformed_lines oversized_lines duplicate_records key_scope_mismatches "
        "missing_key_scope_comparisons incompatible_comparisons "
        "missing_evidence_comparisons order_ambiguous_comparisons "
        "missing_usage_outcomes untimed_records untimed_attempts "
        "eligible_warm_comparisons cache_drops incidents_found "
        "cross_kind_comparisons"
    )
    return dict.fromkeys(names.split(), 0)


def _ingest_file(
    connection: sqlite3.Connection, path: Path, stats: dict[str, int]
) -> None:
    try:
        resolved = str(path.resolve())
        inserted = connection.execute(
            "INSERT OR IGNORE INTO seen_files(path) VALUES (?)", (resolved,)
        ).rowcount
        if not inserted:
            return
        file_size = path.stat().st_size
        stream = path.open("rb")
    except OSError:
        stats["malformed_lines"] += 1
        return
    stats["files"] += 1
    stats["bytes"] += file_size
    with stream:
        for line, error in _bounded_lines(stream):
            if error is not None:
                stats[f"{error}_lines"] += 1
                continue
            try:
                record = json.loads(line)
            except (UnicodeDecodeError, ValueError):
                stats["malformed_lines"] += 1
                continue
            if (
                not isinstance(record, dict)
                or record.get("schemaVersion") not in SCHEMA_VERSIONS
            ):
                stats["malformed_lines"] += 1
                continue
            event = record.get("event")
            fields = _request_fields(record) if event == "request" else None
            table = "requests"
            if event == "outcome":
                fields = _outcome_fields(record)
                table = "outcomes"
                if fields is None and _valid_outcome_envelope(record):
                    stats["records"] += 1
                    stats["outcome_records"] += 1
                    if record.get("terminal") == "completed":
                        stats["missing_usage_outcomes"] += 1
                    if record.get("timestampUnixMs") is None:
                        stats["untimed_records"] += 1
                    continue
            if fields is None:
                stats["malformed_lines"] += 1
                continue
            stats["records"] += 1
            if record.get("timestampUnixMs") is None:
                stats["untimed_records"] += 1
            inserted = connection.execute(
                f"INSERT OR IGNORE INTO {table} VALUES ({','.join('?' for _ in fields)})",
                fields,
            ).rowcount
            if inserted:
                stats[f"{event}_records"] += 1
            else:
                stats["duplicate_records"] += 1


def _materialize_attempts(
    connection: sqlite3.Connection, stats: dict[str, int]
) -> None:
    joined = "requests r JOIN outcomes o USING (run, attempt)"
    stats["key_scope_mismatches"] = connection.execute(
        f"SELECT count(*) FROM {joined} "
        "WHERE r.scope IS NOT NULL AND o.scope IS NOT NULL AND r.scope IS NOT o.scope"
    ).fetchone()[0]
    stats["missing_key_scope_comparisons"] = connection.execute(
        f"SELECT count(*) FROM {joined} WHERE r.scope IS NULL OR o.scope IS NULL"
    ).fetchone()[0]
    stats["untimed_attempts"] = connection.execute(
        f"SELECT count(*) FROM {joined} "
        "WHERE r.scope IS o.scope AND r.scope IS NOT NULL AND r.timestamp IS NULL"
    ).fetchone()[0]
    stats["missing_evidence_comparisons"] += stats["untimed_attempts"]
    connection.executescript(
        """
        CREATE TABLE attempts AS
          SELECT r.run, r.sequence, r.timestamp, r.scope, r.request_kind, r.thread, r.turn,
                 r.affinity, r.retry_ordinal, o.input_tokens, o.cached_input,
                 r.logical, r.wire, r.tools_detail, r.continuation, r.tool_provenance
          FROM requests r JOIN outcomes o USING (run, attempt)
          WHERE r.scope IS o.scope AND r.scope IS NOT NULL AND r.timestamp IS NOT NULL;
        CREATE INDEX attempts_thread_time ON attempts(thread, timestamp);
        CREATE INDEX attempts_turn_time ON attempts(turn, timestamp);
        CREATE INDEX attempts_affinity_time ON attempts(affinity, timestamp);
        """
    )
    stats["completed_attempts"] = connection.execute(
        "SELECT count(*) FROM attempts"
    ).fetchone()[0]
    connection.commit()


def _candidate_query(lineage_column: str, require_scope: bool) -> str:
    scope = (
        """
      AND p.scope IS NOT NULL AND :scope IS NOT NULL AND p.scope = :scope
    """
        if require_scope
        else ""
    )
    return f"""
      SELECT p.* FROM attempts p WHERE p.{lineage_column} = :lineage
        AND (p.timestamp < :timestamp OR
          (p.timestamp = :timestamp AND p.run = :run AND p.sequence < :sequence))
        AND p.timestamp >= :oldest AND ({{window_predicate}}) {scope}
      ORDER BY p.timestamp DESC,
        CASE WHEN p.run = :run THEN p.sequence ELSE -1 END DESC LIMIT 1
    """


def _select_candidate(
    connection: sqlite3.Connection,
    current: sqlite3.Row,
    warm_ms: int,
    split_at: int | None,
    require_scope: bool,
) -> tuple[sqlite3.Row | None, str | None, bool]:
    parameters = {
        "timestamp": current["timestamp"],
        "run": current["run"],
        "sequence": current["sequence"],
        "scope": current["scope"],
        "oldest": current["timestamp"] - warm_ms,
        "split_at": split_at,
    }
    lineages = []
    if current["retry_ordinal"] > 0:
        lineages.append(("turn", "retry_lineage"))
    lineages.extend((("thread", "same_thread"), ("affinity", "related_thread")))
    for column, kind in lineages:
        lineage = current[column]
        if not lineage.startswith("fingerprint:"):
            continue
        parameters["lineage"] = lineage
        if split_at is None:
            window_predicate = "1 = 1"
        elif current["timestamp"] < split_at:
            window_predicate = "p.timestamp < :split_at"
        else:
            window_predicate = "p.timestamp >= :split_at"
        query = _candidate_query(column, require_scope).format(
            window_predicate=window_predicate
        )
        candidate = connection.execute(query, parameters).fetchone()
        ambiguous = (
            candidate is not None
            and require_scope
            and _has_cross_run_tie(connection, current, column, lineage)
        )
        if ambiguous:
            return None, kind, True
        if candidate is not None:
            return candidate, kind, False
    return None, None, False


def _has_cross_run_tie(
    connection: sqlite3.Connection,
    current: sqlite3.Row,
    lineage_column: str,
    lineage: str,
) -> bool:
    if current["scope"] is None:
        return False
    query = f"""
        SELECT 1 FROM attempts p
        WHERE p.{lineage_column} = :lineage
          AND p.timestamp = :timestamp
          AND p.run != :run
          AND p.scope = :scope
        LIMIT 1
    """
    match = connection.execute(
        query,
        {
            "lineage": lineage,
            "timestamp": current["timestamp"],
            "run": current["run"],
            "scope": current["scope"],
        },
    ).fetchone()
    return match is not None


def _common_prefix(first: list[str], second: list[str]) -> int:
    count = 0
    for earlier, later in zip(first, second):
        if earlier != later:
            break
        count += 1
    return count


def _list_relation(
    previous: dict[str, Any], current: dict[str, Any]
) -> tuple[str, int]:
    if "status" in previous or "status" in current:
        return ("equal", 0) if previous == current else ("rewrite", 0)
    common = _common_prefix(previous["retained"], current["retained"])
    if previous == current:
        return "equal", common
    previous_fully_retained = previous["count"] == len(previous["retained"])
    current_fully_retained = current["count"] == len(current["retained"])
    if (
        previous["count"] < current["count"]
        and previous_fully_retained
        and common == previous["count"]
    ):
        return "prior_is_prefix", common
    if (
        current["count"] < previous["count"]
        and current_fully_retained
        and common == current["count"]
    ):
        return "current_is_prefix", common
    if common == min(len(previous["retained"]), len(current["retained"])):
        return "indeterminate_after_retained", common
    return "rewrite", common


def _list_difference(
    scope: str, previous: dict[str, Any], current: dict[str, Any]
) -> dict[str, Any] | None:
    if previous == current:
        return None
    if "status" in previous or "status" in current:
        return {"scope": "component", "name": scope}
    common = _common_prefix(previous["retained"], current["retained"])
    if common < min(len(previous["retained"]), len(current["retained"])):
        return {"scope": scope, "ordinal": common}
    fully_observed = (common == previous["count"] or common == current["count"]) and (
        previous["count"] == len(previous["retained"])
        or current["count"] == len(current["retained"])
    )
    if fully_observed:
        return {"scope": scope, "ordinal": common}
    return {"scope": "component", "name": scope}


def _first_difference(
    previous: dict[str, Any], current: dict[str, Any]
) -> dict[str, Any] | None:
    previous_components = previous["components"]
    current_components = current["components"]
    if previous_components["instructions"] != current_components["instructions"]:
        return {"scope": "component", "name": "instructions"}
    difference = _list_difference("input", previous["input"], current["input"])
    if difference is not None:
        return difference
    difference = _list_difference("tool", previous["tools"], current["tools"])
    if difference is not None:
        return difference
    for name in FIXED_COMPONENTS[1:]:
        if previous_components[name] != current_components[name]:
            return {"scope": "component", "name": name}
    if previous["body"] != current["body"]:
        return {"scope": "component", "name": "logicalBody"}
    return None


def _transport_difference(
    previous_wire: dict[str, Any], current_wire: dict[str, Any]
) -> dict[str, str] | None:
    if previous_wire["kind"] != current_wire["kind"]:
        return {"scope": "transport", "name": "kind"}
    previous = previous_wire["transport"]
    current = current_wire["transport"]
    if previous == current:
        return None
    if "status" in previous or "status" in current:
        return {"scope": "transport", "name": "manifest"}
    for name in TRANSPORT_FIELDS:
        if previous[name] != current[name]:
            return {"scope": "transport", "name": name}
    for name in TRANSPORT_IDENTITY_FIELDS:
        if previous["identity"][name] != current["identity"][name]:
            return {"scope": "transport_identity", "name": name}
    return None


def _wire_observation(wire: dict[str, Any]) -> str:
    if "status" in wire:
        return "missing"
    if wire["kind"] != "websocket":
        return "http"
    transport = wire["transport"]
    if "status" in transport:
        return "websocket_transport_unavailable"
    if transport["incremental"] == "true":
        return "websocket_incremental"
    return "websocket_full"


def _fingerprint_bytes(fingerprint: str) -> int | None:
    """Recover the byte count from a stored fingerprint/observation string.

    Never returns or otherwise exposes the hmac itself. Retained input and
    tool entries (and toolsDetail `descriptor`) are encoded by
    `_fingerprint()` as a bare `[hmac,bytes]` JSON array; toolsDetail
    `description`/`parameters` are encoded by `_observation()` with a
    leading "fingerprint:" prefix (or "status:..." for missing/unavailable,
    which carries no byte count).
    """
    encoded = fingerprint
    if encoded.startswith("fingerprint:"):
        encoded = encoded[len("fingerprint:") :]
    elif encoded.startswith("status:"):
        return None
    try:
        parsed = json.loads(encoded)
    except (ValueError, TypeError):
        return None
    if not isinstance(parsed, list) or len(parsed) != 2:
        return None
    byte_count = parsed[1]
    return byte_count if isinstance(byte_count, int) else None


def _incident_granularity(
    previous_input: dict[str, Any], current_input: dict[str, Any]
) -> dict[str, Any]:
    """Compute retained-input-list granularity for one incident.

    Uses only byte sizes, ordinal positions, and counts already present in
    the v1 manifest -- never fingerprint/hmac values.
    """
    granularity: dict[str, Any] = {
        "first_differing_input_ordinal": None,
        "first_differing_input_bytes_previous": None,
        "first_differing_input_bytes_current": None,
        "identical_retained_before_first_diff": None,
        "identical_retained_after_first_diff": None,
        "input_count_change": None,
        "classification": None,
    }
    if "status" in previous_input or "status" in current_input:
        return granularity
    granularity["input_count_change"] = current_input["count"] - previous_input["count"]
    previous_retained = previous_input["retained"]
    current_retained = current_input["retained"]
    overlap = min(len(previous_retained), len(current_retained))
    diff_ordinals = [
        ordinal
        for ordinal in range(overlap)
        if previous_retained[ordinal] != current_retained[ordinal]
    ]
    if not diff_ordinals:
        return granularity
    first_ordinal = diff_ordinals[0]
    granularity["first_differing_input_ordinal"] = first_ordinal
    granularity["first_differing_input_bytes_previous"] = _fingerprint_bytes(
        previous_retained[first_ordinal]
    )
    granularity["first_differing_input_bytes_current"] = _fingerprint_bytes(
        current_retained[first_ordinal]
    )
    granularity["identical_retained_before_first_diff"] = first_ordinal
    granularity["identical_retained_after_first_diff"] = sum(
        1
        for ordinal in range(first_ordinal + 1, overlap)
        if previous_retained[ordinal] == current_retained[ordinal]
    )
    previous_bytes = granularity["first_differing_input_bytes_previous"]
    current_bytes = granularity["first_differing_input_bytes_current"]
    if (
        diff_ordinals == [0]
        and previous_bytes is not None
        and current_bytes is not None
        and abs(current_bytes - previous_bytes) <= TOOL_CATALOG_DRIFT_MAX_BYTES
    ):
        granularity["classification"] = "tool_catalog_drift_candidate"
    return granularity


def _continuation_note(continuation: dict[str, Any] | None) -> dict[str, Any] | None:
    """Report the optional schemaVersion-2 `continuation` field of the later request."""
    if continuation is None:
        return None
    note = {
        "decision": continuation["decision"],
        "dropReason": continuation.get("dropReason"),
    }
    for key in ("firstMismatchedProperty", "firstMismatchedInputIndex", "divergence"):
        if key in continuation:
            note[key] = continuation[key]
    return note


def _tool_provenance_note(
    previous_provenance: dict[str, Any] | None,
    current_provenance: dict[str, Any] | None,
) -> dict[str, Any] | None:
    """Report the optional schemaVersion-2 `toolProvenance` field of the later request."""
    if current_provenance is None:
        return None
    catalog = current_provenance["modelCatalog"]
    note: dict[str, Any] = {
        "presetCount": catalog["presetCount"],
        "lockContentionFallback": catalog["lockContentionFallback"],
        "roleFileReadFailures": current_provenance["roleFileReadFailures"],
    }
    if previous_provenance is not None:
        previous_preset = previous_provenance["modelCatalog"]["presetCount"]
        current_preset = catalog["presetCount"]
        if previous_preset != current_preset:
            note["presetCountChanged"] = {
                "previous": previous_preset,
                "current": current_preset,
            }
    return note


def _tools_detail_diff(
    previous_detail: dict[str, Any] | None, current_detail: dict[str, Any] | None
) -> dict[str, Any] | None:
    """Diff two `logical.toolsDetail` manifests by plaintext tool name.

    Tool names are plaintext by design and are printed freely; descriptor,
    description, and parameters are only ever compared for equality or
    reduced to a byte-delta, never printed as fingerprint/hmac values.
    """
    if previous_detail is None or current_detail is None:
        return None
    previous_by_name = {item["name"]: item for item in previous_detail["retained"]}
    current_by_name = {item["name"]: item for item in current_detail["retained"]}
    added = sorted(set(current_by_name) - set(previous_by_name))
    removed = sorted(set(previous_by_name) - set(current_by_name))
    changed = []
    for name in sorted(set(previous_by_name) & set(current_by_name)):
        previous_tool = previous_by_name[name]
        current_tool = current_by_name[name]
        parts = []
        for part_name in ("description", "parameters"):
            if previous_tool[part_name] != current_tool[part_name]:
                previous_bytes = _fingerprint_bytes(previous_tool[part_name])
                current_bytes = _fingerprint_bytes(current_tool[part_name])
                delta = (
                    current_bytes - previous_bytes
                    if previous_bytes is not None and current_bytes is not None
                    else None
                )
                parts.append(
                    {
                        "part": part_name,
                        "previous_bytes": previous_bytes,
                        "current_bytes": current_bytes,
                        "byte_delta": delta,
                    }
                )
        if parts or previous_tool["descriptor"] != current_tool["descriptor"]:
            changed.append({"name": name, "parts": parts})
    return {"added": added, "removed": removed, "changed": changed}


def _comparison(
    previous: sqlite3.Row, current: sqlite3.Row, kind: str
) -> tuple[dict[str, Any], bool]:
    previous_logical = json.loads(previous["logical"])
    current_logical = json.loads(current["logical"])
    previous_wire = json.loads(previous["wire"])
    current_wire = json.loads(current["wire"])
    input_relation, common_input = _list_relation(
        previous_logical["input"], current_logical["input"]
    )
    granularity = _incident_granularity(
        previous_logical["input"], current_logical["input"]
    )
    current_continuation = json.loads(current["continuation"])
    previous_tool_provenance = json.loads(previous["tool_provenance"])
    current_tool_provenance = json.loads(current["tool_provenance"])
    previous_tools_detail = json.loads(previous["tools_detail"])
    current_tools_detail = json.loads(current["tools_detail"])
    # `_analyze` only calls `_comparison` for same-requestKind pairs; cross-kind
    # pairs are reported separately (see CROSS-KIND COMPARISONS) and excluded
    # from the headline cache_drops count and rate.
    first_difference = _first_difference(previous_logical, current_logical)
    wire_missing = "status" in previous_wire or "status" in current_wire
    if not wire_missing:
        transport_missing = (
            "status" in previous_wire["transport"]
            or "status" in current_wire["transport"]
        )
    else:
        transport_missing = True
    if first_difference is None and not wire_missing:
        first_difference = _transport_difference(previous_wire, current_wire)
        if first_difference is None and previous_wire["body"] != current_wire["body"]:
            first_difference = {"scope": "component", "name": "wireBody"}
    partial = wire_missing or transport_missing
    assessment = (
        "request_difference_candidate"
        if first_difference is not None
        else "unexplained_observed_equality"
    )
    evidence_status = "partial" if partial else "complete"
    if assessment == "unexplained_observed_equality":
        evidence_status = "partial" if partial else "indeterminate"
    expected = min(previous["cached_input"], current["input_tokens"])
    observed = current["cached_input"]
    incident = {
        "timestamp_unix_ms": current["timestamp"],
        "sequence": current["sequence"],
        "previous_timestamp_unix_ms": previous["timestamp"],
        "previous_sequence": previous["sequence"],
        "elapsed_ms": current["timestamp"] - previous["timestamp"],
        "comparison_kind": kind,
        "request_kind": current["request_kind"],
        "input_tokens": current["input_tokens"],
        "cached_input_tokens": observed,
        "previous_cached_input_tokens": previous["cached_input"],
        "expected_cached_tokens": expected,
        "estimated_lost_cached_tokens": expected - observed,
        "assessment": assessment,
        "evidence_status": evidence_status,
        "logical_input_relation": input_relation,
        "common_input_prefix_count": common_input,
        "wire_observation": _wire_observation(current_wire),
        "first_difference": first_difference,
        "first_differing_input_ordinal": granularity["first_differing_input_ordinal"],
        "first_differing_input_bytes_previous": granularity[
            "first_differing_input_bytes_previous"
        ],
        "first_differing_input_bytes_current": granularity[
            "first_differing_input_bytes_current"
        ],
        "identical_retained_before_first_diff": granularity[
            "identical_retained_before_first_diff"
        ],
        "identical_retained_after_first_diff": granularity[
            "identical_retained_after_first_diff"
        ],
        "input_count_change": granularity["input_count_change"],
        "classification": granularity["classification"],
        "continuation": _continuation_note(current_continuation),
        "tool_provenance": _tool_provenance_note(
            previous_tool_provenance, current_tool_provenance
        ),
        "tools_detail_diff": _tools_detail_diff(
            previous_tools_detail, current_tools_detail
        ),
    }
    return incident, partial


def _window_name(timestamp: int, split_at: int | None) -> str:
    return (
        "all" if split_at is None else ("before" if timestamp < split_at else "after")
    )


def _empty_window(name: str) -> dict[str, Any]:
    return {
        "name": name,
        "start_unix_ms": None,
        "end_unix_ms": None,
        "eligible_warm_comparisons": 0,
        "cache_drops": 0,
        "cache_drop_rate": None,
        "expected_cached_tokens": 0,
        "observed_cached_tokens": 0,
        "cached_token_retention_rate": None,
    }


def _analyze(
    connection: sqlite3.Connection, args: argparse.Namespace, stats: dict[str, int]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    names = ("all",) if args.split_at_unix_ms is None else ("before", "after")
    windows = {name: _empty_window(name) for name in names}
    for row in connection.execute("SELECT timestamp FROM attempts ORDER BY timestamp"):
        window = windows[_window_name(row["timestamp"], args.split_at_unix_ms)]
        if window["start_unix_ms"] is None:
            window["start_unix_ms"] = row["timestamp"]
        window["end_unix_ms"] = row["timestamp"]
    incidents: list[dict[str, Any]] = []
    cross_kind_comparisons: list[dict[str, Any]] = []
    warm_ms = round(args.warm_minutes * 60_000)
    current_rows = connection.execute(
        "SELECT * FROM attempts ORDER BY timestamp, run, sequence"
    )
    for current in current_rows:
        previous, kind, ambiguous = _select_candidate(
            connection, current, warm_ms, args.split_at_unix_ms, require_scope=True
        )
        if ambiguous:
            stats["order_ambiguous_comparisons"] += 1
            continue
        if previous is None:
            continue
        if previous["request_kind"] != current["request_kind"]:
            # Cross-kind warm comparisons (e.g. warmup vs turn) are known
            # artifacts of comparing dissimilar traffic rather than genuine
            # same-conversation cache busts. They are excluded from the
            # headline cache_drops count/rate and reported separately.
            stats["cross_kind_comparisons"] += 1
            if len(cross_kind_comparisons) < args.limit:
                cross_kind_comparisons.append(
                    {
                        "timestamp_unix_ms": current["timestamp"],
                        "sequence": current["sequence"],
                        "previous_timestamp_unix_ms": previous["timestamp"],
                        "previous_sequence": previous["sequence"],
                        "elapsed_ms": current["timestamp"] - previous["timestamp"],
                        "comparison_kind": kind or "same_thread",
                        "previous_request_kind": previous["request_kind"],
                        "current_request_kind": current["request_kind"],
                        "input_tokens": current["input_tokens"],
                        "cached_input_tokens": current["cached_input"],
                        "previous_cached_input_tokens": previous["cached_input"],
                    }
                )
            continue
        previous_logical = json.loads(previous["logical"])
        current_logical = json.loads(current["logical"])
        if (
            previous_logical["model"] != current_logical["model"]
            or previous_logical["reasoning"] != current_logical["reasoning"]
        ):
            stats["incompatible_comparisons"] += 1
            continue
        if previous["cached_input"] < args.min_cached_tokens:
            continue
        expected = min(previous["cached_input"], current["input_tokens"])
        observed = current["cached_input"]
        window = windows[_window_name(current["timestamp"], args.split_at_unix_ms)]
        window["eligible_warm_comparisons"] += 1
        window["expected_cached_tokens"] += expected
        window["observed_cached_tokens"] += observed
        stats["eligible_warm_comparisons"] += 1
        incident, partial = _comparison(previous, current, kind or "same_thread")
        if partial:
            stats["missing_evidence_comparisons"] += 1
        lost = expected - observed
        is_drop = (
            lost >= args.min_lost_tokens and observed <= expected * args.drop_fraction
        )
        if not is_drop:
            continue
        window["cache_drops"] += 1
        stats["cache_drops"] += 1
        stats["incidents_found"] += 1
        if len(incidents) < args.limit:
            incidents.append(incident)
    for window in windows.values():
        eligible = window["eligible_warm_comparisons"]
        expected = window["expected_cached_tokens"]
        window["cache_drop_rate"] = (
            window["cache_drops"] / eligible if eligible else None
        )
        window["cached_token_retention_rate"] = (
            window["observed_cached_tokens"] / expected if expected else None
        )
    return incidents, list(windows.values()), cross_kind_comparisons


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "roots",
        nargs="*",
        default=[os.path.expanduser("~/.codex/cache-diagnostics")],
        help="diagnostic files or directories",
    )
    parser.add_argument("--warm-minutes", type=float, default=DEFAULT_WARM_MINUTES)
    parser.add_argument(
        "--min-cached-tokens", type=int, default=DEFAULT_MIN_CACHED_TOKENS
    )
    parser.add_argument("--min-lost-tokens", type=int, default=DEFAULT_MIN_LOST_TOKENS)
    parser.add_argument("--drop-fraction", type=float, default=DEFAULT_DROP_FRACTION)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--split-at-unix-ms", type=int)
    parser.add_argument("--json", action="store_true")
    return parser


def _validate_args(parser: argparse.ArgumentParser, args: argparse.Namespace) -> None:
    if args.warm_minutes <= 0:
        parser.error("--warm-minutes must be greater than zero")
    if args.min_cached_tokens < 0 or args.min_lost_tokens < 0:
        parser.error("token thresholds must not be negative")
    if not 0 <= args.drop_fraction <= 1:
        parser.error("--drop-fraction must be between zero and one")
    if args.limit < 0:
        parser.error("--limit must not be negative")
    if args.split_at_unix_ms is not None and args.split_at_unix_ms < 0:
        parser.error("--split-at-unix-ms must not be negative")


def _print_text(report: dict[str, Any]) -> None:
    stats = report["stats"]
    print("VERDICT")
    print(
        f"  {stats['incidents_found']:,} provider-reported cache drops were "
        "identified for evidence review."
    )
    print(
        f"  Scanned {stats['files']:,} immutable diagnostic files and "
        f"formed {stats['eligible_warm_comparisons']:,} eligible warm comparisons."
    )
    print("WINDOWS")
    for window in report["windows"]:
        if window["start_unix_ms"] is None:
            time_range = "no observed timestamps"
        else:
            time_range = f"{window['start_unix_ms']}..{window['end_unix_ms']}"
        drop_rate = window["cache_drop_rate"]
        retention = window["cached_token_retention_rate"]
        drop_rate_text = "no data" if drop_rate is None else f"{drop_rate:.6f}"
        retention_text = "no data" if retention is None else f"{retention:.6f}"
        print(
            f"  {window['name']}: range={time_range}; "
            f"eligible={window['eligible_warm_comparisons']:,}; "
            f"drops={window['cache_drops']:,}; cache_drop_rate={drop_rate_text}; "
            f"cached_tokens={window['observed_cached_tokens']:,}/"
            f"{window['expected_cached_tokens']:,}; retention={retention_text}"
        )
    print("CROSS-KIND COMPARISONS")
    cross_kind = report["cross_kind_comparisons"]
    print(
        f"  {report['stats']['cross_kind_comparisons']:,} warm comparisons had "
        "different requestKind values on the two compared requests; they are "
        "excluded from the headline cache_drops count and rate above."
    )
    for index, pair in enumerate(cross_kind, 1):
        print(
            f"  {index}. {pair['previous_request_kind']}→{pair['current_request_kind']}; "
            f"comparison_kind={pair['comparison_kind']}; "
            f"cached_input_tokens={pair['previous_cached_input_tokens']:,}→"
            f"{pair['cached_input_tokens']:,}"
        )
    for index, incident in enumerate(report["incidents"], 1):
        if incident["assessment"] == "request_difference_candidate":
            finding = "observed request-difference candidate"
        else:
            finding = "no observed request difference; cause remains indeterminate"
        print(
            f"  {index}. {finding}; lost cached tokens≈"
            f"{incident['estimated_lost_cached_tokens']:,}; "
            f"evidence={incident['evidence_status']}"
        )
        if incident["first_differing_input_ordinal"] is not None:
            print(
                "     first differing retained input: "
                f"ordinal={incident['first_differing_input_ordinal']}; "
                "bytes(previous→current)="
                f"{incident['first_differing_input_bytes_previous']}→"
                f"{incident['first_differing_input_bytes_current']}; "
                f"identical_before={incident['identical_retained_before_first_diff']}; "
                f"identical_after={incident['identical_retained_after_first_diff']}; "
                f"input_count_change={incident['input_count_change']}"
            )
            if incident["classification"]:
                print(f"     classification={incident['classification']}")
        if incident["continuation"] is not None:
            continuation = incident["continuation"]
            detail = f"decision={continuation['decision']} dropReason={continuation['dropReason']}"
            if "firstMismatchedProperty" in continuation:
                detail += f" firstMismatchedProperty={continuation['firstMismatchedProperty']}"
            if "firstMismatchedInputIndex" in continuation:
                detail += f" firstMismatchedInputIndex={continuation['firstMismatchedInputIndex']}"
            if "divergence" in continuation:
                divergence = continuation["divergence"]
                detail += (
                    f" divergence(scope={divergence['scope']}, "
                    f"byteOffset={divergence['byteOffset']}, "
                    f"previousBytes={divergence['previousBytes']}, "
                    f"currentBytes={divergence['currentBytes']})"
                )
            print(f"     continuation: {detail}")
        if incident["tool_provenance"] is not None:
            provenance = incident["tool_provenance"]
            print(
                "     toolProvenance: "
                f"presetCount={provenance['presetCount']}; "
                f"lockContentionFallback={provenance['lockContentionFallback']}; "
                f"roleFileReadFailures={provenance['roleFileReadFailures']}"
            )
            if provenance["lockContentionFallback"] is True:
                print("     WARNING: lockContentionFallback=true on current request")
            if "presetCountChanged" in provenance:
                changed = provenance["presetCountChanged"]
                print(
                    "     presetCount differs between compared requests: "
                    f"{changed['previous']}→{changed['current']}"
                )
        if incident["tools_detail_diff"] is not None:
            diff = incident["tools_detail_diff"]
            print(f"     toolsDetail: added={diff['added']} removed={diff['removed']}")
            for change in diff["changed"]:
                parts = []
                for part in change["parts"]:
                    if part["byte_delta"] is not None:
                        parts.append(
                            f"{part['part']}:{part['previous_bytes']}→"
                            f"{part['current_bytes']} (delta={part['byte_delta']:+d})"
                        )
                    else:
                        parts.append(f"{part['part']}:changed")
                descriptor_note = "" if parts else "descriptor changed"
                detail = ", ".join(parts) if parts else descriptor_note
                print(f"       changed tool={change['name']} {detail}")


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    _validate_args(parser, args)
    stats = _empty_stats()
    with tempfile.TemporaryDirectory(prefix="codex-cache-audit-") as scratch:
        connection = _create_database(str(Path(scratch, "index.sqlite3")))
        try:
            for path in _run_files(args.roots):
                _ingest_file(connection, path, stats)
            connection.commit()
            _materialize_attempts(connection, stats)
            incidents, windows, cross_kind_comparisons = _analyze(
                connection, args, stats
            )
        finally:
            connection.close()
    report = {
        "config": {
            "warm_window_minutes": args.warm_minutes,
            "min_cached_tokens": args.min_cached_tokens,
            "min_lost_tokens": args.min_lost_tokens,
            "drop_fraction": args.drop_fraction,
            "split_at_unix_ms": args.split_at_unix_ms,
        },
        "stats": stats,
        "windows": windows,
        "incidents": incidents,
        "cross_kind_comparisons": cross_kind_comparisons,
    }
    if args.json:
        json.dump(report, sys.stdout, sort_keys=True, separators=(",", ":"))
        print()
    else:
        _print_text(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
