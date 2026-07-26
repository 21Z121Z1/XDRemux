"""Regression contracts for the Python converter's failure handling.

Each case here reproduces a defect that reached main: a crash on a valid photo,
a truncated file left behind by a failed write, and an unbounded parser loop on
a malformed container.
"""

import os
import struct
import tempfile
import unittest
from unittest import mock

import numpy as np

from xdremux.python import gainmap, heif_io, isobmff_patch


class GainMapEdgeCaseTests(unittest.TestCase):
    def test_unit_edr_scale_yields_a_zero_gain_map(self) -> None:
        # edr_scale == 1.0 is a legitimate early-LHDR photo carrying no HDR
        # gain. It used to divide by zero and abort the conversion.
        mask = np.full((8, 8), 128, dtype=np.uint8)
        result = gainmap.reconstruct(mask, 1.0, 2.0)

        self.assertEqual(result.shape, mask.shape)
        self.assertEqual(result.dtype, np.uint8)
        self.assertTrue(np.all(result == 0))

    def test_ordinary_edr_scale_still_produces_gain(self) -> None:
        mask = np.full((8, 8), 200, dtype=np.uint8)
        result = gainmap.reconstruct(mask, 2.5, 2.0)

        self.assertEqual(result.shape, mask.shape)
        self.assertTrue(np.any(result != 0))


class AtomicWriteTests(unittest.TestCase):
    def test_successful_write_replaces_the_previous_content(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "photo.heic")
            with open(path, "wb") as handle:
                handle.write(b"original")

            heif_io.atomic_write_bytes(path, b"converted payload")

            with open(path, "rb") as handle:
                self.assertEqual(handle.read(), b"converted payload")
            self.assertEqual(os.listdir(directory), ["photo.heic"])

    def test_failed_write_leaves_the_original_intact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "photo.heic")
            with open(path, "wb") as handle:
                handle.write(b"original")

            def explode(*_args: object, **_kwargs: object) -> None:
                raise OSError("simulated disk failure")

            with mock.patch.object(heif_io.os, "replace", explode):
                with self.assertRaises(OSError):
                    heif_io.atomic_write_bytes(path, b"converted payload")

            with open(path, "rb") as handle:
                self.assertEqual(handle.read(), b"original")
            # The staging file must not survive a failed write.
            self.assertEqual(os.listdir(directory), ["photo.heic"])


class BoxWalkerTests(unittest.TestCase):
    def test_zero_largesize_is_rejected_instead_of_looping(self) -> None:
        # size==1 selects the 64-bit largesize; a largesize of 0 cannot advance
        # the cursor and used to spin forever.
        data = struct.pack(">I", 1) + b"free" + struct.pack(">Q", 0)

        with self.assertRaises(ValueError):
            list(isobmff_patch._boxes(data, 0, len(data)))

    def test_truncated_largesize_header_is_rejected(self) -> None:
        data = struct.pack(">I", 1) + b"free" + b"\x00\x00\x00"

        with self.assertRaises(ValueError):
            list(isobmff_patch._boxes(data, 0, len(data)))

    def test_well_formed_boxes_are_walked(self) -> None:
        payload = b"\x01\x02\x03\x04"
        data = struct.pack(">I", 8 + len(payload)) + b"mdat" + payload

        boxes = list(isobmff_patch._boxes(data, 0, len(data)))

        self.assertEqual([box[0] for box in boxes], ["mdat"])
        self.assertEqual(data[boxes[0][1]:boxes[0][2]], payload)


if __name__ == "__main__":
    unittest.main()
