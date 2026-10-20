"""Behavioral checks for complete native Tone measurements and failure receipts."""
from pathlib import Path
import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest

import numpy as np

from scripts.apple import check_style_tone_response as tone


class StyleToneResponseTests(unittest.TestCase):
    def metadata(self, resources, name):
        photo = resources / name
        photo.mkdir(parents=True)
        (photo / "styles-summary.json").write_text(json.dumps({"0": 15, "h": 0.5}))
        np.concatenate(([257], np.rint(np.linspace(0, 65534, 256)), [65534])).astype("<u2").tofile(photo / "key3.bin")
        for key in ("c", "d"):
            np.full(6, 0.4, dtype="<f2").tofile(photo / f"key{key}.bin")

    def make_sweep(self, root, name="sample.xdremux"):
        root.mkdir()
        photos = [name]
        rows = []
        for value in tone.TONES:
            renderer = root / name / f"tone-{value:g}" / "renderer-0"
            renderer.mkdir(parents=True)
            pixels = np.full((3, 4, 4), 0.4 + 0.05*value, dtype="<f2")
            pixels[:, :, 3] = 1
            pixels.tofile(renderer / "_outputImageTexture.bin")
            (renderer / "trace.json").write_text(json.dumps({"process_status": 0, "finish_status": 0,
                                                             "textures": {"_outputImageTexture": {"width": 4, "height": 3}}}))
            rows.append({"photo": name, "tone": value, "returncode": 0, "components": 36,
                         "differing_components": 0 if value == 0 else 1,
                         "max_abs": 0 if value == 0 else 0.01, "psnr_peak1_db": "inf" if value == 0 else 40})
        (root / "tone-sweep.json").write_text(json.dumps(rows))
        (root / "consumer-run.json").write_text(json.dumps({"status": "completed", "returncode": 0,
                                                            "photos": photos, "tones": list(tone.TONES)}))
        sanity = root.parent / (root.name + "-sanity.json")
        sanity.write_text(json.dumps([{"photo": "IMG_0032", "tone": 0, "returncode": 0,
                                      "components": 36, "differing_components": 0,
                                      "max_abs": 0, "psnr_peak1_db": "inf"}]))
        return sanity

    def test_reference_pass_cannot_admit_an_empty_product_sweep(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            sanity = self.make_sweep(root / "renders")
            for renderer in (root / "renders").glob("*/tone-*/renderer-0"):
                (renderer / "_outputImageTexture.bin").unlink()
            report = tone.analyze(sanity, root / "renders", root / "report")
            self.assertFalse(report["trusted"])
            self.assertEqual(report["status"], "failed")

    def test_all_requested_tones_and_photos_are_required(self):
        for missing in ("tone", "photo"):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                sanity = self.make_sweep(root / "renders")
                if missing == "tone":
                    (root / "renders/sample.xdremux/tone--1/renderer-0/_outputImageTexture.bin").unlink()
                else:
                    path = root / "renders/consumer-run.json"
                    receipt = json.loads(path.read_text())
                    receipt["photos"].append("omitted.xdremux")
                    path.write_text(json.dumps(receipt))
                self.assertFalse(tone.analyze(sanity, root / "renders", root / "report")["trusted"])

    def test_failed_process_or_model_comparison_cannot_reuse_present_renders(self):
        for target in ("consumer-run.json", "tone-sweep.json"):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                sanity = self.make_sweep(root / "renders")
                path = root / "renders" / target
                data = json.loads(path.read_text())
                if isinstance(data, list):
                    data[0]["returncode"] = 7
                else:
                    data.update(status="failed", returncode=7)
                path.write_text(json.dumps(data))
                self.assertFalse(tone.analyze(sanity, root / "renders", root / "report")["trusted"])

    def test_corrupt_or_nonfinite_render_leaves_a_failure_report(self):
        for corrupt in ("truncated", "nonfinite", "shape", "native-status-type"):
            with self.subTest(corrupt=corrupt), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                sanity = self.make_sweep(root / "renders")
                renderer = root / "renders/sample.xdremux/tone-1/renderer-0"
                path = renderer / "_outputImageTexture.bin"
                if corrupt == "truncated":
                    path.write_bytes(path.read_bytes()[:-1])
                elif corrupt == "nonfinite":
                    values = np.fromfile(path, dtype="<f2")
                    values[0] = np.nan
                    values.tofile(path)
                elif corrupt == "shape":
                    trace = json.loads((renderer / "trace.json").read_text())
                    trace["textures"]["_outputImageTexture"]["width"] = -1
                    (renderer / "trace.json").write_text(json.dumps(trace))
                else:
                    trace = json.loads((renderer / "trace.json").read_text())
                    trace["process_status"] = None
                    (renderer / "trace.json").write_text(json.dumps(trace))
                result = subprocess.run([sys.executable, str(Path(tone.__file__)), "--sanity", str(sanity),
                                         "--renders", str(root / "renders"), "--out", str(root / "report")],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                report = json.loads((root / "report/tone-response.json").read_text())
                self.assertFalse(report["trusted"])
                self.assertEqual(report["status"], "failed")

    def test_host_gate_requires_a_successful_nonempty_native_comparison(self):
        row = {"photo": "IMG_0032", "tone": 0, "returncode": 1, "components": 36,
               "differing_components": 0, "max_abs": 0, "psnr_peak1_db": "inf"}
        self.assertFalse(tone.sanity_gate([row])["trusted"])
        row.update(returncode=0, components=0)
        self.assertFalse(tone.sanity_gate([row])["trusted"])

    def test_nonzero_tone_keeps_signed_zero_bit_difference_and_infinite_psnr(self):
        # The fixed consumer counts differing half bits, but computes PSNR on
        # numeric RGB differences. +0/-0 may therefore differ with max_abs=0.
        rows = [{"photo": "sample.xdremux", "tone": value, "returncode": 0, "components": 36,
                 "differing_components": 0 if value == 0 else 1, "max_abs": 0,
                 "psnr_peak1_db": "inf"} for value in tone.TONES]
        self.assertEqual(tone.sweep_errors(rows, ["sample.xdremux"]), [])

    def test_complete_data_still_produces_real_tone_statistics(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            sanity = self.make_sweep(root / "renders")
            report = tone.analyze(sanity, root / "renders", root / "report")
            self.assertTrue(report["trusted"])
            self.assertEqual(len(report["photos"][0]["tones"]), 11)
            self.assertLess(report["photos"][0]["signed_luminance_at_minus_1"], 0)
            self.assertGreater(report["photos"][0]["signed_luminance_at_plus_1"], 0)

    def test_comparison_rejects_missing_product_measurements_and_metadata(self):
        apple = {"trusted": True, "photos": [{"photo": "IMG_0032", "darken_brighten_ratio": 0.2}]}
        product = {"trusted": True, "photos": [{"photo": "sample.xdremux"}]}
        with tempfile.TemporaryDirectory() as raw:
            result = tone.write_asymmetry(apple, product, Path(raw) / "missing", Path(raw) / "report")
            self.assertEqual(result.get("status"), "failed")
            self.assertFalse(result.get("trusted"))
            self.assertNotIn("No numeric plist field", (Path(raw) / "report/asymmetry.md").read_text())

    def test_complete_comparison_requires_both_groups_and_real_metadata_arrays(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            apple_sanity = self.make_sweep(root / "apple", "IMG_0032")
            product_sanity = self.make_sweep(root / "product")
            apple = tone.analyze(apple_sanity, root / "apple", root / "apple-report")
            product = tone.analyze(product_sanity, root / "product", root / "product-report")
            self.metadata(root / "resources", "IMG_0032")
            self.metadata(root / "resources", "sample.xdremux")
            result = tone.write_asymmetry(apple, product, root / "resources", root / "comparison")
            self.assertEqual(result["status"], "completed")
            self.assertTrue(result["trusted"])
            self.assertTrue(result["curves"]["xdremux"][0]["inside_apple_range"])
            (root / "resources/sample.xdremux/keyd.bin").write_bytes(b"\x00")
            result = tone.write_asymmetry(apple, product, root / "resources", root / "damaged-comparison")
            self.assertEqual(result["status"], "failed")
            self.assertFalse(result["trusted"])

    def test_malformed_sweep_and_duplicate_rows_leave_untrusted_reports(self):
        for failure in ("json", "duplicate", "metric"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                sanity = self.make_sweep(root / "renders")
                path = root / "renders/tone-sweep.json"
                rows = json.loads(path.read_text())
                if failure == "json":
                    path.write_text("{")
                elif failure == "duplicate":
                    path.write_text(json.dumps(rows + [rows[0]]))
                else:
                    rows[0]["max_abs"] = float("nan")
                    path.write_text(json.dumps(rows))
                report = tone.analyze(sanity, root / "renders", root / "report")
                self.assertFalse(report["trusted"])
                self.assertEqual(report["status"], "failed")

    def test_consumer_zero_exit_with_extractor_failure_retains_stderr_and_fails(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            research = root / "consumer"
            script = research / "consumer-model/harness/run_tone_sweep.py"
            script.parent.mkdir(parents=True)
            script.write_text("from pathlib import Path\nimport json,sys\n"
                              "out=Path(sys.argv[1]); case=out/'sample.xdremux'; case.mkdir()\n"
                              "(case/'extract.log').write_text(json.dumps({'source_sha256':'abc','primary_id':1,'items':{'pixels':[1,2,3]}})+'\\nTraceback: extractor first fault\\nValueError: linked item missing\\n')\n"
                              "(case/'pixels.json').write_text('{\"pixels\":[1,2,3]}')\n"
                              "(out/'tone-sweep.json').write_text(json.dumps([{'photo':'sample.xdremux','error':'extract_inputs returncode 1'}]))\n")
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                receipt = tone.run_consumer(research, [Path("sample.xdremux.heic")], root / "out")
            self.assertEqual(receipt["status"], "failed")
            self.assertEqual(receipt["returncode"], 0)
            self.assertIn("ValueError: linked item missing", "\n".join(p.read_text() for p in (root / "out/diagnostics").glob("*.log")))
            self.assertNotIn("pixels", "\n".join(p.read_text() for p in (root / "out/diagnostics").glob("*.log")))
            self.assertFalse((root / "out/diagnostics/pixels.json").exists())
            self.assertTrue(any(log["omitted_inventory_lines"] == 1 for log in receipt["diagnostics"]))
            self.assertEqual(json.loads((root / "out/consumer-run.json").read_text())["status"], "failed")

    def test_consumer_process_failure_retains_both_streams(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            research = root / "consumer"
            script = research / "consumer-model/harness/run_tone_sweep.py"
            script.parent.mkdir(parents=True)
            script.write_text("import sys\nprint('consumer started')\nprint('native tool failed first',file=sys.stderr)\nsys.exit(7)\n")
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                receipt = tone.run_consumer(research, [Path("sample.xdremux.heic")], root / "out")
            self.assertEqual(receipt["returncode"], 7)
            self.assertEqual(receipt["status"], "failed")
            self.assertIn("native tool failed first", (root / "out/diagnostics/consumer.stderr.log").read_text())
            self.assertIn("consumer started", (root / "out/diagnostics/consumer.stdout.log").read_text())

    def test_native_facts_dictionary_stays_local_while_stderr_and_exception_survive(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            research = root / "consumer"
            script = research / "consumer-model/harness/run_tone_sweep.py"
            script.parent.mkdir(parents=True)
            script.write_text("from pathlib import Path\nimport json,sys\n"
                              "out=Path(sys.argv[1]); case=out/'sample.xdremux'; case.mkdir()\n"
                              "tdir=case/'tone-0'; tdir.mkdir()\n"
                              "(case/'tone-0.log').write_text('Trace exception: native first fault\\n{\\n raw_pixel_dictionary = SECRET_PIXELS;\\n}\\n')\n"
                              "(tdir/'facts.json').write_text(json.dumps({'exception':'native first fault','pixels':['SECRET_PIXELS']}))\n"
                              "(out/'tone-sweep.json').write_text(json.dumps([{'photo':'sample.xdremux','tone':0,'returncode':6,'error':'native failed'}]))\n")
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                receipt = tone.run_consumer(research, [Path("sample.xdremux.heic")], root / "out")
            exported = "\n".join(p.read_text() for p in (root / "out/diagnostics").glob("*.log")) + json.dumps(receipt)
            self.assertIn("Trace exception: native first fault", exported)
            self.assertNotIn("SECRET_PIXELS", exported)
            self.assertIn("native first fault", json.dumps(receipt.get("native_failures")))

    def test_finish_status_is_observed_without_guessing_its_success_value(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            sanity = self.make_sweep(root / "renders")
            path = root / "renders/sample.xdremux/tone-0/renderer-0/trace.json"
            trace = json.loads(path.read_text())
            trace["finish_status"] = 1
            path.write_text(json.dumps(trace))
            report = tone.analyze(sanity, root / "renders", root / "report")
            self.assertTrue(report["trusted"])
            self.assertEqual(report["photos"][0]["native_statuses"][0.0]["finish_status"], 1)

    def test_consumer_timeout_is_a_failure_with_a_reaped_process(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            research = root / "consumer"
            script = research / "consumer-model/harness/run_tone_sweep.py"
            script.parent.mkdir(parents=True)
            script.write_text("import time\ntime.sleep(30)\n")
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                receipt = tone.run_consumer(research, [Path("sample.xdremux.heic")], root / "out", timeout=0.05)
            self.assertEqual(receipt["status"], "failed")
            self.assertIsNotNone(receipt["returncode"])
            self.assertNotEqual(receipt["returncode"], 0)


if __name__ == "__main__":
    unittest.main()
