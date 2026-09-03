from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "crates" / "xdremux-runtime" / "src"


class RustArtifactLifecycleTests(unittest.TestCase):
    def test_portrait_transaction_stages_validates_and_publishes_atomically(self) -> None:
        source = (RUNTIME / "lib.rs").read_text(encoding="utf-8")
        for marker in (
            "tempdir_in(parent)",
            "validate_gain_map_structure",
            "AtomicFilePublisher::new(output.to_path_buf())",
            "publisher.publish_bytes(bytes)",
        ):
            self.assertIn(marker, source)
        self.assertNotIn("publish_bytes(carrier", source)

    def test_styles_transaction_uses_the_same_publication_invariant(self) -> None:
        source = (RUNTIME / "apple_styles.rs").read_text(encoding="utf-8")
        for marker in (
            "tempdir_in(parent)",
            "assemble_photographic_styles_heif",
            "validate_gain_map_structure",
            "AtomicFilePublisher::new(output.to_path_buf())",
            "publisher.publish_bytes(assembled)",
        ):
            self.assertIn(marker, source)
        self.assertNotIn("XDRemuxAppleFeatures", source)
        self.assertNotIn("xdremux_py", source)

    def test_portable_file_transaction_has_no_framework_implementation_branch(self) -> None:
        source = (RUNTIME / "lib.rs").read_text(encoding="utf-8")
        self.assertIn("execute_conversion", source)
        self.assertIn("AtomicFilePublisher", source)
        self.assertNotIn("Command::new", source)
        self.assertNotIn("swift", source.lower())
        self.assertNotIn("python", source.lower())


if __name__ == "__main__":
    unittest.main()
