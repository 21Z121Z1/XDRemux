import importlib.util
import pathlib
import unittest


MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "scripts" / "repository_settings.py"
SPEC = importlib.util.spec_from_file_location("repository_settings", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
repository_settings = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(repository_settings)


class RepositorySettingsPayloadTests(unittest.TestCase):
    def test_repository_patch_enables_expected_safe_defaults(self) -> None:
        payload = repository_settings.repository_patch()
        self.assertTrue(payload["allow_auto_merge"])
        self.assertTrue(payload["delete_branch_on_merge"])
        self.assertTrue(payload["allow_update_branch"])
        self.assertEqual(
            payload["security_and_analysis"]["secret_scanning"]["status"],
            "enabled",
        )
        self.assertEqual(
            payload["security_and_analysis"]["secret_scanning_push_protection"][
                "status"
            ],
            "enabled",
        )

    def test_workflow_permissions_are_read_only(self) -> None:
        self.assertEqual(
            repository_settings.workflow_permissions_payload(),
            {
                "default_workflow_permissions": "read",
                "can_approve_pull_request_reviews": False,
            },
        )

    def test_codeql_uses_default_suite_and_detected_languages(self) -> None:
        self.assertEqual(
            repository_settings.codeql_payload(),
            {
                "state": "configured",
                "runner_type": "standard",
                "query_suite": "default",
                "languages": ["actions", "python", "swift"],
            },
        )

    def test_baseline_ruleset_requires_exact_head_gate_without_codeql(self) -> None:
        payload = repository_settings.ruleset_payload(enforce_codeql=False)
        rule_types = {rule["type"] for rule in payload["rules"]}
        self.assertEqual(payload["enforcement"], "active")
        self.assertEqual(
            payload["conditions"]["ref_name"]["include"], ["~DEFAULT_BRANCH"]
        )
        self.assertIn("deletion", rule_types)
        self.assertIn("non_fast_forward", rule_types)
        self.assertIn("pull_request", rule_types)
        self.assertIn("required_status_checks", rule_types)
        self.assertNotIn("code_scanning", rule_types)

        pull_request_rule = next(
            rule for rule in payload["rules"] if rule["type"] == "pull_request"
        )
        parameters = pull_request_rule["parameters"]
        self.assertEqual(parameters["required_approving_review_count"], 0)
        self.assertTrue(parameters["required_review_thread_resolution"])

        status_rule = next(
            rule
            for rule in payload["rules"]
            if rule["type"] == "required_status_checks"
        )
        status_parameters = status_rule["parameters"]
        self.assertEqual(
            status_parameters["required_status_checks"],
            [{"context": "exact-head"}],
        )
        self.assertTrue(status_parameters["strict_required_status_checks_policy"])

    def test_codeql_merge_protection_is_opt_in_after_initial_scan(self) -> None:
        payload = repository_settings.ruleset_payload(enforce_codeql=True)
        codeql_rule = next(
            rule for rule in payload["rules"] if rule["type"] == "code_scanning"
        )
        tool = codeql_rule["parameters"]["code_scanning_tools"][0]
        self.assertEqual(tool["tool"], "CodeQL")
        self.assertEqual(tool["alerts_threshold"], "errors")
        self.assertEqual(tool["security_alerts_threshold"], "high_or_higher")

    def test_topics_are_normalized_and_unique(self) -> None:
        topics = repository_settings.TOPICS
        self.assertEqual(len(topics), len(set(topics)))
        self.assertEqual(topics, sorted(topics))


if __name__ == "__main__":
    unittest.main()
