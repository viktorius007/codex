#!/usr/bin/env python3
"""Find likely prompt-cache busts in local Codex rollout files.

The scanner reads rollout JSONL locally and never prints prompts, model output,
tool arguments, tool output, working directories, or world-state values.

An EventMsg line of type `tool_catalog_changed` (payload fields
`previous_digest`, `current_digest`, `added`, `removed`, `changed`) is
surfaced in a drop incident's between-samples diagnostics as
`tool_catalog_changed(added=...,removed=...,changed=...)`, using the
plaintext tool-name lists only -- the digests themselves are never printed.
A drop whose between-samples window contains such a line is classified
with cause `tool_catalog_change`, ranked above the `no_visible_rollout_change`
fallback; model/effort changes are still treated as intentional and ignored
before classification runs.
"""

import argparse
import collections
import datetime as dt
import hashlib
import json
import os
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterable


DEFAULT_WARM_MINUTES = 30
DEFAULT_MIN_CACHED_TOKENS = 1024
DEFAULT_MIN_LOST_TOKENS = 1024
DEFAULT_DROP_FRACTION = 0.5
RELEVANT_LINE_MARKERS = (
    '"type":"session_meta"',
    '"type": "session_meta"',
    '"type":"turn_context"',
    '"type": "turn_context"',
    '"type":"world_state"',
    '"type": "world_state"',
    '"type":"compacted"',
    '"type": "compacted"',
    '"type":"token_usage_record"',
    '"type": "token_usage_record"',
    '"type":"token_count"',
    '"type": "token_count"',
    '"type":"thread_settings_applied"',
    '"type": "thread_settings_applied"',
    '"type":"custom_tool_call"',
    '"type": "custom_tool_call"',
    '"type":"function_call"',
    '"type": "function_call"',
    '"type":"tool_catalog_changed"',
    '"type": "tool_catalog_changed"',
)
CONTEXT_ID_FIELDS = {"turn_id", "root_turn_id"}
INTENTIONAL_CONTEXT_FIELDS = {
    "model",
    "effort",
    "thread_settings.model",
    "thread_settings.model_provider_id",
    "thread_settings.reasoning_effort",
}


@dataclass(frozen=True)
class UsageSample:
    timestamp: float
    timestamp_text: str
    path: str
    ordinal: int | None
    session_id: str
    thread_id: str
    response_id: str | None
    input_tokens: int
    cached_tokens: int
    cache_write_tokens: int
    model: str | None
    effort: str | None
    source_kind: str
    history_kind: str
    context_changes: tuple[str, ...]
    world_changes: tuple[str, ...]
    activity: tuple[str, ...]
    startup_state: tuple[tuple[str, str], ...]
    compaction_request: bool = False


@dataclass(frozen=True)
class Incident:
    kind: str
    cause: str
    timestamp: str
    seconds_since_previous: float
    path: str
    previous_path: str
    ordinal: int | None
    previous_ordinal: int | None
    session_id: str
    thread_id: str
    previous_thread_id: str
    input_tokens: int
    cached_tokens: int
    previous_cached_tokens: int
    expected_cached_floor: int
    estimated_lost_cached_tokens: int
    cache_write_tokens: int
    source_kind: str
    context_changes: tuple[str, ...]
    world_changes: tuple[str, ...]
    activity: tuple[str, ...]
    startup_differences: tuple[str, ...]


@dataclass
class ScanStats:
    files: int = 0
    bytes: int = 0
    malformed_lines: int = 0
    usage_samples: int = 0
    ignored_intentional_changes: int = 0
    ignored_outside_window: int = 0


@dataclass
class ParsedRollout:
    samples: list[UsageSample] = field(default_factory=list)
    malformed_lines: int = 0


@dataclass
class PendingSignals:
    context: set[str] = field(default_factory=set)
    world: set[str] = field(default_factory=set)
    activity: collections.Counter[str] = field(default_factory=collections.Counter)

    def clear(self) -> None:
        self.context.clear()
        self.world.clear()
        self.activity.clear()


