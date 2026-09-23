"""Historical records are immutable hypotheses, not product defaults.

Raw blob/SHA-256 provenance was verified against archived Git objects during
convergence; these portable tests guard the curated decoded payloads without
requiring an unshallow clone or private firmware.
"""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "research/texture_styles/evidence"


class TextureEvidenceProvenanceTests(unittest.TestCase):
    def test_historical_contracts_retain_expected_payloads_and_explicit_status(self):
        records = json.loads((EVIDENCE / "historical-contracts.json").read_text())
        self.assertEqual(records["schema"], 1)
        self.assertIn("not newly verified", records["warning"])
        expected = EXPECTED_PAYLOADS
        self.assertEqual({r["source_path"]: r["source_sha256"] for r in records["artifacts"]},
                         {p: pair[0] for p, pair in expected.items()})
        for record in records["artifacts"]:
            self.assertEqual(record["status"], "historical-hypothesis-not-product")
            for field in ("source_commit", "source_blob"):
                self.assertRegex(record[field], r"^[a-f0-9]{40}$")
            canonical = json.dumps(record["evidence"], sort_keys=True, separators=(",", ":"),
                                   ensure_ascii=False).encode()
            self.assertEqual(hashlib.sha256(canonical).hexdigest(), expected[record["source_path"]][1])

    def test_native_standard_matrix_matches_the_original_raw_file(self):
        raw = (EVIDENCE / "native-standard-matrix.json").read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),
                         "2bb45a9273a270802da6148b21431666a03dd8d0296915532e396bf197277d4d")
        artifacts = json.loads((EVIDENCE / "historical-contracts.json").read_text())["artifacts"]
        original = next(x for x in artifacts if x["source_path"].endswith("native_standard_contract.json"))
        self.assertEqual(json.loads(raw), original["evidence"])


EXPECTED_PAYLOADS = {'scripts/research/texture_style_ios27_rc_contract.json': ('a06616b94c1a6830b4bea5b8af07b73e18e9f413744326db9bc28415d2884ff4',
                                                           'bd3a3da93c809ca8e3c04c329a0e872688369e0678b70ada8b080d1b51f90f8f'),
 'scripts/research/texture_style_makernote_field_writer_selftest.json': ('8628b53e25944c8115d48d4b20542b2b8b2d23527c4fac815a3db7336bcfd582',
                                                                         'fe667e00e9368a191c59651bac72c8e7afab6c91a83c318bcc4933199c603c19'),
 'scripts/research/texture_style_native_standard_contract.json': ('2bb45a9273a270802da6148b21431666a03dd8d0296915532e396bf197277d4d',
                                                                  '8386c82aebce747d91b8ebae442e89af3041fb933ed65c029eba22628d895f04'),
 'scripts/research/texture_style_ps3_portrait_contract.json': ('58be0e6f11c191ea4c8bb34fbcf20fad2c7bb01e64f19347b721a93272638dd9',
                                                               '894be23b8c8a89d3d50696e987295f329f22d26dadcdc12246307420478f16b3'),
 'scripts/research/texture_style_tag84_v4_contract.json': ('ec4a94baa90e143045eb95f991156cd96d5d36c2b236a8b2706c8f6ac7c2fbe0',
                                                           'eaf0c1fbf4e01fddcdebf60d8a9d748b1c26dc2b10a671af67be0f6ccaec2576'),
 'scripts/research/texture_style_tag84_v4_full_numeric_contract.json': ('bfb467ee50262581a5bcbe26baf3eb9dca251a52ad133ef73006e3056b9abe49',
                                                                        'f3a8d7b5e5d7e86cc78d33d903c1e34dc9e16378aa5daf05e8ccd88211e2f4d8'),
 'scripts/research/texture_style_tag84_v5_numeric_contract.json': ('575a90f4acd571339976d5bbd802b3e16cd9a3e998ee8510df50186b80f0cd13',
                                                                   'fdfc27c8264147821080912579b3c969079934c9f0ac9f553f64a6486046d909')}

if __name__ == "__main__":
    unittest.main()
