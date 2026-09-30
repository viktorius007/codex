#!/usr/bin/env python3

import hashlib
import json
import os
import re
import subprocess
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
CODEX_DATA = Path(os.environ.get("CODEX_DATA_DIR", "/Users/viktor/.codex"))
WINDOW_START = "2026-09-18T14:00:00Z"
WINDOW_END = "2026-09-19T14:00:00Z"

USER_ROLLOUTS = [
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T11-53-16-01a0b75e-2a0d-7ca0-bddb-f59049ce1530.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T12-12-20-01a0b76f-9e55-7060-a4a5-74c923840d81.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T12-26-27-01a0b77c-8c00-76c1-be80-26d140fc209c.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T12-37-29-01a0b786-a3f3-7120-b40d-33196c23e7ea.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T21-37-26-01a0b974-fc5f-7c01-8999-b2dd73c0a686.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T22-56-20-01a0b9bd-35e4-7f32-a73e-5acbe0a1f658.jsonl",
    CODEX_DATA / "sessions/2026/09/19/rollout-2026-09-19T22-59-12-01a0b9bf-d50c-7bd2-a600-0960d73b43ec.jsonl",
]

POLLING_ROLLOUTS = [USER_ROLLOUTS[2], USER_ROLLOUTS[3], USER_ROLLOUTS[4]]

AUDIT_ROLLOUT = (
    CODEX_DATA
    / "sessions/2026/09/20/rollout-2026-09-20T07-43-46-01a0bba0-1713-7e03-8cc4-82c99c9dddbe.jsonl"
)

LIVE_PROBE_ROLLOUTS = [
    CODEX_DATA / "sessions/2026/09/20/rollout-2026-09-20T08-38-44-01a0bbd2-6b78-70b2-bd6a-bb57be40955b.jsonl",
    CODEX_DATA / "sessions/2026/09/20/rollout-2026-09-20T08-38-48-01a0bbd2-7c09-7161-bc1b-533139333f24.jsonl",
]

EXPECTED = {
    "empty_write_stdin": {"records": 127, "total_tokens": 16_368_566, "fresh_tokens": 378_550},
    "code_wait": {"records": 43, "total_tokens": 3_803_025, "fresh_tokens": 186_001},
    "wait_agent_timed_out": {"records": 148, "total_tokens": 18_471_910, "fresh_tokens": 217_702},
    "wait_agent_all": {"records": 216},
    "wasted_total": {"records": 318, "total_tokens": 38_643_501, "fresh_tokens": 782_253, "turns": 74},
    "exec_completion_notifications": 76,
}

