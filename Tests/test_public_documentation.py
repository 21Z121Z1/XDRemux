import json
from pathlib import Path
import re
import subprocess
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]

# Current normative documents. Historical evidence records are intentionally not
# rewritten or required to have a translated body.
BILINGUAL_STEMS = (
    "README",
    "docs/README",
    "docs/style-guide",
    "docs/architecture",
    "docs/roadmap",
    "docs/exec-plans/README",
    "docs/cli",
    "docs/apple-features",
    "docs/development",
    "docs/supported-devices",
    "docs/quality/testing",
    "docs/quality/evals",
    "docs/quality/logging",
    "docs/validation/README",
    "docs/xdremux/README",
    "Tests/README",
    "fixtures/README",
    "Models/ReverseKey1Ensemble.model-card",
    "docs/validation/encoding-quality-pareto-20260718.summary",
    "docs/validation/vendor-live-photo-geometry.summary",
    "docs/xdremux/iso-conformance-audit-20260511.summary",
)

# AGENTS.md is a tool-discovered fixed filename, so English remains at the
# conventional path and Chinese is published as a sidecar.
SPECIAL_BILINGUAL_PAIRS = (
    ("AGENTS.md", "AGENTS.zh-CN.md"),
)

PUBLIC_DOCUMENTS = tuple(
    path
    for stem in BILINGUAL_STEMS
    for path in (ROOT / f"{stem}.md", ROOT / f"{stem}.en.md")
) + tuple(
    ROOT / path
    for pair in SPECIAL_BILINGUAL_PAIRS
    for path in pair
)

HISTORICAL_RECORDS = (
    ROOT / "docs/validation/encoding-quality-pareto-20260718.md",
    ROOT / "docs/validation/vendor-live-photo-geometry.md",
    ROOT / "docs/xdremux/iso-conformance-audit-20260511.md",
)

MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


