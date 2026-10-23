"""Synthetic process-boundary tests; no model, private media, or network needed."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import unittest
import venv
from unittest.mock import patch

from scripts.distribution import install_geocalib

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("geocalib_safety", ROOT / "model_adapters/geocalib/estimate.py")
ESTIMATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ESTIMATOR)


class LauncherIsolationTests(unittest.TestCase):
    def test_isolation_keeps_venv_packages_without_cwd_or_pythonpath(self):
        with tempfile.TemporaryDirectory(prefix="camera's environment ") as directory:
            # macOS exposes the same temp directory through /var and /private/var.
            root = Path(directory).resolve()
            environment = root / "venv"
            venv.EnvBuilder(with_pip=False).create(environment)
            python = environment / "bin/python"
            site = subprocess.check_output([str(python), "-I", "-c", "import sysconfig; print(sysconfig.get_path('purelib'))"], text=True).strip()
            # An inert installed module proves -I still reads the selected venv.
            (Path(site) / "camera_test_package.py").write_text("VALUE = 17\n")
            source = root / "model_adapters/geocalib"
            source.mkdir(parents=True)
            (source / "estimate.py").write_text("import json, sys, camera_test_package\nprint(json.dumps({'prefix': sys.prefix, 'path': sys.path, 'value': camera_test_package.VALUE, 'isolated': sys.flags.isolated}))\n")
            weights = root / "weights"
            weights.write_bytes(b"synthetic")
            cwd = root / "cwd"
            cwd.mkdir()
            injected = root / "pythonpath"
            injected.mkdir()
            with patch.object(install_geocalib, "ROOT", root), patch.object(install_geocalib, "WEIGHTS_SHA256", hashlib.sha256(b"synthetic").hexdigest()):
                helper = install_geocalib.install(python, weights, root)
            result = subprocess.run([str(helper)], cwd=cwd, env={**os.environ, "PYTHONPATH": str(injected)}, check=True, capture_output=True, text=True, timeout=10)
            facts = json.loads(result.stdout)
            self.assertEqual(facts["value"], 17)
            self.assertEqual(facts["prefix"], str(environment))
            self.assertNotIn(str(injected), facts["path"])
            self.assertNotIn(str(cwd), facts["path"])
            self.assertNotIn("", facts["path"])
            self.assertEqual(facts["isolated"], 1)


class BoundedProcessTests(unittest.TestCase):
    def run_child(self, source, **kwargs):
        return ESTIMATOR.run_bounded([sys.executable, "-I", "-c", source], stdout_limit=4096, stderr_limit=4096, timeout=2, **kwargs)

    def test_both_pipes_are_drained_without_deadlock(self):
        result = ESTIMATOR.run_bounded([sys.executable, "-I", "-c", "import os; os.write(2, b'e'*131072); os.write(1, b'o'*131072)"], stdout_limit=131072, stderr_limit=131072, timeout=2)
        self.assertEqual(result.stdout, b"o" * 131072)
        self.assertEqual(result.stderr, b"e" * 131072)

    def test_each_output_limit_kills_and_reaps_child(self):
        real_popen = subprocess.Popen
        for descriptor, name in [(1, "stdout"), (2, "stderr")]:
            children = []
            def record(*args, **kwargs):
                child = real_popen(*args, **kwargs)
                children.append(child)
                return child
            with self.subTest(pipe=name), patch.object(ESTIMATOR.subprocess, "Popen", side_effect=record):
                with self.assertRaisesRegex(ValueError, name + " exceeded"):
                    self.run_child(f"import os, time; os.write({descriptor}, b'x'*4097); time.sleep(30)")
                self.assertIsNotNone(children[0].returncode)
                with self.assertRaises(ChildProcessError):
                    os.waitpid(children[0].pid, os.WNOHANG)
                self.assertTrue(children[0].stdout.closed)
                self.assertTrue(children[0].stderr.closed)

    def test_timeout_with_open_or_closed_pipes_reaps_child(self):
        for prefix in ["", "os.close(1); os.close(2);"]:
            children = []
            real_popen = subprocess.Popen
            def record(*args, **kwargs):
                child = real_popen(*args, **kwargs)
                children.append(child)
                return child
            with self.subTest(prefix=prefix), patch.object(ESTIMATOR.subprocess, "Popen", side_effect=record):
                with self.assertRaises(subprocess.TimeoutExpired):
                    ESTIMATOR.run_bounded([sys.executable, "-I", "-c", "import os, time; " + prefix + "time.sleep(30)"], stdout_limit=1024, stderr_limit=1024, timeout=0.2)
                self.assertIsNotNone(children[0].returncode)
                with self.assertRaises(ChildProcessError):
                    os.waitpid(children[0].pid, os.WNOHANG)

    def test_nonzero_exit_keeps_bounded_diagnostics(self):
        with self.assertRaises(subprocess.CalledProcessError) as caught:
            self.run_child("import sys; print('invalid', file=sys.stderr); sys.exit(3)")
        self.assertEqual(caught.exception.returncode, 3)
        self.assertEqual(caught.exception.stderr, b"invalid\n")

    def test_probe_rejects_oversized_stdout_or_stderr(self):
        real_popen = subprocess.Popen
        for descriptor in [1, 2]:
            def synthetic_probe(command, **kwargs):
                self.assertEqual(command[0], "ffprobe")
                payload = json.dumps({"streams": [{"duration": "2"}]}).encode()
                source = f"import os; os.write({descriptor}, b' '*65537); os.write(1, {payload!r})"
                return real_popen([sys.executable, "-I", "-c", source], **kwargs)
            with self.subTest(descriptor=descriptor), patch.object(ESTIMATOR.subprocess, "Popen", side_effect=synthetic_probe):
                with self.assertRaisesRegex(ValueError, "exceeded"):
                    ESTIMATOR.source_duration(Path("synthetic.mov"))

    @unittest.skipUnless(shutil.which("ffmpeg") and shutil.which("ffprobe"), "FFmpeg is not installed")
    def test_real_probe_accepts_synthetic_video_with_large_unneeded_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            video = Path(directory) / "synthetic.mov"
            subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=size=32x32:rate=5", "-t", "1", "-c:v", "mpeg4", "-metadata", "comment=" + "a" * 70000, str(video)], check=True, timeout=10)
            self.assertAlmostEqual(ESTIMATOR.source_duration(video), 1.0)

    def test_probe_selects_only_required_fields(self):
        facts = {"streams": [{"duration": "2.5", "sample_aspect_ratio": "1:1", "disposition": {"attached_pic": 0}}]}
        with patch.object(ESTIMATOR, "run_bounded", return_value=subprocess.CompletedProcess([], 0, json.dumps(facts).encode(), b"")) as run:
            self.assertEqual(ESTIMATOR.source_duration(Path("clip.mov")), 2.5)
        args = run.call_args.args[0]
        self.assertNotIn("-show_streams", args)
        self.assertNotIn("-show_format", args)
        self.assertEqual(args[args.index("-show_entries") + 1], "stream=duration,sample_aspect_ratio,color_transfer:stream_disposition=attached_pic:format=duration")


if __name__ == "__main__":
    unittest.main()
