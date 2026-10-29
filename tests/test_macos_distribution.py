"""Portable contracts for package relocation and reproducible archives."""
from pathlib import Path
import io
import os
import subprocess
import sys
import tarfile
import tempfile
import tomllib
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


    def test_repository_licenses_separate_code_fixtures_and_models(self):
        root = Path(__file__).resolve().parents[1]
        self.assertIn("GNU GENERAL PUBLIC LICENSE", (root / "LICENSE").read_text())
        scope = (root / "LICENSING.md").read_text()
        for tag in ("v1.0", "v1.1", "v1.2", "v1.3", "v1.3.1", "v1.4"):
            self.assertIn(f"[{tag}]", scope)
        self.assertIn("not licensed under GPL-3.0-only", scope)
        fixtures = (root / "fixtures/LICENSE.md").read_text()
        self.assertIn("metadata", fixtures)
        self.assertIn("No additional permission is granted", fixtures)
        self.assertIn("machine-learning models", fixtures)
        self.assertIn("All rights reserved", (root / "research/oppo_styles/models/LICENSING.md").read_text())
        with (root / "Cargo.toml").open("rb") as stream:
            workspace = tomllib.load(stream)
        self.assertEqual(workspace["workspace"]["package"]["license"], "GPL-3.0-only")
        for crate in workspace["workspace"]["members"]:
            with (root / crate / "Cargo.toml").open("rb") as stream:
                cargo = tomllib.load(stream)
            self.assertIs(cargo["package"]["license"]["workspace"], True)
        with (root / "pyproject.toml").open("rb") as stream:
            python = tomllib.load(stream)
        self.assertEqual(python["project"]["license"], "GPL-3.0-only")
        self.assertIn("/LICENSE", python["tool"]["hatch"]["build"]["targets"]["sdist"]["include"])

    def test_project_license_staging_keeps_exact_source_revision(self):
        checkout = Path(__file__).resolve().parents[1]
        sha = "a" * 40
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "package"
            with self.assertRaisesRegex(ValueError, "exact 40-character"):
                macos.stage_project_licenses(package, "main", checkout)
            self.assertFalse(package.exists())
            macos.stage_project_licenses(package, sha, checkout)
            self.assertEqual((package / "share/licenses/xdremux/LICENSE").read_bytes(),
                             (checkout / "LICENSE").read_bytes())
            text = (package / "CORRESPONDING_SOURCE.md").read_text()
            self.assertIn(f"https://github.com/21Z121Z1/XDRemux/tree/{sha}", text)
            self.assertIn("share/sources/", text)

    def test_git_workspace_inherited_license_is_bundled(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkout = root / "liblivephoto-checkout"
            crate = checkout / "crates/liblivephoto-format"
            crate.mkdir(parents=True)
            (checkout / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
            (checkout / "LICENSE").write_text("Apache License Version 2.0")
            (crate / "Cargo.toml").write_text("[package]\nname = 'liblivephoto-format'\n")
            entry = {"id": "git+https://example.org/LibLivePhoto#abcdef",
                     "name": "liblivephoto-format", "version": "0.1.0",
                     "license": "Apache-2.0", "source": "git+https://example.org/LibLivePhoto",
                     "manifest_path": str(crate / "Cargo.toml")}
            destination = root / "package"
            macos.stage_rust_dependency_licenses(
                destination, {"packages": [entry], "resolve": {"nodes": [{"id": entry["id"]}]}}
            )
            self.assertEqual(
                (destination / "share/licenses/rust/liblivephoto-format-0.1.0/LICENSE").read_text(),
                "Apache License Version 2.0",
            )

    def test_rust_dependency_notices_are_verified_against_payload(self):
        checkout = Path(__file__).resolve().parents[1]
        sha = "b" * 40
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            cargo = root / "liblivephoto-format"
            cargo.mkdir()
            (cargo / "LICENSE").write_text("Apache License Version 2.0")
            (cargo / "Cargo.toml").write_text("[package]\nname = 'liblivephoto-format'\n")
            entry = {"id": "git+https://example.org/project#deadbeef", "name": "liblivephoto-format",
                     "version": "1.0.0", "license": "Apache-2.0",
                     "source": "git+https://example.org/project", "manifest_path": str(cargo / "Cargo.toml")}
            metadata = {"packages": [entry], "resolve": {"nodes": [{"id": entry["id"]}]}}
            macos.stage_project_licenses(package, sha, checkout)
            records = macos.stage_rust_dependency_licenses(package, metadata)
            self.assertEqual(records[0]["license"], "Apache-2.0")
            manifest = {"rust_dependencies": 1}
            verified = smoke_macos.verify_license_payload(package, manifest, sha)
            self.assertEqual(verified["rust_dependencies"], 1)
            self.assertIn("liblivephoto-format", verified["rust_packages"])
            (package / "share/licenses/rust/liblivephoto-format-1.0.0/LICENSE").write_text("tampered")
            with self.assertRaisesRegex(ValueError, "license mismatch"):
                smoke_macos.verify_license_payload(package, manifest, sha)


if __name__ == "__main__":
    unittest.main()