class PublicDocumentationTests(unittest.TestCase):
    def test_bilingual_readmes_publish_matching_categorize_workflows(self) -> None:
        required = (
            "swift run xdremux categorize",
            "python3 -m xdremux_py categorize",
            "--categorize",
        )
        forbidden = ("--categorize-output", "--organize-by-mode", "xdremux classify")
        for readme in (ROOT / "README.md", ROOT / "README.en.md"):
            text = readme.read_text(encoding="utf-8")
            for value in required:
                self.assertIn(value, text, f"{value} missing from {readme.name}")
            for value in forbidden:
                self.assertNotIn(value, text, f"legacy naming in {readme.name}: {value}")

    def test_local_links_in_public_documents_resolve_inside_the_repository(self) -> None:
        for document in PUBLIC_DOCUMENTS:
            self.assertTrue(document.is_file(), f"missing public document: {document}")
            for match in MARKDOWN_LINK.finditer(document.read_text(encoding="utf-8")):
                target = match.group(1).strip().strip("<>")
                if target.startswith(("https://", "http://", "mailto:", "#")):
                    continue
                target = target.split("#", maxsplit=1)[0]
                if not target:
                    continue
                resolved = (document.parent / target).resolve()
                try:
                    resolved.relative_to(ROOT)
                except ValueError:
                    self.fail(f"{document}: link escapes repository: {target}")
                self.assertTrue(resolved.exists(), f"{document}: missing link target {target}")

    def test_standard_bilingual_documents_exist_and_cross_link(self) -> None:
        for stem in BILINGUAL_STEMS:
            chinese = ROOT / f"{stem}.md"
            english = ROOT / f"{stem}.en.md"
            self.assertTrue(chinese.is_file(), f"missing Chinese document: {stem}.md")
            self.assertTrue(english.is_file(), f"missing English document: {stem}.en.md")

            name = Path(stem).name
            self.assertIn(
                f"({name}.en.md)",
                chinese.read_text(encoding="utf-8"),
                f"{stem}.md does not link to its English version",
            )
            self.assertIn(
                f"({name}.md)",
                english.read_text(encoding="utf-8"),
                f"{stem}.en.md does not link to its Chinese version",
            )

    def test_special_bilingual_documents_exist_and_cross_link(self) -> None:
        for english_relative, chinese_relative in SPECIAL_BILINGUAL_PAIRS:
            english = ROOT / english_relative
            chinese = ROOT / chinese_relative
            self.assertTrue(english.is_file(), f"missing English document: {english_relative}")
            self.assertTrue(chinese.is_file(), f"missing Chinese document: {chinese_relative}")
            self.assertIn(
                f"({Path(chinese_relative).name})",
                english.read_text(encoding="utf-8"),
            )
            self.assertIn(
                f"({Path(english_relative).name})",
                chinese.read_text(encoding="utf-8"),
            )

    def test_historical_records_are_kept_with_current_bilingual_summaries(self) -> None:
        for record in HISTORICAL_RECORDS:
            self.assertTrue(record.is_file(), f"missing historical record: {record}")
            stem = record.with_suffix("")
            english_summary = Path(f"{stem}.summary.en.md")
            chinese_summary = Path(f"{stem}.summary.md")
            self.assertTrue(english_summary.is_file(), f"missing historical English summary: {record}")
            self.assertTrue(chinese_summary.is_file(), f"missing historical Chinese summary: {record}")

    def test_style_guide_defines_canonical_language_and_non_compliance_claim(self) -> None:
        english = (ROOT / "docs/style-guide.en.md").read_text(encoding="utf-8")
        self.assertIn("English is the canonical source", english)
        self.assertIn("does not claim formal ASD-STE100 compliance", english)
        for term in ("Motion Photo", "Live Photo", "Gain Map", "still-image-time"):
            self.assertIn(term, english)

    def test_current_cli_docs_describe_python_motion_photo_support(self) -> None:
        english = (ROOT / "docs/cli.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/cli.md").read_text(encoding="utf-8")
        self.assertIn("Motion Photo to Live Photo conversion", english)
        self.assertNotIn("does HDR conversion only", english)
        self.assertIn("Motion Photo", chinese)
        self.assertIn("Live Photo", chinese)

    def test_current_quality_docs_acknowledge_versioned_motion_fixtures(self) -> None:
        for relative in (
            "docs/quality/testing.en.md",
            "docs/quality/evals.en.md",
            "Tests/README.en.md",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("fixtures/", text)
            self.assertNotIn("Real samples are not in the repository", text)

    def test_agent_map_is_valid_and_matches_architecture_capabilities(self) -> None:
        agent_map_path = ROOT / "docs/agent-map.json"
        self.assertTrue(agent_map_path.is_file())
        agent_map = json.loads(agent_map_path.read_text(encoding="utf-8"))
        self.assertEqual(agent_map["schema_version"], 1)

        architecture = (ROOT / "docs/architecture.en.md").read_text(encoding="utf-8")
        capabilities = agent_map["capabilities"]
        identifiers = [item["id"] for item in capabilities]
        self.assertEqual(len(identifiers), len(set(identifiers)))
        for identifier in identifiers:
            self.assertIn(f"`{identifier}`", architecture)

        branch_roles = agent_map["branch_roles"]
        for branch in (
            "main",
            "feat/rust-xdremux-format",
            "codex/reverse-key1-oppo-solver",
        ):
            self.assertIn(branch, branch_roles)
            metadata = branch_roles[branch]
            for field in ("role", "intended_base", "promotion_gate", "retirement_condition"):
                self.assertIn(field, metadata)

        codec = next(item for item in capabilities if item["id"] == "adapter.codec")
        self.assertIn("crates/xdremux-codec", codec["rust_owner"])

    def test_agent_context_routes_capability_without_repository_scan(self) -> None:
        helper = ROOT / "scripts/agent_context.py"
        self.assertTrue(helper.is_file())
        output = subprocess.check_output(
            [sys.executable, str(helper), "capability", "engine.plan", "--json"],
            cwd=ROOT,
            text=True,
        )
        payload = json.loads(output)
        self.assertEqual(payload["id"], "engine.plan")
        self.assertEqual(payload["layer"], 3)
        self.assertIn("crates/xdremux-engine", payload["rust_owner"])

    def test_agent_system_docs_publish_bootstrap_and_transition_contract(self) -> None:
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        architecture = (ROOT / "docs/architecture.en.md").read_text(encoding="utf-8")
        roadmap = (ROOT / "docs/roadmap.en.md").read_text(encoding="utf-8")
        validation = (ROOT / "docs/validation/README.en.md").read_text(encoding="utf-8")
        execution_plans = (ROOT / "docs/exec-plans/README.en.md").read_text(encoding="utf-8")
        development = (ROOT / "docs/development.en.md").read_text(encoding="utf-8")
        pr_template = (ROOT / ".github/pull_request_template.md").read_text(encoding="utf-8")

        for path in (
            "docs/architecture.en.md",
            "docs/agent-map.json",
            "docs/roadmap.en.md",
            "docs/validation/README.en.md",
            "docs/exec-plans/README.en.md",
        ):
            self.assertIn(path, agents)

        self.assertIn("scripts/agent_context.py status", agents)
        self.assertIn("scripts/agent_context.py capability", agents)
        self.assertIn("xdremux-codec", architecture)
        self.assertIn("xdremux-codec", roadmap)
        self.assertIn("Cargo.toml", roadmap)

        for migration_field in (
            "normalized contract",
            "Rust owner",
            "promotion evidence",
        ):
            self.assertIn(migration_field, roadmap)

        for evidence_role in ("Required gate", "Promotion evidence", "Diagnostic probe"):
            self.assertIn(evidence_role, validation)

        for plan_field in (
            "Target capability / layer",
            "Last verified HEAD",
            "Residual gaps",
            "Next action",
        ):
            self.assertIn(plan_field, execution_plans)

        for ledger_field in (
            "Target capability / layer:",
            "Invariant that must remain true:",
            "Exact committed HEAD:",
            "Residual gaps",
            "Normalized contract:",
            "Diagnostic probes used for discovery only:",
        ):
            self.assertIn(ledger_field, pr_template)

        self.assertIn("is the final release that ships both", development)
        self.assertNotIn("There is no stable release tag contract", development)

    def test_ci_references_present_documentation_test_module(self) -> None:
        workflow = ROOT / ".github" / "workflows" / "ci.yml"
        text = workflow.read_text(encoding="utf-8")
        self.assertIn("Tests.test_public_documentation", text)
        self.assertTrue((ROOT / "Tests" / "test_public_documentation.py").is_file())


if __name__ == "__main__":
    unittest.main()