@dataclass
class RolloutAccumulator:
    display_path: str
    meta: dict[str, Any] = field(default_factory=dict)
    context_hashes: dict[str, str] = field(default_factory=dict)
    settings_hashes: dict[str, str] = field(default_factory=dict)
    world_hashes: dict[str, str] = field(default_factory=dict)
    direct_pending: PendingSignals = field(default_factory=PendingSignals)
    legacy_pending: PendingSignals = field(default_factory=PendingSignals)
    direct_samples: list[UsageSample] = field(default_factory=list)
    legacy_samples: list[UsageSample] = field(default_factory=list)
    compaction_response_ids: set[str] = field(default_factory=set)

    def consume(self, record: dict[str, Any]) -> None:
        record_type = record.get("type")
        payload = record.get("payload") or {}
        if record_type == "session_meta":
            self.meta = payload
        elif record_type == "turn_context":
            self.consume_turn_context(payload)
        elif record_type == "world_state":
            self.consume_world_state(payload)
        elif record_type == "compacted":
            response_id = payload.get("compaction_response_id")
            if response_id:
                self.compaction_response_ids.add(response_id)
            self.add_activity("compacted")
        elif record_type == "response_item" and payload.get("type") in {
            "function_call",
            "custom_tool_call",
        }:
            self.add_activity(f"tool:{payload.get('name') or 'unknown'}")
        elif record_type == "event_msg":
            self.consume_event(record, payload)
        elif record_type == "token_usage_record":
            self.add_sample(record, payload.get("usage") or {}, "direct")

    def consume_turn_context(self, payload: dict[str, Any]) -> None:
        current = hash_fields(payload, CONTEXT_ID_FIELDS)
        if self.context_hashes:
            changes = changed_fields(self.context_hashes, current)
        elif self.direct_samples or self.legacy_samples:
            changes = tuple(sorted(current))
        else:
            changes = ()
        self.direct_pending.context.update(changes)
        self.legacy_pending.context.update(changes)
        self.context_hashes = current
        self.add_activity("turn_context")

    def consume_world_state(self, payload: dict[str, Any]) -> None:
        current = hash_fields(payload.get("state") or {})
        if payload.get("full"):
            if self.world_hashes:
                changes = changed_fields(self.world_hashes, current)
            elif self.direct_samples or self.legacy_samples:
                changes = tuple(sorted(current))
            else:
                changes = ()
            self.world_hashes = current
            event = "world_state_full"
        else:
            changes = tuple(sorted(current))
            self.world_hashes.update(current)
            event = "world_state_patch"
        self.direct_pending.world.update(changes)
        self.legacy_pending.world.update(changes)
        self.add_activity(event)

    def consume_event(self, record: dict[str, Any], payload: dict[str, Any]) -> None:
        event_type = payload.get("type")
        if event_type == "thread_settings_applied":
            current = hash_fields(payload.get("thread_settings") or {})
            if self.settings_hashes:
                changes = changed_fields(self.settings_hashes, current)
            elif self.direct_samples or self.legacy_samples:
                changes = tuple(sorted(current))
            else:
                changes = ()
            qualified = {f"thread_settings.{key}" for key in changes}
            self.direct_pending.context.update(qualified)
            self.legacy_pending.context.update(qualified)
            self.settings_hashes = current
            self.add_activity(event_type)
        elif event_type == "token_count":
            usage = (payload.get("info") or {}).get("last_token_usage") or {}
            if usage.get("input_tokens"):
                self.add_sample(record, usage, "legacy")
        elif event_type == "tool_catalog_changed":
            # previous_digest/current_digest are intentionally never
            # surfaced; only the plaintext tool-name lists are recorded.
            added = sorted(payload.get("added") or [])
            removed = sorted(payload.get("removed") or [])
            changed = sorted(payload.get("changed") or [])
            self.add_activity(
                f"tool_catalog_changed(added={added},removed={removed},changed={changed})"
            )

    def add_activity(self, event: str) -> None:
        self.direct_pending.activity[event] += 1
        self.legacy_pending.activity[event] += 1

    def add_sample(
        self,
        record: dict[str, Any],
        usage: dict[str, Any],
        stream: str,
    ) -> None:
        if not usage.get("input_tokens"):
            return
        if stream == "direct":
            samples = self.direct_samples
            pending = self.direct_pending
        else:
            samples = self.legacy_samples
            pending = self.legacy_pending
        samples.append(
            make_sample(
                record,
                usage,
                self.meta,
                self.context_hashes,
                self.world_hashes,
                pending.context,
                pending.world,
                pending.activity,
                self.display_path,
            )
        )
        pending.clear()

    def finish(self, malformed_lines: int) -> ParsedRollout:
        samples = self.direct_samples or deduplicate_legacy_samples(self.legacy_samples)
        if self.compaction_response_ids:
            samples = [
                UsageSample(
                    **{
                        **asdict(sample),
                        "compaction_request": (
                            sample.response_id in self.compaction_response_ids
                        ),
                    }
                )
                for sample in samples
            ]
        return ParsedRollout(samples=samples, malformed_lines=malformed_lines)


