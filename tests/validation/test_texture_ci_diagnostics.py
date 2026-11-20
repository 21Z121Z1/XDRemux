"""Exercise the Texture CI adapter observer without Apple frameworks."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.apple.check_texture_styles import WRAPPER


@unittest.skipIf(os.name == "nt", "The macOS observer launches a POSIX executable")
class TextureCIDiagnosticsTests(unittest.TestCase):
    def test_adapter_eof_reports_only_bounded_native_failure_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            observer = root / "observer.py"
            observer.write_text(WRAPPER)
            adapter = root / "adapter"
            adapter.write_text(
                "#!/bin/sh\n"
                "IFS= read -r request\n"
                "exit 2\n"
            )
            adapter.chmod(0o755)
            report = root / "UnderEyeBrighten.json"
            report.write_text(json.dumps({
                "stage": "calculateStats",
                "exception": {"name": "ExampleException", "reason": "shader unavailable"},
                "image_pixels": {"raw": "internal pixel payload"},
                "person_before_stats": {"geometry": [1, 2]},
            }))
            request = {
                "operation": "texture-statistics",
                "output_path": str(report),
                "texture": {"effect": "UnderEyeBrighten"},
            }
            result = subprocess.run(
                [sys.executable, str(observer)],
                input=json.dumps(request) + "\n",
                env={
                    **os.environ,
                    "XDREMUX_REAL_TEXTURE_ADAPTER": str(adapter),
                    "XDREMUX_TEXTURE_OBSERVATIONS": directory,
                },
                capture_output=True,
                text=True,
                timeout=10,
            )

        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        prefix = "Texture native failure "
        line, = [line for line in result.stderr.splitlines() if line.startswith(prefix)]
        self.assertEqual(json.loads(line.removeprefix(prefix)), {
            "operation": "texture-statistics",
            "effect": "UnderEyeBrighten",
            "stage": "calculateStats",
            "exception": "ExampleException",
            "reason": "shader unavailable",
        })
