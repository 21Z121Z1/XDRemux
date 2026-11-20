from pathlib import Path
import re
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

    def test_styles_transaction_validates_the_final_artifact_before_publication(self) -> None:
        source = (RUNTIME / "apple_styles.rs").read_text(encoding="utf-8")
        # Still and Live Photo conversion share the assembly and final
        # validation. Follow the caller's data flow across that boundary.
        conversion = source.split("pub(crate) fn convert_file(", 1)[1].split("\nstruct StyleCarrier", 1)[0]
        assembly = source.split("\nfn assemble_on_base(", 1)[1].split("\npub(super) fn validate_maker_apple_properties(", 1)[0]
        for marker in (
            "tempdir_in(parent)",
            "AtomicFilePublisher::new(output.to_path_buf())",
        ):
            self.assertIn(marker, conversion)
        self.assertIn("assemble_photographic_styles_heif", assembly)

        returned = re.search(r"Ok\((?P<artifact>\w+)\)\s*}\s*$", assembly)
        self.assertIsNotNone(returned, "Styles assembly must return the validated file")
        artifact = returned.group("artifact")
        validation = assembly.index(f"validate_gain_map_structure(&{artifact})")
        self.assertLess(assembly.index("crate::apple_texture::generate("), validation)
        self.assertLess(validation, returned.start())
        self.assertLess(assembly.index("validate_maker_apple_properties("), returned.start())

        validated = re.search(
            r"let (?P<artifact>\w+) = assemble_on_base\([\s\S]*?\)\?;", conversion
        )
        self.assertIsNotNone(validated, "The conversion must propagate validation errors")
        result = conversion.index(f"Ok({validated.group('artifact')})", validated.end())
        propagated = conversion.index("let assembled = result?;", result)
        publication = conversion.index("publisher.publish_bytes(assembled)", propagated)
        self.assertLess(result, propagated)
        self.assertLess(propagated, publication, "Styles must propagate validation errors before publishing")
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