def parse_timestamp(value: str) -> float:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()


def stable_hash(value: Any) -> str:
    encoded = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()[:12]


def source_kind(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, dict) and value:
        outer = sorted(value)[0]
        inner = value[outer]
        if isinstance(inner, str):
            return f"{outer}:{inner}"
        if isinstance(inner, dict) and inner:
            return f"{outer}:{sorted(inner)[0]}"
        return outer
    return "unknown"


def history_kind(meta: dict[str, Any], source: str) -> str:
    inherited_history = (
        meta.get("forked_from_id") is not None
        or meta.get("history_base") is not None
        or meta.get("subagent_history_start_ordinal") is not None
    )
    if inherited_history:
        return "forked"
    if "subagent" in source:
        return "fresh"
    return "standalone"


def changed_fields(
    previous: dict[str, str], current: dict[str, str]
) -> tuple[str, ...]:
    return tuple(
        sorted(
            key
            for key in previous.keys() | current.keys()
            if previous.get(key) != current.get(key)
        )
    )


def hash_fields(
    payload: dict[str, Any], excluded: set[str] | None = None
) -> dict[str, str]:
    excluded = excluded or set()
    return {
        key: stable_hash(value) for key, value in payload.items() if key not in excluded
    }


def find_rollouts(roots: Iterable[Path], include_backups: bool) -> Iterable[Path]:
    seen: set[Path] = set()
    for root in roots:
        expanded = root.expanduser()
        paths = [expanded] if expanded.is_file() else expanded.rglob("rollout-*.jsonl")
        for path in paths:
            if not include_backups and "backup" in path.name:
                continue
            resolved = path.resolve()
            if resolved not in seen:
                seen.add(resolved)
                yield resolved


def usage_values(payload: dict[str, Any]) -> tuple[int, int, int]:
    return (
        int(payload.get("input_tokens") or 0),
        int(payload.get("cached_input_tokens") or 0),
        int(payload.get("cache_write_input_tokens") or 0),
    )


def parse_rollout(path: Path, display_path: str) -> ParsedRollout:
    accumulator = RolloutAccumulator(display_path)
    malformed = 0
    with path.open(encoding="utf-8", errors="replace") as rollout:
        for line in rollout:
            if not any(marker in line for marker in RELEVANT_LINE_MARKERS):
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                malformed += 1
                continue
            accumulator.consume(record)
    return accumulator.finish(malformed)


