#!/usr/bin/env python3

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path):
    with path.open(encoding="utf-8") as handle:
        return [json.loads(line) for line in handle]


def verify_checksums():
    for line in (ROOT / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
        expected, name = line.split("  ", 1)
        path = ROOT / name
        actual = sha256(path)
        if actual != expected:
            raise RuntimeError(f"checksum mismatch for {name}: {actual} != {expected}")


def aggregate(rows):
    return {
        "records": len(rows),
        "turns": len({(row["session_id"], row["turn_id"]) for row in rows}),
        "input_tokens": sum(row["input_tokens"] for row in rows),
        "cached_input_tokens": sum(row["cached_input_tokens"] for row in rows),
        "output_tokens": sum(row["output_tokens"] for row in rows),
        "total_tokens": sum(row["total_tokens"] for row in rows),
        "fresh_tokens": sum(row["fresh_tokens"] for row in rows),
        "outcomes": dict(sorted(Counter(row["outcome"] for row in rows).items())),
        "sessions": dict(sorted(Counter(row["session_id"] for row in rows).items())),
    }


def verify_polling_data():
    records = load_jsonl(ROOT / "polling-records.jsonl")
    stored = load_json(ROOT / "polling-summary.json")
    groups = {
        "empty_write_stdin": [row for row in records if row["kind"] == "empty_write_stdin"],
        "code_wait": [row for row in records if row["kind"] == "code_wait"],
        "wait_agent_all": [row for row in records if row["kind"] == "wait_agent"],
        "wait_agent_timed_out": [
            row
            for row in records
            if row["kind"] == "wait_agent" and row["outcome"] == "timed_out"
        ],
    }
    groups["wasted_total"] = (
        groups["empty_write_stdin"]
        + groups["code_wait"]
        + groups["wait_agent_timed_out"]
    )
    computed = {name: aggregate(rows) for name, rows in groups.items()}
    if computed != stored:
        raise RuntimeError("polling-summary.json does not match polling-records.jsonl")
    expected = {
        "empty_write_stdin": (127, 16_368_566, 378_550),
        "code_wait": (43, 3_803_025, 186_001),
        "wait_agent_all": (216, 27_309_194, 283_274),
        "wait_agent_timed_out": (148, 18_471_910, 217_702),
        "wasted_total": (318, 38_643_501, 782_253),
    }
    for name, values in expected.items():
        actual = (
            computed[name]["records"],
            computed[name]["total_tokens"],
            computed[name]["fresh_tokens"],
        )
        if actual != values:
            raise RuntimeError(f"published aggregate mismatch for {name}: {actual} != {values}")
    if computed["wasted_total"]["turns"] != 74:
        raise RuntimeError("published conservative parent-turn count is not 74")
    return computed


def verify_bundle_counts():
    if len(load_jsonl(ROOT / "exec-completion-notifications.jsonl")) != 76:
        raise RuntimeError("exec completion record count is not 76")
    github = load_json(ROOT / "github-snapshot.json")
    if len(github["issues"]) != 23 or len(github["pull_requests"]) != 3:
        raise RuntimeError("GitHub snapshot cardinality mismatch")
    manifest = load_json(ROOT / "source-manifest.json")
    if len(manifest["user_rollouts"]) != 7 or len(manifest["live_probe_rollouts"]) != 2:
        raise RuntimeError("source manifest cardinality mismatch")
    if len(load_jsonl(ROOT / "original-audit-command-log.jsonl")) != 56:
        raise RuntimeError("original audit command count is not 56")


def verify_external_sources():
    manifest = load_json(ROOT / "source-manifest.json")
    sources = manifest["user_rollouts"] + manifest["live_probe_rollouts"]
    for source in sources:
        path = Path(source["path"])
        if not path.is_file():
            raise RuntimeError(f"external source is missing: {path}")
        actual = sha256(path)
        if actual != source["sha256"]:
            raise RuntimeError(f"external source changed: {path}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check-external",
        action="store_true",
        help="also require the original rollout files to exist with their captured hashes",
    )
    args = parser.parse_args()
    verify_checksums()
    summary = verify_polling_data()
    verify_bundle_counts()
    if args.check_external:
        verify_external_sources()
    print(
        json.dumps(
            {
                "status": "ok",
                "external_sources_checked": args.check_external,
                "wasted_total": summary["wasted_total"],
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
