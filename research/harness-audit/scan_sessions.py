"""Read only, content free summary of a bounded Codex rollout sample.

The output has counts and hashes only. It never writes source rollouts or prints
message bodies, command text, tool arguments, tool results, or file paths.
"""

import collections
import hashlib
import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path.home() / ".codex/sessions/2026/09"
DAYS = ("22", "23")
CUTOFF = datetime.fromisoformat("2026-09-23T12:15:38+00:00").timestamp()
LOG_START = datetime.fromisoformat("2026-09-22T00:00:00+00:00").timestamp()
LOG_TARGETS = (
    "codex_core::responses_retry",
    "codex_api::endpoint::responses_websocket",
    "rmcp::transport::worker",
)


def short_hash(value):
    return hashlib.sha256(value.encode()).hexdigest()[:16]


def scan_logs():
    db = Path.home() / ".codex/logs_2.sqlite"
    connection = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    digest = hashlib.sha256()
    counts = collections.Counter()
    source_lines = collections.Counter()
    markers = collections.Counter()
    min_ts = None
    max_ts = None
    rows = connection.execute(
        "select id,ts,level,target,file,line,feedback_log_body "
        "from logs where ts between ? and ? and level in ('WARN','ERROR') "
        "and target in (?,?,?) order by id",
        (int(LOG_START), int(CUTOFF), *LOG_TARGETS),
    )
    for row_id, ts, level, target, source, line, body in rows:
        body = body or ""
        body_hash = hashlib.sha256(body.encode()).hexdigest()
        digest.update(
            json.dumps(
                [row_id, ts, level, target, source, line, body_hash],
                separators=(",", ":"),
            ).encode()
        )
        counts[f"{level}:{target}"] += 1
        source_lines[f"{source}:{line}"] += 1
        min_ts = min(min_ts, ts) if min_ts else ts
        max_ts = max(max_ts, ts) if max_ts else ts
        if target == "codex_core::responses_retry":
            lower = body.lower()
            for marker in (
                "websocket",
                "timeout",
                "closed",
                "usage",
                "incomplete",
            ):
                if marker in lower:
                    markers[marker] += 1
    connection.close()
    return {
        "start_utc": datetime.fromtimestamp(LOG_START, timezone.utc).isoformat(),
        "end_utc": datetime.fromtimestamp(CUTOFF, timezone.utc).isoformat(),
        "first_row_utc": datetime.fromtimestamp(min_ts, timezone.utc).isoformat()
        if min_ts
        else None,
        "last_row_utc": datetime.fromtimestamp(max_ts, timezone.utc).isoformat()
        if max_ts
        else None,
        "row_count": sum(counts.values()),
        "row_manifest_sha256": digest.hexdigest(),
        "level_target_counts": dict(counts),
        "source_line_counts": dict(source_lines),
        "retry_body_marker_counts": dict(markers),
    }