def make_sample(
    record: dict[str, Any],
    usage: dict[str, Any],
    meta: dict[str, Any],
    context_hashes: dict[str, str],
    world_hashes: dict[str, str],
    pending_context: set[str],
    pending_world: set[str],
    activity: collections.Counter[str],
    display_path: str,
) -> UsageSample:
    payload = record.get("payload") or {}
    input_tokens, cached_tokens, cache_write_tokens = usage_values(usage)
    timestamp_text = str(record.get("timestamp") or "")
    thread_id = str(payload.get("thread_id") or meta.get("id") or display_path)
    session_id = str(payload.get("session_id") or meta.get("session_id") or thread_id)
    startup = {"base_instructions": stable_hash(meta.get("base_instructions"))}
    startup.update(
        {f"turn_context.{key}": value for key, value in context_hashes.items()}
    )
    startup.update({f"world_state.{key}": value for key, value in world_hashes.items()})
    sample_source = source_kind(meta.get("source") or meta.get("thread_source"))
    return UsageSample(
        timestamp=parse_timestamp(timestamp_text),
        timestamp_text=timestamp_text,
        path=display_path,
        ordinal=record.get("ordinal"),
        session_id=session_id,
        thread_id=thread_id,
        response_id=payload.get("response_id"),
        input_tokens=input_tokens,
        cached_tokens=cached_tokens,
        cache_write_tokens=cache_write_tokens,
        model=meta_value(meta, context_hashes, "model"),
        effort=meta_value(meta, context_hashes, "effort"),
        source_kind=sample_source,
        history_kind=history_kind(meta, sample_source),
        context_changes=tuple(sorted(pending_context)),
        world_changes=tuple(sorted(pending_world)),
        activity=tuple(
            f"{name}x{count}" if count > 1 else name
            for name, count in sorted(activity.items())
        ),
        startup_state=tuple(sorted(startup.items())),
    )


def meta_value(meta: dict[str, Any], hashes: dict[str, str], key: str) -> str | None:
    value = meta.get(key)
    if value is not None:
        return str(value)
    return hashes.get(key)


def deduplicate_legacy_samples(samples: list[UsageSample]) -> list[UsageSample]:
    result: list[UsageSample] = []
    previous_signature: tuple[int, int, int] | None = None
    for sample in samples:
        signature = (
            sample.input_tokens,
            sample.cached_tokens,
            sample.cache_write_tokens,
        )
        if signature != previous_signature:
            result.append(sample)
        previous_signature = signature
    return result


def startup_differences(previous: UsageSample, current: UsageSample) -> tuple[str, ...]:
    previous_state = dict(previous.startup_state)
    current_state = dict(current.startup_state)
    return changed_fields(previous_state, current_state)


def has_intentional_change(previous: UsageSample, current: UsageSample) -> bool:
    if previous.model != current.model or previous.effort != current.effort:
        return True
    return bool(INTENTIONAL_CONTEXT_FIELDS.intersection(current.context_changes))


def classify_cause(
    previous: UsageSample,
    current: UsageSample,
    kind: str,
    startup_diff: tuple[str, ...],
) -> str:
    if current.compaction_request:
        return "local_compaction_request"
    if any(item.startswith("compacted") for item in current.activity):
        return "post_compaction"
    if kind == "cross_thread_start" and current.history_kind == "forked":
        return "forked_subagent_start"
    if kind == "cross_thread_start" and current.history_kind == "fresh":
        return "fresh_subagent_start"
    if kind == "cross_thread_start":
        return "new_related_thread"
    if any(item.startswith("thread_settings.") for item in current.context_changes):
        return "thread_settings_changed"
    if current.context_changes:
        return "turn_context_changed"
    if current.world_changes:
        return "world_state_changed"
    if startup_diff:
        return "startup_prefix_changed"
    if any(item.startswith("tool_catalog_changed(") for item in current.activity):
        return "tool_catalog_change"
    return "no_visible_rollout_change"


def compare_samples(
    previous: UsageSample,
    current: UsageSample,
    kind: str,
    warm_seconds: float,
    min_cached_tokens: int,
    min_lost_tokens: int,
    drop_fraction: float,
    stats: ScanStats,
) -> Incident | None:
    elapsed = current.timestamp - previous.timestamp
    if elapsed < 0 or elapsed > warm_seconds:
        stats.ignored_outside_window += 1
        return None
    expected = min(previous.cached_tokens, current.input_tokens)
    lost = expected - current.cached_tokens
    if (
        previous.cached_tokens < min_cached_tokens
        or lost < min_lost_tokens
        or current.cached_tokens > expected * drop_fraction
    ):
        return None
    if has_intentional_change(previous, current):
        stats.ignored_intentional_changes += 1
        return None
    startup_diff = (
        startup_differences(previous, current) if previous.path != current.path else ()
    )
    return Incident(
        kind=kind,
        cause=classify_cause(previous, current, kind, startup_diff),
        timestamp=current.timestamp_text,
        seconds_since_previous=round(elapsed, 3),
        path=current.path,
        previous_path=previous.path,
        ordinal=current.ordinal,
        previous_ordinal=previous.ordinal,
        session_id=current.session_id,
        thread_id=current.thread_id,
        previous_thread_id=previous.thread_id,
        input_tokens=current.input_tokens,
        cached_tokens=current.cached_tokens,
        previous_cached_tokens=previous.cached_tokens,
        expected_cached_floor=expected,
        estimated_lost_cached_tokens=lost,
        cache_write_tokens=current.cache_write_tokens,
        source_kind=current.source_kind,
        context_changes=current.context_changes,
        world_changes=current.world_changes,
        activity=current.activity,
        startup_differences=startup_diff,
    )


