from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path

from xdremux_py.cli import _default_batch_candidates, _validate_unique_normal_plan
from xdremux_py.live_photo_batch import (
    StateWriter,
    load_state,
    planned_output_image,
    provenance_allows_reuse,
    source_signature,
    state_path,
)


class PythonLivePhotoBatchReliabilityTests(unittest.TestCase):
    def test_duplicate_basenames_preserve_relative_directories_across_subset_rerun(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            a = root / "A" / "IMG.jpg"
            b = root / "B" / "IMG.jpg"
            a.parent.mkdir(parents=True)
            b.parent.mkdir(parents=True)
            a.write_bytes(b"a")
            b.write_bytes(b"b")
            self.assertEqual(planned_output_image(a, root, output), output / "A" / "IMG.motion.heic")
            self.assertEqual(planned_output_image(b, root, output), output / "B" / "IMG.motion.heic")
            self.assertEqual(planned_output_image(b, root, output), planned_output_image(b, root, output))

    def test_jpeg_batch_output_does_not_use_sibling_source_heic_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            source = root / "A" / "IMG.jpg"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"jpeg")
            sibling_heic = root / "A" / "IMG.heic"
            sibling_heic.write_bytes(b"heic")
            planned = planned_output_image(source, root, root)
            self.assertNotEqual(planned, sibling_heic)
            self.assertEqual(planned, root / "A" / "IMG.motion.heic")

    def test_same_stem_jpeg_and_heif_inputs_have_distinct_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            jpeg = root / "A" / "IMG.jpg"
            heif = root / "A" / "IMG.heic"
            jpeg.parent.mkdir(parents=True)
            jpeg.write_bytes(b"jpeg")
            heif.write_bytes(b"heif")
            self.assertEqual(planned_output_image(jpeg, root, output), output / "A" / "IMG.motion.heic")
            self.assertEqual(planned_output_image(heif, root, output), output / "A" / "IMG.live.heic")
            self.assertNotEqual(planned_output_image(jpeg, root, output), planned_output_image(heif, root, output))

    def test_absolute_input_root_does_not_leak_into_output_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            first_root = base / "root-one"
            second_root = base / "root-two"
            output = base / "output"
            first = first_root / "A" / "IMG.jpg"
            second = second_root / "A" / "IMG.jpg"
            first.parent.mkdir(parents=True)
            second.parent.mkdir(parents=True)
            first.write_bytes(b"first")
            second.write_bytes(b"second")
            self.assertEqual(planned_output_image(first, first_root, output), planned_output_image(second, second_root, output))

    def test_heif_motion_photo_uses_readable_live_filename(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            source = root / "Trips" / "same.heic"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"heif")
            output = planned_output_image(source, root, Path(tmp) / "output")
            self.assertEqual(output, Path(tmp) / "output" / "Trips" / "same.live.heic")

    def test_source_signature_uses_size_and_mtime(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source.jpg"
            source.write_bytes(b"AAAA")
            before = source_signature(source)
            new_mtime = before.mtime_ns + 10_000_000_000
            os.utime(source, ns=(new_mtime, new_mtime))
            after = source_signature(source)
            self.assertEqual(before.size, after.size)
            self.assertNotEqual(before.mtime_ns, after.mtime_ns)

    def test_state_reuse_requires_source_metadata_outputs_and_pair_match(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            root.mkdir()
            output.mkdir()
            source = root / "IMG.jpg"
            source.write_bytes(b"source")
            image = planned_output_image(source, root, output)
            video = image.with_suffix(".mov")
            signature = source_signature(source)
            checkpoint = output / ".state.jsonl"
            with StateWriter(checkpoint) as writer:
                writer.append(source=source, input_root=root, image=image, video=video, status="success", signature=signature, asset_identifier="ASSET-1")
            prior = load_state(checkpoint)[str(source.resolve())]
            self.assertTrue(provenance_allows_reuse(prior, signature, image, video, lambda i, v, identifier: identifier == "ASSET-1"))
            self.assertFalse(provenance_allows_reuse(prior, signature, image, video, lambda i, v, identifier: False))

    def test_pr18_camel_case_checkpoint_migrates_without_hash_dependency(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            root.mkdir()
            output.mkdir()
            source = root / "IMG.jpg"
            source.write_bytes(b"source")
            signature = source_signature(source)
            image = planned_output_image(source, root, output)
            video = image.with_suffix(".mov")
            checkpoint = output / ".state.jsonl"
            checkpoint.write_text(json.dumps({"kind":"item","inputPath":str(source.resolve()),"sourceRelativePath":"IMG.jpg","outputImagePath":str(image.resolve()),"outputVideoPath":str(video.resolve()),"status":"success","inputSize":signature.size,"inputMtimeNs":signature.mtime_ns,"inputSHA256":"deadbeef","assetIdentifier":"ASSET-OLD","error":None}) + "\n", encoding="utf-8")
            prior = load_state(checkpoint)[str(source.resolve())]
            self.assertTrue(prior.matches_source(signature))
            self.assertEqual(prior.asset_identifier, "ASSET-OLD")

    def test_new_python_checkpoint_is_runtime_local_not_swift_wire_schema(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            root.mkdir()
            output.mkdir()
            source = root / "IMG.jpg"
            source.write_bytes(b"source")
            signature = source_signature(source)
            image = planned_output_image(source, root, output)
            video = image.with_suffix(".mov")
            checkpoint = output / ".state.jsonl"
            with StateWriter(checkpoint) as writer:
                writer.append(source=source, input_root=root, image=image, video=video, status="success", signature=signature, asset_identifier="ASSET-1")
            records = [json.loads(line) for line in checkpoint.read_text(encoding="utf-8").splitlines()]
            self.assertEqual(records[0]["schema_version"], 1)
            self.assertEqual(records[1]["input_path"], str(source.resolve()))
            self.assertNotIn("inputPath", records[1])
            self.assertNotIn("inputSHA256", records[1])

    def test_custom_checkpoint_path_stays_separate_from_legacy_batch_checkpoint(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "output"
            requested = Path(tmp) / "state.jsonl"
            self.assertEqual(state_path(output, requested), requested.parent / "state.jsonl.motion-photo")

    def test_hidden_publication_temp_is_never_discovered_as_user_input(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            visible = root / "IMG.heic"
            hidden_temp = root / ".IMG.abc123.tmp.heic"
            visible.write_bytes(b"visible")
            hidden_temp.write_bytes(b"publication-temp")
            self.assertEqual(_default_batch_candidates(root), [visible])

    def test_normal_output_collision_fails_before_any_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            first = root / "A" / "IMG.heic"
            second = root / "B" / "IMG.heic"
            output = root / "output" / "IMG.heic"
            with self.assertRaisesRegex(ValueError, "planned ProXDR output collision"):
                _validate_unique_normal_plan([(first, output), (second, output)])


if __name__ == "__main__":
    unittest.main()
