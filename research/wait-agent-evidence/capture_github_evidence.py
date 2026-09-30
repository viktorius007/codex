#!/usr/bin/env python3

import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path


REPO_NAME = "openai/codex"
OUT = Path(__file__).resolve().parent / "github-snapshot.json"
ISSUES = [
    13733,
    15723,
    18394,
    20312,
    28144,
    29122,
    29922,
    32188,
    32640,
    34468,
    34866,
    35108,
    35259,
    37238,
    37299,
    38093,
    38495,
    39854,
    41875,
    42074,
    45974,
    46085,
    46120,
]
PULLS = [34887, 35594, 37357]


def gh(path: str, *, paginate: bool = False):
    command = ["gh", "api"]
    if paginate:
        command.extend(["--paginate", "--slurp"])
    command.append(path)
    output = subprocess.check_output(command, text=True)
    value = json.loads(output)
    if paginate:
        return [item for page in value for item in page]
    return value


def actor(value):
    if not value:
        return None
    return value.get("login")


def normalize_issue(number: int):
    issue = gh(f"repos/{REPO_NAME}/issues/{number}")
    comments = gh(f"repos/{REPO_NAME}/issues/{number}/comments?per_page=100", paginate=True)
    timeline = gh(f"repos/{REPO_NAME}/issues/{number}/timeline?per_page=100", paginate=True)
    return {
        "number": number,
        "url": issue.get("html_url"),
        "title": issue.get("title"),
        "state": issue.get("state"),
        "state_reason": issue.get("state_reason"),
        "created_at": issue.get("created_at"),
        "updated_at": issue.get("updated_at"),
        "closed_at": issue.get("closed_at"),
        "author": actor(issue.get("user")),
        "labels": [label.get("name") for label in issue.get("labels", [])],
        "body": issue.get("body"),
        "comments": [
            {
                "id": comment.get("id"),
                "url": comment.get("html_url"),
                "author": actor(comment.get("user")),
                "created_at": comment.get("created_at"),
                "updated_at": comment.get("updated_at"),
                "body": comment.get("body"),
            }
            for comment in comments
        ],
        "timeline_links": [
            {
                "event": event.get("event"),
                "created_at": event.get("created_at"),
                "actor": actor(event.get("actor")),
                "commit_id": event.get("commit_id"),
                "source_issue": (
                    {
                        "number": event.get("source", {}).get("issue", {}).get("number"),
                        "title": event.get("source", {}).get("issue", {}).get("title"),
                        "url": event.get("source", {}).get("issue", {}).get("html_url"),
                        "is_pull_request": bool(
                            event.get("source", {}).get("issue", {}).get("pull_request")
                        ),
                    }
                    if event.get("source", {}).get("issue")
                    else None
                ),
            }
            for event in timeline
            if event.get("event") in {"cross-referenced", "connected", "closed", "reopened"}
        ],
    }


def normalize_pull(number: int):
    pull = gh(f"repos/{REPO_NAME}/pulls/{number}")
    issue_comments = gh(f"repos/{REPO_NAME}/issues/{number}/comments?per_page=100", paginate=True)
    review_comments = gh(f"repos/{REPO_NAME}/pulls/{number}/comments?per_page=100", paginate=True)
    reviews = gh(f"repos/{REPO_NAME}/pulls/{number}/reviews?per_page=100", paginate=True)
    commits = gh(f"repos/{REPO_NAME}/pulls/{number}/commits?per_page=100", paginate=True)
    return {
        "number": number,
        "url": pull.get("html_url"),
        "title": pull.get("title"),
        "state": pull.get("state"),
        "draft": pull.get("draft"),
        "merged": pull.get("merged"),
        "merge_commit_sha": pull.get("merge_commit_sha"),
        "created_at": pull.get("created_at"),
        "updated_at": pull.get("updated_at"),
        "closed_at": pull.get("closed_at"),
        "merged_at": pull.get("merged_at"),
        "author": actor(pull.get("user")),
        "base": pull.get("base", {}).get("ref"),
        "head": pull.get("head", {}).get("sha"),
        "body": pull.get("body"),
        "changed_files": pull.get("changed_files"),
        "additions": pull.get("additions"),
        "deletions": pull.get("deletions"),
        "commits": [
            {
                "sha": commit.get("sha"),
                "authored_at": commit.get("commit", {}).get("author", {}).get("date"),
                "committed_at": commit.get("commit", {}).get("committer", {}).get("date"),
                "message": commit.get("commit", {}).get("message"),
            }
            for commit in commits
        ],
        "issue_comments": [
            {
                "url": comment.get("html_url"),
                "author": actor(comment.get("user")),
                "created_at": comment.get("created_at"),
                "body": comment.get("body"),
            }
            for comment in issue_comments
        ],
        "review_comments": [
            {
                "url": comment.get("html_url"),
                "author": actor(comment.get("user")),
                "created_at": comment.get("created_at"),
                "path": comment.get("path"),
                "line": comment.get("line"),
                "body": comment.get("body"),
            }
            for comment in review_comments
        ],
        "reviews": [
            {
                "id": review.get("id"),
                "author": actor(review.get("user")),
                "submitted_at": review.get("submitted_at"),
                "state": review.get("state"),
                "body": review.get("body"),
            }
            for review in reviews
        ],
    }


def main():
    snapshot = {
        "schema_version": 1,
        "repository": REPO_NAME,
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "capture_commands": {
            "issue": "gh api repos/openai/codex/issues/<number>",
            "issue_comments": "gh api --paginate --slurp repos/openai/codex/issues/<number>/comments?per_page=100",
            "issue_timeline": "gh api --paginate --slurp repos/openai/codex/issues/<number>/timeline?per_page=100",
            "pull": "gh api repos/openai/codex/pulls/<number>",
            "pull_comments_reviews_commits": "gh api --paginate --slurp on the corresponding REST collections",
        },
        "issues": [normalize_issue(number) for number in ISSUES],
        "pull_requests": [normalize_pull(number) for number in PULLS],
    }
    OUT.write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"status": "ok", "issues": len(ISSUES), "pull_requests": len(PULLS)}))


if __name__ == "__main__":
    main()