def find_incidents(
    samples: list[UsageSample],
    warm_minutes: float,
    min_cached_tokens: int,
    min_lost_tokens: int,
    drop_fraction: float,
    stats: ScanStats,
) -> list[Incident]:
    warm_seconds = warm_minutes * 60
    incidents: list[Incident] = []
    by_thread: dict[str, list[UsageSample]] = collections.defaultdict(list)
    by_session: dict[str, list[UsageSample]] = collections.defaultdict(list)
    for sample in samples:
        by_thread[sample.thread_id].append(sample)
        by_session[sample.session_id].append(sample)

    compared_pairs: set[tuple[str, float, str, float]] = set()
    for thread_samples in by_thread.values():
        ordered = sorted(thread_samples, key=lambda sample: sample.timestamp)
        for previous, current in zip(ordered, ordered[1:]):
            incident = compare_samples(
                previous,
                current,
                "same_thread",
                warm_seconds,
                min_cached_tokens,
                min_lost_tokens,
                drop_fraction,
                stats,
            )
            if incident:
                incidents.append(incident)
            compared_pairs.add(
                (
                    previous.thread_id,
                    previous.timestamp,
                    current.thread_id,
                    current.timestamp,
                )
            )

    for session_samples in by_session.values():
        ordered = sorted(session_samples, key=lambda sample: sample.timestamp)
        latest = None
        seen_threads: set[str] = set()
        for current in ordered:
            if current.thread_id not in seen_threads and latest is not None:
                pair = (
                    latest.thread_id,
                    latest.timestamp,
                    current.thread_id,
                    current.timestamp,
                )
                if pair not in compared_pairs:
                    incident = compare_samples(
                        latest,
                        current,
                        "cross_thread_start",
                        warm_seconds,
                        min_cached_tokens,
                        min_lost_tokens,
                        drop_fraction,
                        stats,
                    )
                    if incident:
                        incidents.append(incident)
            seen_threads.add(current.thread_id)
            latest = current
    return sorted(
        incidents, key=lambda incident: -incident.estimated_lost_cached_tokens
    )


def display_path(path: Path, roots: list[Path]) -> str:
    for root in roots:
        expanded = root.expanduser().resolve()
        if expanded.is_dir():
            try:
                return str(path.relative_to(expanded))
            except ValueError:
                continue
    return path.name


def scan(args: argparse.Namespace) -> tuple[list[Incident], ScanStats]:
    roots = [Path(root) for root in args.roots]
    stats = ScanStats()
    samples: list[UsageSample] = []
    for path in find_rollouts(roots, args.include_backups):
        stats.files += 1
        stats.bytes += path.stat().st_size
        parsed = parse_rollout(path, display_path(path, roots))
        stats.malformed_lines += parsed.malformed_lines
        samples.extend(parsed.samples)
    stats.usage_samples = len(samples)
    incidents = find_incidents(
        samples,
        args.warm_minutes,
        args.min_cached_tokens,
        args.min_lost_tokens,
        args.drop_fraction,
        stats,
    )
    return incidents, stats


