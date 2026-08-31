from __future__ import annotations

import struct
import tempfile
import unittest
from pathlib import Path

from xdremux_py.motion_photo import parse_motion_photo


def _box(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I4s", len(payload) + 8, kind) + payload


def _fake_video() -> bytes:
    return _box(b"ftyp", b"isom" + struct.pack(">I", 0)) + _box(b"mdat", b"payload")


class PythonOppoMotionPhotoTests(unittest.TestCase):
    def test_minus_one_xmp_timestamp_falls_back_to_lpex_cover_frame(self):
        video = _fake_video()
        xmp = f'''<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
 OpCamera:VideoLength="{len(video)}"
 GCamera:MotionPhotoPresentationTimestampUs="-1"/></rdf:RDF></x:xmpmeta>'''.encode()
        lpex = b'lpexLivePhotoExtension {"version":0,"coverFramePts":777777}'
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "oppo-sentinel.jpg"
            path.write_bytes(b"\xff\xd8" + xmp + lpex + b"\xff\xd9" + video)
            asset = parse_motion_photo(path)
            self.assertIsNotNone(asset)
            assert asset is not None
            self.assertEqual(asset.presentation_timestamp_us, 777777)
            self.assertEqual(asset.presentation_source, "oppoCoverFrame")


if __name__ == "__main__":
    unittest.main()
