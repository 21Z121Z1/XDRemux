from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo_batch import (
    StateWriter,
    load_state,
    planned_output_image,
    provenance_allows_reuse,
    source_signature,
    state_path,
)


class PythonLivePhotoBatchReliabilityTests(unittest.TestCase):
    def test_duplicate_basenames_have_stable_distinct_outputs_across_subset_rerun(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            output = Path(tmp) / "output"
            a = root / "A" / "IMG.jpg"
            b = root / "B" / "IMG.jpg"
            a.parent.mkdir(parents=True)
            b.parent.mkdir(parents=True)
            a.write_bytes(b"a")
            b.write_bytes(b"b")

            a_output = planned_output_image(a, root, output)
            b_output = planned_output_image(b, root, output)
            b_subset_output = planned_output_image(b, root, output)

            self.assertNotEqual(a_output, b_output)
            self.assertEqual(b_output, b_subset_output)
            self.assertTrue(a_output.name.startswith("IMG~"))
            self.assertTrue(b_output.name.startswith("IMG~"))

    def test_heif_motion_photo_uses_live_namespace_and_stable_token(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "input"
            root.mkdir()
            source = root / "same.heic"
            source.write_bytes(b"heif")
            output = planned_output_image(source, root, Path(tmp) / "output")
            self.assertTrue(output.name.startswith("same.live~"))
            self.assertTrue(output.name.endswith(".heic"))

    def test_content_hash_detects_change_even_when_size_and_mtime_are_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source.jpg"
            source.write_bytes(b"AAAA")
            before = source_signature(source)
            source.write_bytes(b"BBBB")
            os.utime(source, ns=(before.mtime_ns, before.mtime_ns))
            after = source_signature(source)
            self.assertEqual(before.size, after.size)
            self.assertEqual(before.mtime_ns, after.mtime_ns)
            self.assertNotEqual(before.sha256, after.sha256)

    def test_state_requires_source_hash_outputs_asset_identifier_and_pair_match(self):
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
                writer.append(
                    source=source,
                    input_root=root,
                    image=image,
                    video=video,
                    status="success",
                    signature=signature,
                    asset_identifier="ASSET-1",
                )

            prior = load_state(checkpoint)[str(source.resolve())]
            self.assertTrue(
                provenance_allows_reuse(
                    prior,
                    signature,
                    image,
                    video,
                    lambda i, v, identifier: identifier == "ASSET-1",
                )
            )
            self.assertFalse(
                provenance_allows_reuse(
                    prior,
                    signature,
                    image,
                    video,
                    lambda i, v, identifier: False,
                )
            )
            source.write_bytes(b"SOURCE")
            changed = source_signature(source)
            self.assertFalse(
                provenance_allows_reuse(
                    prior,
                    changed,
                    image,
                    video,
                    lambda i, v, identifier: True,
                )
            )

    def test_python_writer_uses_swift_checkpoint_wire_schema(self):
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
                writer.append(
                    source=source,
                    input_root=root,
                    image=image,
                    video=video,
                    status="success",
                    signature=signature,
                    asset_identifier="ASSET-1",
                )

            records = [json.loads(line) for line in checkpoint.read_text(encoding="utf-8").splitlines()]
            self.assertEqual(records[0]["schemaVersion"], 2)
            item = records[1]
            self.assertEqual(item["inputPath"], str(source.resolve()))
            self.assertEqual(item["sourceRelativePath"], "IMG.jpg")
            self.assertEqual(item["inputSHA256"], signature.sha256)
            self.assertEqual(item["assetIdentifier"], "ASSET-1")
            self.assertNotIn("input_path", item)
            self.assertNotIn("input_sha256", item)

    def test_custom_checkpoint_path_matches_swift_motion_photo_suffix_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "output"
            requested = Path(tmp) / "state.jsonl"
            self.assertEqual(
                state_path(output, requested),
                requested.parent / "state.jsonl.motion-photo",
            )

    def test_schema_one_style_entry_without_digest_is_not_trusted(self):
        with tempfile.TemporaryDirectory() as tmp:
            checkpoint = Path(tmp) / "state.jsonl"
            checkpoint.write_text(
                '{"kind":"header","schema_version":1}\n'
                '{"kind":"item","input_path":"/tmp/a.jpg","output_image_path":"/tmp/a.heic",'
                '"output_video_path":"/tmp/a.mov","status":"success","input_size":1,'
                '"input_mtime_ns":1,"error":null}\n',
                encoding="utf-8",
            )
            self.assertEqual(load_state(checkpoint), {})


if __name__ == "__main__":
    unittest.main()