def print_text(
    incidents: list[Incident], stats: ScanStats, args: argparse.Namespace
) -> None:
    print("VERDICT")
    print(
        f"  {len(incidents)} likely unintended cache drops within "
        f"{args.warm_minutes:g} minutes."
    )
    print(
        f"  Scanned {stats.files:,} rollouts ({stats.bytes / 1024**3:.2f} GiB) and "
        f"{stats.usage_samples:,} model-usage samples."
    )
    print(
        f"  Ignored {stats.ignored_intentional_changes:,} model/effort changes and "
        f"{stats.ignored_outside_window:,} comparisons outside the warm window."
    )
    print(
        "  'No visible rollout change' means the rollout lacks the decisive request/tool "
        "snapshot; it does not prove a backend fault."
    )
    if stats.malformed_lines:
        print(f"  Skipped {stats.malformed_lines:,} malformed relevant JSONL lines.")
    if not incidents:
        return

    print()
    print("LIKELY CAUSES")
    for cause, count in collections.Counter(
        item.cause for item in incidents
    ).most_common():
        lost = sum(
            item.estimated_lost_cached_tokens
            for item in incidents
            if item.cause == cause
        )
        print(f"  {count:5,}  {lost:12,} estimated lost cached tokens  {cause}")

    if args.limit == 0:
        return
    print()
    print(f"TOP INCIDENTS (up to {args.limit})")
    for index, incident in enumerate(incidents[: args.limit], 1):
        print(
            f"  {index}. {incident.timestamp}  {incident.cause}  "
            f"lost≈{incident.estimated_lost_cached_tokens:,}  "
            f"cached {incident.previous_cached_tokens:,}→{incident.cached_tokens:,}  "
            f"input={incident.input_tokens:,}  gap={incident.seconds_since_previous:g}s"
        )
        print(
            f"     {incident.kind}; source={incident.source_kind}; file={incident.path}; "
            f"ordinal={incident.ordinal}; previous_file={incident.previous_path}; "
            f"previous_ordinal={incident.previous_ordinal}"
        )
        diagnostic_groups = (
            ("turn_context=", incident.context_changes),
            ("world_state=", incident.world_changes),
            ("startup=", incident.startup_differences),
            ("between=", incident.activity),
        )
        details = [
            prefix + ",".join(values) for prefix, values in diagnostic_groups if values
        ]
        details.extend(
            [f"cache_write_tokens={incident.cache_write_tokens:,}"]
            if incident.cache_write_tokens
            else []
        )
        print(
            "     diagnostics: " + ("; ".join(details) if details else "none recorded")
        )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "roots",
        nargs="*",
        default=[os.path.expanduser("~/.codex/sessions")],
        help="rollout files or directories (default: ~/.codex/sessions)",
    )
    parser.add_argument(
        "--warm-minutes",
        type=float,
        default=DEFAULT_WARM_MINUTES,
        help="maximum gap treated as warm (default: 30)",
    )
    parser.add_argument(
        "--min-cached-tokens", type=int, default=DEFAULT_MIN_CACHED_TOKENS
    )
    parser.add_argument("--min-lost-tokens", type=int, default=DEFAULT_MIN_LOST_TOKENS)
    parser.add_argument(
        "--drop-fraction",
        type=float,
        default=DEFAULT_DROP_FRACTION,
        help="flag when cached tokens fall to this fraction of the expected floor (default: 0.5)",
    )
    parser.add_argument(
        "--limit", type=int, default=50, help="maximum incidents to print"
    )
    parser.add_argument("--include-backups", action="store_true")
    parser.add_argument(
        "--json", action="store_true", help="print machine-readable JSON"
    )
    return parser


def validate_args(parser: argparse.ArgumentParser, args: argparse.Namespace) -> None:
    if args.warm_minutes <= 0:
        parser.error("--warm-minutes must be greater than zero")
    if args.min_cached_tokens < 0 or args.min_lost_tokens < 0:
        parser.error("token thresholds must not be negative")
    if not 0 <= args.drop_fraction <= 1:
        parser.error("--drop-fraction must be between zero and one")
    if args.limit < 0:
        parser.error("--limit must not be negative")


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    validate_args(parser, args)
    incidents, stats = scan(args)
    if args.json:
        json.dump(
            {
                "warm_window_minutes": args.warm_minutes,
                "stats": asdict(stats),
                "incidents": [asdict(incident) for incident in incidents[: args.limit]],
            },
            sys.stdout,
            indent=2,
        )
        print()
    else:
        print_text(incidents, stats, args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