def main():
    candidates = sorted(p for day in DAYS for p in (ROOT / day).glob("*.jsonl"))
    files = [p for p in candidates if p.stat().st_mtime <= CUTOFF]
    event_types = collections.Counter()
    message_types = collections.Counter()
    item_types = collections.Counter()
    tool_names = collections.Counter()
    versions = collections.Counter()
    models = collections.Counter()
    anomalies = []
    manifest = hashlib.sha256()
    total_bytes = 0
    total_lines = 0
    total_calls = 0
    total_call_items = 0
    total_outputs = 0
    min_ts = None
    max_ts = None

    for path in files:
        file_digest = hashlib.sha256()
        calls = {}
        call_counts = collections.Counter()
        outputs = collections.Counter()
        starts = 0
        completes = 0
        aborted = 0
        local_events = collections.Counter()
        line_count = 0
        byte_count = 0
        file_id = short_hash(str(path.relative_to(ROOT)))
        with path.open("rb") as stream:
            for raw in stream:
                file_digest.update(raw)
                byte_count += len(raw)
                line_count += 1
                row = json.loads(raw)
                ts = row.get("timestamp")
                if ts:
                    min_ts = min(min_ts, ts) if min_ts else ts
                    max_ts = max(max_ts, ts) if max_ts else ts
                kind = row.get("type")
                event_types[kind] += 1
                payload = row.get("payload") or {}
                if kind == "session_meta":
                    versions[payload.get("cli_version", "unknown")] += 1
                elif kind == "turn_context":
                    models[payload.get("model", "unknown")] += 1
                elif kind == "event_msg":
                    subkind = payload.get("type", "unknown")
                    message_types[subkind] += 1
                    local_events[subkind] += 1
                    if subkind == "task_started":
                        starts += 1
                    elif subkind == "task_complete":
                        completes += 1
                    elif subkind == "turn_aborted":
                        aborted += 1
                    elif subkind == "item_completed":
                        item = payload.get("item") or {}
                        item_types[(item.get("type"), item.get("status"))] += 1
                elif kind == "response_item":
                    subkind = payload.get("type", "unknown")
                    if subkind in ("function_call", "custom_tool_call"):
                        calls[payload.get("call_id")] = payload.get("name")
                        call_counts[payload.get("call_id")] += 1
                        tool_names[payload.get("name", "unknown")] += 1
                    elif subkind in ("function_call_output", "custom_tool_call_output"):
                        outputs[payload.get("call_id")] += 1
        if byte_count != path.stat().st_size:
            raise RuntimeError("source changed during read: " + file_id)
        manifest.update(
            json.dumps(
                [file_id, byte_count, file_digest.hexdigest()], separators=(",", ":")
            ).encode()
        )
        total_bytes += byte_count
        total_lines += line_count
        total_calls += len(calls)
        total_call_items += sum(call_counts.values())
        total_outputs += sum(outputs.values())
        missing = sorted(short_hash(str(key)) for key in calls.keys() - outputs.keys())
        orphan = sorted(short_hash(str(key)) for key in outputs.keys() - calls.keys())
        duplicate = sorted(short_hash(str(key)) for key, n in outputs.items() if n > 1)
        duplicate_calls = sorted(
            short_hash(str(key)) for key, n in call_counts.items() if n > 1
        )
        if (
            missing
            or orphan
            or duplicate
            or duplicate_calls
            or aborted
            or local_events["tool_catalog_changed"]
        ):
            anomalies.append(
                {
                    "file_id": file_id,
                    "calls_without_output": missing,
                    "outputs_without_call": orphan,
                    "duplicate_output_ids": duplicate,
                    "duplicate_call_ids": duplicate_calls,
                    "task_started": starts,
                    "task_complete": completes,
                    "turn_aborted": aborted,
                    "tool_catalog_changed": local_events["tool_catalog_changed"],
                }
            )

    print(
        json.dumps(
            {
                "selection": {
                    "days_utc": DAYS,
                    "last_modified_at_or_before_utc": datetime.fromtimestamp(
                        CUTOFF, timezone.utc
                    ).isoformat(),
                    "candidate_files": len(candidates),
                    "selected_files": len(files),
                    "selected_bytes": total_bytes,
                    "selected_lines": total_lines,
                    "first_event_utc": min_ts,
                    "last_event_utc": max_ts,
                    "manifest_sha256": manifest.hexdigest(),
                },
                "session_versions": dict(versions),
                "turn_models": dict(models),
                "record_types": dict(event_types),
                "event_types": dict(message_types),
                "completed_item_types": {
                    f"{type_name}:{status}": n
                    for (type_name, status), n in sorted(item_types.items())
                },
                "tool_names": dict(tool_names),
                "tool_call_ids": total_calls,
                "tool_call_items": total_call_items,
                "tool_output_items": total_outputs,
                "anomalies": anomalies,
                "runtime_log_slice": scan_logs(),
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
