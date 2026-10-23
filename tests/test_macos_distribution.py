"""Portable contracts for package relocation and reproducible archives."""
from pathlib import Path
import io
import os
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from scripts.distribution import macos, smoke_macos


class MacOSDistributionTests(unittest.TestCase):
    @staticmethod
    def package_fixture(root):
        package = root / "package"
        for relative in ("bin/xdremux", "libexec/xdremux-apple-adapter",
                         "lib/libheif.1.dylib", "lib/libde265.0.dylib",
                         "lib/libx265.1.dylib", "lib/libsharpyuv.0.dylib"):
            path = package / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"portable fixture; Mach-O observations are injected")
            path.chmod(0o755)
        return package

    def test_product_entrypoints_reject_intel_before_creating_output(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)
            commands = [
                ["build_native.py", "--arch", "x86_64", "--work", str(scratch / "native")],
                ["macos.py", "audit", "--arch", "x86_64", "--package", str(scratch / "package")],
                ["smoke_macos.py", "--arch", "x86_64", "--archive", str(scratch / "archive.tar.gz"),
                 "--fixtures", str(root / "fixtures"), "--expected-head", "0" * 40,
                 "--output", str(scratch / "evidence")],
            ]
            for command in commands:
                with self.subTest(script=command[0]):
                    result = subprocess.run([sys.executable, str(root / "scripts/distribution" / command[0]),
                                             *command[1:]], capture_output=True, text=True)
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("invalid choice: 'x86_64'", result.stderr)
            self.assertEqual(list(scratch.iterdir()), [])

    def test_package_audit_requires_native_arm64_macos27_host(self):
        facts = {"architectures": ["arm64"], "minimum_os": "27.0", "dependencies": [],
                 "rpaths": [], "install_id": None, "dyld_environment": []}
        for system, arch, version in [("Linux", "arm64", ""), ("Darwin", "x86_64", "27.0"),
                                      ("Darwin", "arm64", "26.4")]:
            with self.subTest(system=system, arch=arch, version=version), tempfile.TemporaryDirectory() as temporary:
                package = self.package_fixture(Path(temporary))
                with patch.object(macos.platform, "system", return_value=system), \
                     patch.object(macos.platform, "machine", return_value=arch), \
                     patch.object(macos.platform, "mac_ver", return_value=(version, (), "")), \
                     patch.object(macos, "inspect", return_value=facts), patch.object(macos, "call"):
                    with self.assertRaisesRegex(ValueError, "arm64.*macOS 27"):
                        macos.audit(package, "arm64")

    def test_package_audit_rejects_wrong_architecture_or_deployment_target(self):
        baseline = {"architectures": ["arm64"], "minimum_os": "27.0", "dependencies": [],
                    "rpaths": [], "install_id": None, "dyld_environment": []}
        cases = [{"architectures": ["x86_64"]}, {"architectures": ["arm64", "x86_64"]},
                 *[{"minimum_os": value} for value in (None, "15.0", "26.0", "28.0")]]
        with tempfile.TemporaryDirectory() as temporary:
            package = self.package_fixture(Path(temporary))
            with patch.object(macos.platform, "system", return_value="Darwin"), \
                 patch.object(macos.platform, "machine", return_value="arm64"), \
                 patch.object(macos.platform, "mac_ver", return_value=("27.0.1", (), "")), \
                 patch.object(macos, "call"), patch.object(macos, "inspect", return_value=baseline):
                report = macos.audit(package, "arm64")
                self.assertEqual(len(report["files"]), 6)
                for changed in cases:
                    with self.subTest(changed=changed), patch.object(macos, "inspect", return_value={**baseline, **changed}):
                        with self.assertRaisesRegex(ValueError, "architecture|deployment target"):
                            macos.audit(package, "arm64")

    def test_all_dynamic_load_kinds_and_environment_are_observed(self):
        text = """Load command 0
          cmd LC_ID_DYLIB
         name @rpath/libheif.1.dylib (offset 24)
Load command 1
          cmd LC_LOAD_DYLIB
         name /usr/lib/libSystem.B.dylib (offset 24)
Load command 2
          cmd LC_LOAD_WEAK_DYLIB
         name @loader_path/liboptional.dylib (offset 24)
Load command 3
          cmd LC_REEXPORT_DYLIB
         name @loader_path/libexport.dylib (offset 24)
Load command 4
          cmd LC_LOAD_UPWARD_DYLIB
         name @loader_path/libup.dylib (offset 24)
Load command 5
          cmd LC_LAZY_LOAD_DYLIB
         name @loader_path/liblazy.dylib (offset 24)
Load command 6
          cmd LC_DYLD_ENVIRONMENT
         name DYLD_LIBRARY_PATH=/opt/homebrew/lib (offset 12)
Load command 7
          cmd LC_RPATH
         path /opt/homebrew/lib (offset 12)
Load command 8
          cmd LC_BUILD_VERSION
        minos 15.0
"""
        facts = macos.parse_load_commands(text)
        self.assertEqual(len(facts["dependencies"]), 5)
        self.assertIn("@loader_path/libexport.dylib", facts["dependencies"])
        self.assertEqual(facts["install_id"], "@rpath/libheif.1.dylib")
        self.assertEqual(facts["dyld_environment"], ["DYLD_LIBRARY_PATH=/opt/homebrew/lib"])
        self.assertEqual(facts["rpaths"], ["/opt/homebrew/lib"])
        self.assertEqual(facts["minimum_os"], "15.0")

    def test_loader_relative_reference_cannot_escape_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            (package / "bin").mkdir(parents=True)
            (package / "lib").mkdir()
            image = package / "bin/xdremux"
            library = package / "lib/libheif.1.dylib"
            library.write_bytes(b"library")
            (root / "outside.dylib").write_bytes(b"outside")
            # macOS /var is a symlink; this boundary returns canonical paths.
            self.assertEqual(macos.bundled_reference(package, image, "@loader_path/../lib/libheif.1.dylib"), library.resolve())
            for reference in ["@rpath/libheif.1.dylib", "/opt/homebrew/lib/libheif.1.dylib", "@loader_path/../../outside.dylib"]:
                with self.subTest(reference=reference), self.assertRaises(ValueError):
                    macos.bundled_reference(package, image, reference)
            self.assertFalse(macos.system_reference("/usr/lib/../../opt/homebrew/lib/heif.dylib"))

    def test_archive_roundtrip_is_reproducible_across_mtimes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "XDRemux macOS"
            (package / "bin").mkdir(parents=True)
            binary = package / "bin/xdremux"
            binary.write_bytes(b"representative executable bytes")
            binary.chmod(0o755)
            first, second = root / "one.tar.gz", root / "two.tar.gz"
            macos.archive(package, first, 1234567890)
            os.utime(binary, (1734567890, 1734567890))
            os.utime(package, (1734567890, 1734567890))
            macos.archive(package, second, 1234567890)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            restored = macos.extract_archive(first, root / "relocated")
            self.assertEqual((restored / "bin/xdremux").read_bytes(), binary.read_bytes())
            with tarfile.open(first) as archive:
                self.assertTrue(all(m.mtime == 1234567890 and m.uid == 0 and m.gid == 0 for m in archive))

    def test_extraction_rejects_escape_links_duplicates_and_multiple_roots(self):
        cases = [("../escape", "file"), ("/absolute", "file"), ("pkg/link", "link"), ("pkg/item", "duplicate"), ("pkg/./item", "alias"), ("pkg/ITEM", "case"), ("other/item", "root")]
        for name, kind in cases:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = root / "candidate.tar"
                with tarfile.open(source, "w") as archive:
                    initial = tarfile.TarInfo("pkg/item")
                    initial.size = 1
                    archive.addfile(initial, io.BytesIO(b"x"))
                    item = tarfile.TarInfo(name)
                    if kind == "link":
                        item.type = tarfile.SYMTYPE
                        item.linkname = "../../escape"
                        archive.addfile(item)
                    else:
                        item.size = 1
                        archive.addfile(item, io.BytesIO(b"y"))
                with self.assertRaises(ValueError):
                    macos.extract_archive(source, root / "out")
                self.assertFalse((root / "out").exists())
                self.assertFalse((root / "escape").exists())

    def test_dyld_evidence_rejects_host_dependency_even_when_bundled_heif_loaded(self):
        package = Path("/tmp/relocated package")
        good = f"dyld[12]: <uuid> {package}/lib/libheif.1.dylib\n"
        self.assertEqual(smoke_macos.loaded_libraries(good, package), [str(package / "lib/libheif.1.dylib")])
        with self.assertRaisesRegex(ValueError, "non-bundled"):
            smoke_macos.loaded_libraries(good + "dyld[12]: <uuid> /opt/homebrew/lib/libx265.1.dylib\n", package)
        with self.assertRaisesRegex(ValueError, "did not report"):
            smoke_macos.loaded_libraries("", package)


if __name__ == "__main__":
    unittest.main()
