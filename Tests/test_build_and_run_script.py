from pathlib import Path
import os
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "build_and_run.sh"
SCRIPT_TEXT = SCRIPT.read_text()


class BuildAndRunScriptTests(unittest.TestCase):
    def test_build_mode_does_not_launch_app(self) -> None:
        build_case = SCRIPT_TEXT.split("  build)", 1)[1].split("    ;;", 1)[0]
        self.assertNotIn("launch_app", build_case)
        self.assertNotIn("open ", build_case)

    def test_verify_mode_checks_launch_failure(self) -> None:
        verify_case = SCRIPT_TEXT.split("  verify)", 1)[1].split("    ;;", 1)[0]
        self.assertIn("launch_app", verify_case)
        self.assertIn("verify_process", verify_case)

    def test_failed_build_prints_error_and_log_paths(self) -> None:
        with tempfile.TemporaryDirectory(prefix="xdremux-build-script-") as directory:
            root = Path(directory)
            binary_directory = root / "bin"
            binary_directory.mkdir()
            fake_xcodebuild = binary_directory / "xcodebuild"
            fake_xcodebuild.write_text(
                "#!/usr/bin/env bash\necho 'Broken.swift:7: error: expected expression' >&2\nexit 65\n"
            )
            fake_xcodebuild.chmod(0o755)
            environment = os.environ.copy()
            environment["PATH"] = f"{binary_directory}:{environment['PATH']}"
            environment["XDREMUX_DERIVED_DATA"] = str(root / "derived")

            result = subprocess.run(
                [str(SCRIPT), "build"],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Broken.swift:7: error: expected expression", result.stderr)
            self.assertIn("Full log:", result.stderr)
            self.assertIn("Result bundle:", result.stderr)

    def test_verbose_build_streams_complete_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="xdremux-build-script-verbose-") as directory:
            root = Path(directory)
            binary_directory = root / "bin"
            binary_directory.mkdir()
            fake_xcodebuild = binary_directory / "xcodebuild"
            fake_xcodebuild.write_text(
                "#!/usr/bin/env bash\necho 'full verbose compiler line'\nexit 65\n"
            )
            fake_xcodebuild.chmod(0o755)
            environment = os.environ.copy()
            environment["PATH"] = f"{binary_directory}:{environment['PATH']}"
            environment["XDREMUX_DERIVED_DATA"] = str(root / "derived")

            result = subprocess.run(
                [str(SCRIPT), "build", "--verbose"],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("full verbose compiler line", result.stdout)


if __name__ == "__main__":
    unittest.main()
