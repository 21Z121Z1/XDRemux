import unittest

import numpy as np

from xdremux_py.public_style_pretraining import (
    commons_candidates,
    synthetic_affine_pair,
)


class PublicStylePretrainingTests(unittest.TestCase):
    def test_commons_candidates_enforce_license_and_raster_contract(self) -> None:
        def page(title: str, license_name: str, mime: str = "image/jpeg") -> dict:
            return {
                "title": title,
                "imageinfo": [
                    {
                        "mime": mime,
                        "thumburl": f"https://example.test/{title}",
                        "descriptionurl": f"https://example.test/wiki/{title}",
                        "sha1": "a" * 40,
                        "extmetadata": {
                            "LicenseShortName": {"value": license_name},
                            "LicenseUrl": {"value": "https://license.test"},
                            "Artist": {"value": "Photographer"},
                        },
                    }
                ],
            }

        payload = {
            "query": {
                "pages": [
                    page("allowed.jpg", "CC BY 4.0"),
                    page("legacy.jpg", "CC BY-NC 4.0"),
                    page("video.webm", "CC0", "video/webm"),
                ]
            }
        }
        actual = commons_candidates(payload, "test camera", 5)
        self.assertEqual([item["title"] for item in actual], ["allowed.jpg"])
        self.assertEqual(actual[0]["license"], "CC BY 4.0")

    def test_synthetic_affine_key_exactly_recovers_clean_target(self) -> None:
        source = np.random.default_rng(7).random((3, 256, 256), dtype=np.float32)
        styled, clean, key1 = synthetic_affine_pair(source, np.random.default_rng(8))
        coefficients = key1.mean(axis=(0, 1, 2))
        red, green, blue = styled
        terms = np.stack(
            (
                np.ones_like(red), red, green, blue, red * red, red * green,
                red * blue, green * green, green * blue, blue * blue,
            )
        )
        recovered = np.einsum("thw,tc->chw", terms, coefficients)
        self.assertLess(float(np.max(np.abs(recovered - clean))), 2e-5)


if __name__ == "__main__":
    unittest.main()
