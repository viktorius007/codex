#!/usr/bin/env python3

import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import audit_prompt_cache


class PromptCacheAuditTest(unittest.TestCase):
    def test_warm_drop_is_reported_without_rollout_content(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            rollout = Path(temp_dir) / "rollout-fixture.jsonl"
            secret = "PRIVATE-PROMPT-CONTENT"
            records = [
                {
                    "timestamp": "2026-09-13T00:00:00Z",
                    "type": "session_meta",
                    "payload": {
                        "id": "thread-1",
                        "session_id": "session-1",
                        "source": "cli",
                        "base_instructions": secret,
                    },
                },
                {
                    "timestamp": "2026-09-13T00:00:01Z",
                    "type": "turn_context",
                    "payload": {"model": "gpt-test", "effort": "medium"},
                },
                self.usage("2026-09-13T00:00:10Z", "response-1", 20_000, 16_384),
                {
                    "timestamp": "2026-09-13T00:01:00Z",
                    "type": "world_state",
                    "payload": {"full": False, "state": {"skills": secret}},
                },
                self.usage("2026-09-13T00:01:10Z", "response-2", 22_000, 0),
            ]
            rollout.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            args = audit_prompt_cache.build_parser().parse_args([temp_dir])
            incidents, stats = audit_prompt_cache.scan(args)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                audit_prompt_cache.print_text(incidents, stats, args)

            self.assertEqual(len(incidents), 1)
            self.assertEqual(incidents[0].cause, "world_state_changed")
            self.assertEqual(incidents[0].estimated_lost_cached_tokens, 16_384)
            self.assertNotIn(secret, output.getvalue())

            records[-1] = self.usage(
                "2026-09-13T00:01:10Z", "response-2", 22_000, 16_384
            )
            rollout.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            incidents, _ = audit_prompt_cache.scan(args)
            self.assertEqual(incidents, [])

    def test_inherited_history_marker_keeps_forked_subagent_out_of_fresh_bucket(
        self,
    ) -> None:
        source = "subagent:thread_spawn"

        self.assertEqual(audit_prompt_cache.history_kind({}, source), "fresh")
        self.assertEqual(
            audit_prompt_cache.history_kind({"forked_from_id": "parent"}, source),
            "forked",
        )

    def test_tool_catalog_changed_is_classified_and_surfaced_without_digests(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            rollout = Path(temp_dir) / "rollout-fixture.jsonl"
            previous_digest = "PRIVATE-PREVIOUS-DIGEST"
            current_digest = "PRIVATE-CURRENT-DIGEST"
            records = [
                {
                    "timestamp": "2026-09-13T00:00:00Z",
                    "type": "session_meta",
                    "payload": {
                        "id": "thread-1",
                        "session_id": "session-1",
                        "source": "cli",
                        "base_instructions": "base",
                    },
                },
                {
                    "timestamp": "2026-09-13T00:00:01Z",
                    "type": "turn_context",
                    "payload": {"model": "gpt-test", "effort": "medium"},
                },
                self.usage("2026-09-13T00:00:10Z", "response-1", 20_000, 16_384),
                {
                    "timestamp": "2026-09-13T00:00:30Z",
                    "type": "event_msg",
                    "payload": {
                        "type": "tool_catalog_changed",
                        "previous_digest": previous_digest,
                        "current_digest": current_digest,
                        "added": ["new_tool"],
                        "removed": ["old_tool"],
                        "changed": ["shared_tool"],
                    },
                },
                self.usage("2026-09-13T00:01:00Z", "response-2", 22_000, 0),
            ]
            rollout.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            args = audit_prompt_cache.build_parser().parse_args([temp_dir])
            incidents, stats = audit_prompt_cache.scan(args)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                audit_prompt_cache.print_text(incidents, stats, args)
            rendered = output.getvalue()

            self.assertEqual(len(incidents), 1)
            self.assertEqual(incidents[0].cause, "tool_catalog_change")
            self.assertNotIn(previous_digest, rendered)
            self.assertNotIn(current_digest, rendered)
            self.assertIn("tool_catalog_changed(", rendered)
            self.assertIn("new_tool", rendered)
            self.assertIn("old_tool", rendered)
            self.assertIn("shared_tool", rendered)

    def test_tool_catalog_change_is_ranked_above_no_visible_rollout_change(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            rollout = Path(temp_dir) / "rollout-fixture.jsonl"
            records = [
                {
                    "timestamp": "2026-09-13T00:00:00Z",
                    "type": "session_meta",
                    "payload": {
                        "id": "thread-1",
                        "session_id": "session-1",
                        "source": "cli",
                        "base_instructions": "base",
                    },
                },
                self.usage("2026-09-13T00:00:10Z", "response-1", 20_000, 16_384),
                self.usage("2026-09-13T00:01:00Z", "response-2", 22_000, 0),
            ]
            rollout.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            args = audit_prompt_cache.build_parser().parse_args([temp_dir])
            incidents, _ = audit_prompt_cache.scan(args)

            self.assertEqual(len(incidents), 1)
            self.assertEqual(incidents[0].cause, "no_visible_rollout_change")

    def test_model_change_stays_intentional_even_with_tool_catalog_change(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            rollout = Path(temp_dir) / "rollout-fixture.jsonl"
            records = [
                {
                    "timestamp": "2026-09-13T00:00:00Z",
                    "type": "session_meta",
                    "payload": {
                        "id": "thread-1",
                        "session_id": "session-1",
                        "source": "cli",
                        "base_instructions": "base",
                    },
                },
                {
                    "timestamp": "2026-09-13T00:00:01Z",
                    "type": "turn_context",
                    "payload": {"model": "gpt-test-a", "effort": "medium"},
                },
                self.usage("2026-09-13T00:00:10Z", "response-1", 20_000, 16_384),
                {
                    "timestamp": "2026-09-13T00:00:20Z",
                    "type": "turn_context",
                    "payload": {"model": "gpt-test-b", "effort": "medium"},
                },
                {
                    "timestamp": "2026-09-13T00:00:30Z",
                    "type": "event_msg",
                    "payload": {
                        "type": "tool_catalog_changed",
                        "previous_digest": "digest-a",
                        "current_digest": "digest-b",
                        "added": ["new_tool"],
                        "removed": [],
                        "changed": [],
                    },
                },
                self.usage("2026-09-13T00:01:00Z", "response-2", 22_000, 0),
            ]
            rollout.write_text(
                "".join(
                    json.dumps(record, separators=(",", ":")) + "\n"
                    for record in records
                ),
                encoding="utf-8",
            )
            args = audit_prompt_cache.build_parser().parse_args([temp_dir])
            incidents, stats = audit_prompt_cache.scan(args)

            self.assertEqual(incidents, [])
            self.assertEqual(stats.ignored_intentional_changes, 1)

    @staticmethod
    def usage(timestamp: str, response_id: str, input_tokens: int, cached_tokens: int):
        return {
            "timestamp": timestamp,
            "type": "token_usage_record",
            "payload": {
                "thread_id": "thread-1",
                "session_id": "session-1",
                "response_id": response_id,
                "usage": {
                    "input_tokens": input_tokens,
                    "cached_input_tokens": cached_tokens,
                    "cache_write_input_tokens": 0,
                },
            },
        }


if __name__ == "__main__":
    unittest.main()