EMPTY_WRITE_RE = re.compile(r"chars\s*:\s*([\"'])\1")
EXEC_COMPLETION_RE = re.compile(
    r'<exec-command-completed call-id="(?P<call_id>[^"]+)" process-id="(?P<process_id>[^"]+)" exit-code="(?P<exit_code>[^"]+)"'
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def json_lines(path: Path):
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            try:
                yield line_number, json.loads(line)
            except json.JSONDecodeError as error:
                raise RuntimeError(f"invalid JSON at {path}:{line_number}: {error}") from error


def text_value(value) -> str:
    if isinstance(value, str):
        return value
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def message_content_text(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(
            item.get("text", "") if isinstance(item, dict) else text_value(item)
            for item in content
        )
    return text_value(content)


def classify_output(kind: str, output) -> str:
    text = text_value(output)
    if kind == "wait_agent":
        if re.search(r'timed_out[^a-z]*true', text):
            return "timed_out"
        if "interrupted" in text.lower():
            return "interrupted"
        if re.search(r"completed|completed_tasks|agent_status|wait completed", text, re.I):
            return "activity"
        return "other"
    if "Script running with cell ID" in text:
        return "script_running"
    if "Script completed" in text:
        return "script_completed"
    return "other"


def record_session_meta(path: Path):
    first_timestamp = None
    last_timestamp = None
    meta = None
    custom_calls = defaultdict(int)
    function_calls = defaultdict(int)
    for _, row in json_lines(path):
        timestamp = row.get("timestamp")
        if timestamp:
            first_timestamp = first_timestamp or timestamp
            last_timestamp = timestamp
        if row.get("type") == "session_meta" and meta is None:
            meta = row.get("payload", {})
        if row.get("type") == "response_item":
            payload = row.get("payload", {})
            if payload.get("type") == "custom_tool_call":
                custom_calls[payload.get("name", "<missing>")] += 1
            elif payload.get("type") == "function_call":
                function_calls[payload.get("name", "<missing>")] += 1
    stat = path.stat()
    return {
        "path": str(path),
        "sha256": sha256(path),
        "size_bytes": stat.st_size,
        "mtime_ns": stat.st_mtime_ns,
        "first_record_timestamp": first_timestamp,
        "last_record_timestamp": last_timestamp,
        "session_id": (meta or {}).get("session_id"),
        "thread_id": (meta or {}).get("id"),
        "session_started_at": (meta or {}).get("timestamp"),
        "thread_source": (meta or {}).get("thread_source"),
        "cwd": (meta or {}).get("cwd"),
        "cli_version": (meta or {}).get("cli_version"),
        "custom_tool_calls": dict(sorted(custom_calls.items())),
        "function_calls": dict(sorted(function_calls.items())),
    }


def extract_polling_records(path: Path):
    meta = record_session_meta(path)
    session_id = meta["session_id"]
    current_turn = None
    calls = {}
    ready = []
    records = []

    for line_number, row in json_lines(path):
        timestamp = row.get("timestamp", "")
        if not (WINDOW_START <= timestamp < WINDOW_END):
            continue
        row_type = row.get("type")
        payload = row.get("payload", {})

        if row_type == "event_msg" and payload.get("type") == "task_started":
            current_turn = payload.get("turn_id")
            continue

        if row_type == "response_item":
            payload_type = payload.get("type")
            call_id = payload.get("call_id")
            kind = None
            arguments = None

            if payload_type == "custom_tool_call" and payload.get("name") == "exec":
                tool_input = payload.get("input", "")
                if "tools.write_stdin" in tool_input and EMPTY_WRITE_RE.search(tool_input):
                    kind = "empty_write_stdin"
                    arguments = {"classification": "tools.write_stdin with empty chars"}
            elif payload_type == "function_call" and payload.get("name") == "wait":
                kind = "code_wait"
                arguments = payload.get("arguments")
            elif payload_type == "function_call" and payload.get("name") == "wait_agent":
                kind = "wait_agent"
                arguments = payload.get("arguments")

            if kind and call_id:
                calls[call_id] = {
                    "source_path": str(path),
                    "source_sha256": meta["sha256"],
                    "source_line": line_number,
                    "session_id": session_id,
                    "turn_id": current_turn,
                    "kind": kind,
                    "call_id": call_id,
                    "call_timestamp": timestamp,
                    "arguments": arguments,
                }
                continue

            if payload_type in {"custom_tool_call_output", "function_call_output"} and call_id in calls:
                call = calls.pop(call_id)
                call["output_line"] = line_number
                call["output_timestamp"] = timestamp
                call["outcome"] = classify_output(call["kind"], payload.get("output"))
                ready.append(call)
                continue

        if (
            row_type == "event_msg"
            and payload.get("type") == "token_count"
            and payload.get("info", {}).get("last_token_usage") is not None
            and ready
        ):
            usage = payload["info"]["last_token_usage"]
            for call in ready:
                record = dict(call)
                record["usage_line"] = line_number
                record["usage_timestamp"] = timestamp
                record["input_tokens"] = usage.get("input_tokens", 0)
                record["cached_input_tokens"] = usage.get("cached_input_tokens", 0)
                record["output_tokens"] = usage.get("output_tokens", 0)
                record["reasoning_output_tokens"] = usage.get("reasoning_output_tokens", 0)
                record["total_tokens"] = usage.get("total_tokens", 0)
                record["fresh_tokens"] = (
                    record["input_tokens"]
                    - record["cached_input_tokens"]
                    + record["output_tokens"]
                )
                record["usage_attribution_rule"] = "first token_count.last_token_usage after matching tool output"
                records.append(record)
            ready = []

    if calls or ready:
        raise RuntimeError(
            f"unpaired polling records in {path}: calls={list(calls)} ready={len(ready)}"
        )
    return records


def summarize(records):
    groups = {
        "empty_write_stdin": [r for r in records if r["kind"] == "empty_write_stdin"],
        "code_wait": [r for r in records if r["kind"] == "code_wait"],
        "wait_agent_all": [r for r in records if r["kind"] == "wait_agent"],
        "wait_agent_timed_out": [
            r for r in records if r["kind"] == "wait_agent" and r["outcome"] == "timed_out"
        ],
    }
    groups["wasted_total"] = (
        groups["empty_write_stdin"]
        + groups["code_wait"]
        + groups["wait_agent_timed_out"]
    )

    summary = {}
    for name, rows in groups.items():
        summary[name] = {
            "records": len(rows),
            "turns": len({(r["session_id"], r["turn_id"]) for r in rows}),
            "input_tokens": sum(r["input_tokens"] for r in rows),
            "cached_input_tokens": sum(r["cached_input_tokens"] for r in rows),
            "output_tokens": sum(r["output_tokens"] for r in rows),
            "total_tokens": sum(r["total_tokens"] for r in rows),
            "fresh_tokens": sum(r["fresh_tokens"] for r in rows),
            "outcomes": dict(
                sorted(
                    (outcome, sum(r["outcome"] == outcome for r in rows))
                    for outcome in {r["outcome"] for r in rows}
                )
            ),
            "sessions": dict(
                sorted(
                    (session_id, sum(r["session_id"] == session_id for r in rows))
                    for session_id in {r["session_id"] for r in rows}
                )
            ),
        }

    for name, expected in EXPECTED.items():
        if name == "exec_completion_notifications":
            continue
        for field, value in expected.items():
            actual = summary[name][field]
            if actual != value:
                raise RuntimeError(f"{name}.{field}: expected {value}, got {actual}")
    return summary


def extract_completion_notifications(paths):
    records = []
    for path in paths:
        meta = record_session_meta(path)
        for line_number, row in json_lines(path):
            timestamp = row.get("timestamp", "")
            if not (WINDOW_START <= timestamp < WINDOW_END):
                continue
            if row.get("type") != "response_item":
                continue
            payload = row.get("payload", {})
            if payload.get("type") != "message":
                continue
            text = message_content_text(payload.get("content", ""))
            for match in EXEC_COMPLETION_RE.finditer(text):
                records.append(
                    {
                        "source_path": str(path),
                        "source_sha256": meta["sha256"],
                        "source_line": line_number,
                        "session_id": meta["session_id"],
                        "timestamp": timestamp,
                        **match.groupdict(),
                    }
                )
    if len(records) != EXPECTED["exec_completion_notifications"]:
        raise RuntimeError(
            f"exec completion notifications: expected {EXPECTED['exec_completion_notifications']}, got {len(records)}"
        )
    return records


def extract_original_audit_log(path: Path):
    calls = {}
    outputs = {}
    for line_number, row in json_lines(path):
        timestamp = row.get("timestamp", "")
        if not ("2026-09-19T21:51:00Z" <= timestamp < "2026-09-19T22:21:00Z"):
            continue
        if row.get("type") != "response_item":
            continue
        payload = row.get("payload", {})
        if payload.get("type") == "custom_tool_call" and payload.get("name") == "exec":
            calls[payload.get("call_id")] = {
                "timestamp": timestamp,
                "source_line": line_number,
                "call_id": payload.get("call_id"),
                "name": payload.get("name"),
                "input": payload.get("input"),
            }
        elif payload.get("type") == "custom_tool_call_output":
            outputs[payload.get("call_id")] = {
                "output_timestamp": timestamp,
                "output_source_line": line_number,
                "output": payload.get("output"),
            }
    records = []
    for call_id, call in calls.items():
        record = dict(call)
        record.update(outputs.get(call_id, {"output": None}))
        records.append(record)
    return sorted(records, key=lambda record: record["timestamp"])


def git(*args):
    return subprocess.check_output(["git", *args], cwd=REPO, text=True).strip()


def source_snapshot():
    ranges = {
        "codex-rs/features/src/feature_configs.rs": [(280, 293)],
        "codex-rs/core/src/config/mod.rs": [(1290, 1320), (2745, 2790)],
        "codex-rs/core/src/tools/spec_plan.rs": [(685, 705), (1300, 1332)],
        "codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs": [(1, 180)],
        "codex-rs/core/src/session/multi_agents.rs": [(45, 55), (116, 135)],
        "codex-rs/core/src/session/mod.rs": [(2230, 2400)],
        "codex-rs/core/src/agent/control/spawn.rs": [(780, 805)],
        "codex-rs/core/src/tools/handlers/multi_agents_v2/send_message.rs": [(1, 70)],
        "codex-rs/core/src/tools/handlers/multi_agents_v2/followup_task.rs": [(1, 70)],
        "codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs": [(1, 145)],
        "codex-rs/core/src/session/input_queue.rs": [(101, 190)],
        "codex-rs/core/tests/suite/subagent_notifications.rs": [(2330, 2490)],
        "codex-rs/core/tests/suite/unified_exec.rs": [(1215, 1295)],
    }
    files = []
    for relative, selected_ranges in ranges.items():
        path = REPO / relative
        lines = path.read_text(encoding="utf-8").splitlines()
        excerpts = []
        for start, end in selected_ranges:
            excerpts.append(
                {
                    "start_line": start,
                    "end_line": min(end, len(lines)),
                    "text": "\n".join(
                        f"{number}: {lines[number - 1]}"
                        for number in range(start, min(end, len(lines)) + 1)
                    ),
                }
            )
        files.append(
            {
                "path": relative,
                "sha256": sha256(path),
                "git_blob": git("hash-object", relative),
                "excerpts": excerpts,
            }
        )
    commits = []
    for commit in [
        "3e3b41a7a2712420b149088905eff86eb44c67a1",
        "8c5e8401e8f24f366c882b1864d0ef41e45ac9f9",
        "19ccaa5827a7a3f7dbadcca415b836d69d7d4018",
        "4462b9deef211723b781b426f5e5d36a5777115f",
        "8a1c9414399bd8c52cd11d89f420767c41f4ae99",
        "4d7e3e90d9781514f9b3b99c95169a2386868ad9",
        "ffde91b0c52241cf0b746fe3524c52b82ddd099a",
    ]:
        fields = git("show", "-s", "--format=%H%x00%P%x00%aI%x00%cI%x00%s", commit).split("\x00")
        commits.append(
            {
                "commit": fields[0],
                "parents": fields[1].split(),
                "authored_at": fields[2],
                "committed_at": fields[3],
                "subject": fields[4],
            }
        )
    return {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "head": git("rev-parse", "HEAD"),
        "branch": git("branch", "--show-current"),
        "status_porcelain": git("status", "--short"),
        "files": files,
        "commits": commits,
        "ancestry": {
            "exec_patch_in_incident_source": subprocess.call(
                ["git", "merge-base", "--is-ancestor", "3e3b41a7a2", "ffde91b0c522"],
                cwd=REPO,
            )
            == 0,
            "agent_patch_in_incident_source": subprocess.call(
                ["git", "merge-base", "--is-ancestor", "8c5e8401e8", "ffde91b0c522"],
                cwd=REPO,
            )
            == 0,
        },
    }


def relevant_config_snapshot():
    path = CODEX_DATA / "config.toml"
    import tomllib

    parsed = tomllib.loads(path.read_text(encoding="utf-8"))
    value = parsed.get("features", {}).get("multi_agent_v2", {})
    stat = path.stat()
    return {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "path": str(path),
        "sha256": sha256(path),
        "size_bytes": stat.st_size,
        "mtime_ns": stat.st_mtime_ns,
        "selected_value": {"features": {"multi_agent_v2": value}},
        "redaction": "Only features.multi_agent_v2 is stored; the full-file SHA-256 binds this selection to the original file without copying unrelated configuration.",
    }


def write_json(path: Path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, records):
    with path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, sort_keys=True) + "\n")


