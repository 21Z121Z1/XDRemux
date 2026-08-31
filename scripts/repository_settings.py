#!/usr/bin/env python3
"""Audit and apply the repository-level security baseline for XDRemux.

The script intentionally uses only the Python standard library so the bootstrap
workflow does not need to install dependencies before changing repository
settings. Write operations require a token with repository Administration
read/write permission.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from typing import Any

API_ROOT = "https://api.github.com"
API_VERSION = "2026-03-10"
RULESET_NAME = "XDRemux default branch protection"
TOPICS = [
    "gain-map",
    "hdr",
    "heic",
    "heif",
    "image-processing",
    "iso-21496",
    "motion-photo",
    "proxdr",
    "swift",
]
CODEQL_LANGUAGES = ["actions", "python", "swift"]


def repository_patch() -> dict[str, Any]:
    return {
        "allow_auto_merge": True,
        "delete_branch_on_merge": True,
        "allow_update_branch": True,
        "security_and_analysis": {
            "secret_scanning": {"status": "enabled"},
            "secret_scanning_push_protection": {"status": "enabled"},
        },
    }


def workflow_permissions_payload() -> dict[str, Any]:
    return {
        "default_workflow_permissions": "read",
        "can_approve_pull_request_reviews": False,
    }


def codeql_payload() -> dict[str, Any]:
    return {
        "state": "configured",
        "runner_type": "standard",
        "query_suite": "default",
        "languages": CODEQL_LANGUAGES,
    }


def ruleset_payload(enforce_codeql: bool) -> dict[str, Any]:
    rules: list[dict[str, Any]] = [
        {"type": "deletion"},
        {"type": "non_fast_forward"},
        {
            "type": "pull_request",
            "parameters": {
                "allowed_merge_methods": ["merge", "squash", "rebase"],
                "dismiss_stale_reviews_on_push": False,
                "require_code_owner_review": False,
                "require_last_push_approval": False,
                "required_approving_review_count": 0,
                "required_review_thread_resolution": True,
            },
        },
    ]
    if enforce_codeql:
        rules.append(
            {
                "type": "code_scanning",
                "parameters": {
                    "code_scanning_tools": [
                        {
                            "tool": "CodeQL",
                            "alerts_threshold": "errors",
                            "security_alerts_threshold": "high_or_higher",
                        }
                    ]
                },
            }
        )
    return {
        "name": RULESET_NAME,
        "target": "branch",
        "enforcement": "active",
        "conditions": {
            "ref_name": {
                "include": ["~DEFAULT_BRANCH"],
                "exclude": [],
            }
        },
        "rules": rules,
    }


class GitHubAPI:
    def __init__(self, repository: str, token: str) -> None:
        if repository.count("/") != 1:
            raise ValueError("repository must use OWNER/REPO form")
        self.repository = repository
        self.token = token

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        expected: tuple[int, ...] = (200,),
    ) -> tuple[int, Any]:
        data = None
        headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {self.token}",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "XDRemux-repository-settings",
        }
        if payload is not None:
            data = json.dumps(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(
            f"{API_ROOT}{path}", data=data, headers=headers, method=method
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                status = response.status
                body = response.read()
        except urllib.error.HTTPError as error:
            status = error.code
            body = error.read()
            if status not in expected:
                detail = body.decode("utf-8", errors="replace")
                raise RuntimeError(
                    f"{method} {path} failed with HTTP {status}: {detail}"
                ) from error
        if status not in expected:
            detail = body.decode("utf-8", errors="replace")
            raise RuntimeError(
                f"{method} {path} returned HTTP {status}; expected {expected}: {detail}"
            )
        if not body:
            return status, None
        return status, json.loads(body)

    def repo_path(self, suffix: str = "") -> str:
        return f"/repos/{self.repository}{suffix}"


def upsert_ruleset(api: GitHubAPI, enforce_codeql: bool) -> dict[str, Any]:
    _, rulesets = api.request("GET", api.repo_path("/rulesets"), expected=(200,))
    matches = [item for item in rulesets if item.get("name") == RULESET_NAME]
    payload = ruleset_payload(enforce_codeql)
    if len(matches) > 1:
        raise RuntimeError(f"more than one ruleset is named {RULESET_NAME!r}")
    if matches:
        ruleset_id = matches[0]["id"]
        _, result = api.request(
            "PUT",
            api.repo_path(f"/rulesets/{ruleset_id}"),
            payload,
            expected=(200,),
        )
        return result
    _, result = api.request(
        "POST", api.repo_path("/rulesets"), payload, expected=(201,)
    )
    return result


def apply_settings(api: GitHubAPI, enforce_codeql: bool) -> dict[str, Any]:
    results: dict[str, Any] = {}
    _, results["repository"] = api.request(
        "PATCH", api.repo_path(), repository_patch(), expected=(200,)
    )
    api.request("PUT", api.repo_path("/vulnerability-alerts"), expected=(204,))
    results["dependabot_alerts"] = "enabled"
    api.request("PUT", api.repo_path("/automated-security-fixes"), expected=(204,))
    results["dependabot_security_updates"] = "enabled"
    api.request(
        "PUT", api.repo_path("/private-vulnerability-reporting"), expected=(204,)
    )
    results["private_vulnerability_reporting"] = "enabled"
    api.request(
        "PUT",
        api.repo_path("/actions/permissions/workflow"),
        workflow_permissions_payload(),
        expected=(204,),
    )
    results["workflow_permissions"] = workflow_permissions_payload()
    _, results["topics"] = api.request(
        "PUT", api.repo_path("/topics"), {"names": TOPICS}, expected=(200,)
    )
    codeql_status, codeql_result = api.request(
        "PATCH",
        api.repo_path("/code-scanning/default-setup"),
        codeql_payload(),
        expected=(200, 202),
    )
    results["codeql"] = {
        "http_status": codeql_status,
        "response": codeql_result,
    }
    results["ruleset"] = upsert_ruleset(api, enforce_codeql)
    return results


def audit_settings(api: GitHubAPI, enforce_codeql: bool) -> dict[str, Any]:
    checks: dict[str, bool] = {}
    observed: dict[str, Any] = {}

    _, repository = api.request("GET", api.repo_path(), expected=(200,))
    observed["repository"] = {
        "allow_auto_merge": repository.get("allow_auto_merge"),
        "delete_branch_on_merge": repository.get("delete_branch_on_merge"),
        "allow_update_branch": repository.get("allow_update_branch"),
        "security_and_analysis": repository.get("security_and_analysis"),
    }
    checks["merge_settings"] = all(
        repository.get(name) is True
        for name in ("allow_auto_merge", "delete_branch_on_merge", "allow_update_branch")
    )
    security = repository.get("security_and_analysis") or {}
    checks["secret_scanning"] = (
        (security.get("secret_scanning") or {}).get("status") == "enabled"
    )
    checks["push_protection"] = (
        (security.get("secret_scanning_push_protection") or {}).get("status")
        == "enabled"
    )

    status, codeql = api.request(
        "GET",
        api.repo_path("/code-scanning/default-setup"),
        expected=(200, 403, 404),
    )
    observed["codeql"] = {"http_status": status, "response": codeql}
    codeql_languages = set((codeql or {}).get("languages", [])) if status == 200 else set()
    checks["codeql"] = (
        status == 200
        and (codeql or {}).get("state") == "configured"
        and (codeql or {}).get("query_suite") == "default"
        and set(CODEQL_LANGUAGES).issubset(codeql_languages)
    )

    vulnerability_status, _ = api.request(
        "GET", api.repo_path("/vulnerability-alerts"), expected=(204, 404)
    )
    observed["dependabot_alerts_http_status"] = vulnerability_status
    checks["dependabot_alerts"] = vulnerability_status == 204

    fixes_status, fixes = api.request(
        "GET", api.repo_path("/automated-security-fixes"), expected=(200, 404)
    )
    observed["dependabot_security_updates"] = {
        "http_status": fixes_status,
        "response": fixes,
    }
    checks["dependabot_security_updates"] = (
        fixes_status == 200 and (fixes or {}).get("enabled") is True
    )

    _, private_reporting = api.request(
        "GET", api.repo_path("/private-vulnerability-reporting"), expected=(200,)
    )
    observed["private_vulnerability_reporting"] = private_reporting
    checks["private_vulnerability_reporting"] = private_reporting.get("enabled") is True

    _, workflow_permissions = api.request(
        "GET", api.repo_path("/actions/permissions/workflow"), expected=(200,)
    )
    observed["workflow_permissions"] = workflow_permissions
    checks["workflow_permissions"] = (
        workflow_permissions.get("default_workflow_permissions") == "read"
        and workflow_permissions.get("can_approve_pull_request_reviews") is False
    )

    _, topics = api.request("GET", api.repo_path("/topics"), expected=(200,))
    observed["topics"] = topics
    checks["topics"] = set(TOPICS).issubset(set(topics.get("names", [])))

    _, rulesets = api.request("GET", api.repo_path("/rulesets"), expected=(200,))
    matches = [item for item in rulesets if item.get("name") == RULESET_NAME]
    observed["ruleset"] = matches
    checks["ruleset"] = len(matches) == 1 and matches[0].get("enforcement") == "active"
    if checks["ruleset"] and enforce_codeql:
        _, full_ruleset = api.request(
            "GET", api.repo_path(f"/rulesets/{matches[0]['id']}"), expected=(200,)
        )
        checks["ruleset_codeql"] = any(
            rule.get("type") == "code_scanning" for rule in full_ruleset.get("rules", [])
        )

    return {
        "compliant": all(checks.values()),
        "checks": checks,
        "observed": observed,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repository",
        default=os.environ.get("GITHUB_REPOSITORY"),
        help="Repository in OWNER/REPO form. Defaults to GITHUB_REPOSITORY.",
    )
    parser.add_argument("--mode", choices=("audit", "apply"), default="audit")
    parser.add_argument(
        "--enforce-codeql",
        action="store_true",
        help="Add CodeQL merge protection to the default-branch ruleset.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    if not args.repository:
        print("repository is required", file=sys.stderr)
        return 64
    token = os.environ.get("REPOSITORY_ADMIN_TOKEN") or os.environ.get("GH_TOKEN")
    if not token:
        print(
            "REPOSITORY_ADMIN_TOKEN is required. Use a fine-grained token limited "
            "to this repository with Administration read/write permission.",
            file=sys.stderr,
        )
        return 64

    api = GitHubAPI(args.repository, token)
    if args.mode == "apply":
        result = apply_settings(api, args.enforce_codeql)
        print(json.dumps({"applied": result}, indent=2, sort_keys=True))
        # CodeQL default setup may return 202 while GitHub validates the new
        # configuration. Do not treat that asynchronous validation window as a
        # failed write. A later audit verifies the settled state.
        return 0

    result = audit_settings(api, args.enforce_codeql)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["compliant"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
