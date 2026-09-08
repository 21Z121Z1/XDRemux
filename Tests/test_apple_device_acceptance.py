from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from scripts.check_apple_device_acceptance import AcceptanceError, validate_acceptance


HEAD = "0123456789abcdef0123456789abcdef01234567"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class AppleDeviceAcceptanceTests(unittest.TestCase):
    def make_bundle(self) -> tuple[tempfile.TemporaryDirectory[str], Path, dict[str, object]]:
        temporary = tempfile.TemporaryDirectory(prefix="xdremux-device-acceptance-")
        root = Path(temporary.name)
        outputs: dict[str, object] = {}
        for key, filename, payload in (
            ("portrait", "portrait.heic", b"portrait"),
            ("photographic_styles", "photographic-styles.heic", b"styles"),
        ):
            (root / filename).write_bytes(payload)
            outputs[key] = {
                "path": filename,
                "size_bytes": len(payload),
                "sha256": digest(payload),
                "device_acceptance": {
                    "tested": True,
                    "device": "iPhone test device",
                    "os_version": "iOS test build",
                    "photos_imported": True,
                    "feature_recognized": True,
                    "editing_ui_available": True,
                    "edited_render_matches_expected_behavior": True,
                    "revert_or_round_trip_succeeds": True,
                    "notes": None,
                },
            }
        manifest = {
            "manifest_schema_version": 1,
            "head": HEAD,
            "outputs": outputs,
        }
        manifest_path = root / "manifest.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        return temporary, manifest_path, manifest

    def test_complete_exact_head_sha_bound_evidence_passes(self) -> None:
        temporary, manifest_path, _ = self.make_bundle()
        with temporary:
            validate_acceptance(manifest_path, HEAD)

    def test_incomplete_device_observation_fails_closed(self) -> None:
        temporary, manifest_path, manifest = self.make_bundle()
        with temporary:
            outputs = manifest["outputs"]
            assert isinstance(outputs, dict)
            portrait = outputs["portrait"]
            assert isinstance(portrait, dict)
            acceptance = portrait["device_acceptance"]
            assert isinstance(acceptance, dict)
            acceptance["editing_ui_available"] = None
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(AcceptanceError, "editing_ui_available"):
                validate_acceptance(manifest_path, HEAD)

    def test_asset_drift_invalidates_device_evidence(self) -> None:
        temporary, manifest_path, _ = self.make_bundle()
        with temporary:
            (manifest_path.parent / "portrait.heic").write_bytes(b"changed!")
            with self.assertRaisesRegex(AcceptanceError, "size changed|SHA-256 changed"):
                validate_acceptance(manifest_path, HEAD)

    def test_head_mismatch_invalidates_device_evidence(self) -> None:
        temporary, manifest_path, _ = self.make_bundle()
        with temporary:
            with self.assertRaisesRegex(AcceptanceError, "not exact-head"):
                validate_acceptance(manifest_path, "ffffffffffffffffffffffffffffffffffffffff")

    def test_asset_path_cannot_escape_bundle(self) -> None:
        temporary, manifest_path, manifest = self.make_bundle()
        with temporary:
            outputs = manifest["outputs"]
            assert isinstance(outputs, dict)
            portrait = outputs["portrait"]
            assert isinstance(portrait, dict)
            portrait["path"] = "../portrait.heic"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(AcceptanceError, "stay inside"):
                validate_acceptance(manifest_path, HEAD)


if __name__ == "__main__":
    unittest.main()