def main():
    for path in USER_ROLLOUTS + LIVE_PROBE_ROLLOUTS + [AUDIT_ROLLOUT]:
        if not path.is_file():
            raise RuntimeError(f"missing source: {path}")

    polling_records = []
    for path in POLLING_ROLLOUTS:
        polling_records.extend(extract_polling_records(path))
    polling_records.sort(key=lambda record: (record["call_timestamp"], record["call_id"]))
    polling_summary = summarize(polling_records)
    completion_records = extract_completion_notifications(POLLING_ROLLOUTS)

    source_manifest = {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "window": {"start_utc_inclusive": WINDOW_START, "end_utc_exclusive": WINDOW_END},
        "user_rollouts": [record_session_meta(path) for path in USER_ROLLOUTS],
        "polling_rollouts": [str(path) for path in POLLING_ROLLOUTS],
        "live_probe_rollouts": [record_session_meta(path) for path in LIVE_PROBE_ROLLOUTS],
        "audit_rollout": str(AUDIT_ROLLOUT),
    }

    write_jsonl(OUT / "polling-records.jsonl", polling_records)
    write_json(OUT / "polling-summary.json", polling_summary)
    write_jsonl(OUT / "exec-completion-notifications.jsonl", completion_records)
    write_json(OUT / "source-manifest.json", source_manifest)
    write_jsonl(OUT / "original-audit-command-log.jsonl", extract_original_audit_log(AUDIT_ROLLOUT))
    write_json(OUT / "source-snapshot.json", source_snapshot())
    write_json(OUT / "config-snapshot.json", relevant_config_snapshot())

    generated = [
        "polling-records.jsonl",
        "polling-summary.json",
        "exec-completion-notifications.jsonl",
        "source-manifest.json",
        "original-audit-command-log.jsonl",
        "source-snapshot.json",
        "config-snapshot.json",
    ]
    with (OUT / "SHA256SUMS").open("w", encoding="utf-8") as handle:
        for name in generated:
            handle.write(f"{sha256(OUT / name)}  {name}\n")

    print(json.dumps({"status": "ok", "summary": polling_summary}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
