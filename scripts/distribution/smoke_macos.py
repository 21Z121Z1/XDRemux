#!/usr/bin/env python3
"""Verify the archived package outside the checkout with a clean process environment."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
from urllib.parse import urlsplit

if __package__:
    from .macos import MINIMUM_MACOS, PRODUCT_ARCH, audit, extract_archive, require_native_host, sha256, system_reference
else:
    from macos import MINIMUM_MACOS, PRODUCT_ARCH, audit, extract_archive, require_native_host, sha256, system_reference


def loaded_libraries(stderr: str, package: Path, *, require_heif: bool = True) -> list[str]:
    libraries = []
    for line in stderr.splitlines():
        if not line.startswith(("dyld[", "dyld: loaded:")):
            continue
        match = re.search(r"(/[^\n]+)$", line)
        if match is None:
            continue
        value = match.group(1)
        resolved = Path(value).resolve()
        system = system_reference(value) or any(
            value.startswith(prefix) for prefix in (
                "/System/Volumes/Preboot/Cryptexes/OS/usr/lib/",
                "/System/Volumes/Preboot/Cryptexes/OS/System/Library/",
            )
        )
        if not system and not resolved.is_relative_to(package.resolve()):
            raise ValueError(f"Runtime loaded a non-bundled library: {value}")
        libraries.append(value)
    if require_heif and not any(Path(value).name.startswith("libheif.") for value in libraries):
        raise ValueError("dyld did not report the bundled libheif load")
    return libraries


def verify_license_payload(package: Path, manifest: dict, expected_head: str) -> dict:
    notices = package / "share/licenses/xdremux"
    for name in ("LICENSE", "LICENSING.md", "LICENSING.zh-CN.md"):
        item = notices / name
        if not item.is_file() or not item.stat().st_size:
            raise ValueError(f"Missing XDRemux license notice: {name}")
    if "GNU GENERAL PUBLIC LICENSE" not in (notices / "LICENSE").read_text():
        raise ValueError("Missing GPLv3 legal text")
    source_notice = (package / "CORRESPONDING_SOURCE.md").read_text()
    if f"https://github.com/21Z121Z1/XDRemux/tree/{expected_head}" not in source_notice:
        raise ValueError("Missing exact corresponding-source URL")
    inventory = json.loads((package / "share/licenses/rust-dependencies.json").read_text())
    dependencies = inventory["packages"]
    if not dependencies or len(dependencies) != manifest["rust_dependencies"]:
        raise ValueError("Incomplete resolved Rust dependency license inventory")
    for dependency in dependencies:
        names = dependency["notices"]
        if not names:
            raise ValueError(f"Missing Rust dependency license: {dependency['name']}")
        for notice in names:
            item = package / "share/licenses/rust" / f"{dependency['name']}-{dependency['version']}" / notice["file"]
            if not item.is_file() or sha256(item) != notice["sha256"]:
                raise ValueError(f"Rust dependency license mismatch: {item}")
    return {"project_license": "GPL-3.0-only", "rust_dependencies": len(dependencies),
            "rust_packages": [entry["name"] for entry in dependencies]}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--fixtures", required=True, type=Path)
    parser.add_argument("--arch", required=True, choices=(PRODUCT_ARCH,))
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        require_native_host(args.arch)
    except ValueError as error:
        parser.error(str(error))
    evidence = args.output.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    report = {"head": args.expected_head, "architecture": args.arch,
              "host_os": platform.mac_ver()[0], "passed": False,
              "native_feature_acceptance": False, "commands": [], "fixtures": []}
    work = None
    try:
        expected_archive = args.archive.with_suffix(args.archive.suffix + ".sha256").read_text().split()[0]
        report["archive_sha256"] = sha256(args.archive)
        if report["archive_sha256"] != expected_archive:
            raise ValueError("Archive checksum mismatch")
        with tempfile.TemporaryDirectory(prefix="xdremux-relocated-") as temporary:
            work = Path(temporary).resolve()
            checkout = Path(__file__).resolve().parents[2]
            if work.is_relative_to(checkout):
                raise ValueError("Package smoke scratch directory must be outside the checkout")
            if evidence.is_relative_to(work):
                raise ValueError("Evidence directory must survive scratch cleanup")
            report["scratch"] = str(work)
            report["checkout"] = str(checkout)
            package = extract_archive(args.archive, work / "outside checkout")
            facts = audit(package, args.arch)
            (evidence / "relocated-linkage.json").write_text(json.dumps(facts, indent=2) + "\n")
            manifest = json.loads((package / "manifest.json").read_text())
            if (manifest["head"] != args.expected_head or manifest["architecture"] != args.arch
                    or manifest.get("minimum_macos") != MINIMUM_MACOS):
                raise ValueError("Package identity differs from requested source/architecture/minimum OS")
            report["licenses"] = verify_license_payload(package, manifest, args.expected_head)
            report["sources"] = []
            for name, dependency in manifest["packages"].items():
                source = package / "share/sources" / Path(urlsplit(dependency["url"]).path).name
                if sha256(source) != dependency["sha256"]:
                    raise ValueError(f"Bundled source archive checksum mismatch: {name}")
                notices = sorted((package / "share/licenses" / name).iterdir())
                if not notices or any(not notice.is_file() or not notice.stat().st_size for notice in notices):
                    raise ValueError(f"Missing bundled upstream license notices: {name}")
                report["sources"].append({"name": name, "archive": source.name,
                                          "sha256": dependency["sha256"],
                                          "notices": {item.name: sha256(item) for item in notices}})
            cli = package / "bin/xdremux"
            home = work / "home"
            home.mkdir()
            scratch = work / "tmp"
            scratch.mkdir()
            clean = ["/usr/bin/env", "-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin",
                     f"HOME={home}", f"TMPDIR={scratch}", "LANG=C", "DYLD_PRINT_LIBRARIES=1"]

            def run(name: str, arguments: list, success: bool = True, stdin: bytes | None = None) -> bytes:
                command = [*clean, *[str(arg) for arg in arguments]]
                try:
                    result = subprocess.run(command, cwd=work, input=stdin, capture_output=True,
                                            timeout=600, env={"PATH": "/usr/bin:/bin:/usr/sbin:/sbin"})
                except subprocess.TimeoutExpired as error:
                    (evidence / f"{name}.stdout").write_bytes(error.stdout or b"")
                    (evidence / f"{name}.stderr").write_bytes(error.stderr or b"")
                    report["commands"].append({"name": name, "command": command, "timed_out": True})
                    raise
                (evidence / f"{name}.stdout").write_bytes(result.stdout)
                (evidence / f"{name}.stderr").write_bytes(result.stderr)
                record = {"name": name, "command": command, "exit_code": result.returncode}
                report["commands"].append(record)
                if (result.returncode == 0) != success:
                    raise RuntimeError(f"{name} exit {result.returncode}; see {evidence}")
                record["loaded_libraries"] = loaded_libraries(
                    result.stderr.decode(errors="replace"), package, require_heif=Path(arguments[0]) == cli)
                return result.stdout

            run("help-en", [cli, "--lang", "en", "--help"])
            chinese = run("help-zh", [cli, "--lang", "zh-CN", "--help"])
            if "用法：" not in chinese.decode():
                raise ValueError("Packaged Chinese help was not selected")
            adapter = package / "libexec/xdremux-apple-adapter"
            capabilities = json.loads(run("adapter-handshake", [adapter], stdin=b'{"schema_version":2,"operation":"capabilities"}\n'))
            if capabilities.get("schema_version") != 2:
                raise ValueError("Packaged adapter protocol handshake failed")
            fixtures = args.fixtures.resolve()
            identities = {line.split(maxsplit=1)[1].strip(): line.split()[0]
                          for line in (fixtures / "SHA256SUMS").read_text().splitlines() if line.strip()}
            for index, relative in enumerate((
                "motion-photo/samsung/heif-ultrahdr-01.heic",
                "proxdr/oppo/find-x6-pro/lhdr-v1-01.heic",
            )):
                source = fixtures / relative
                original_hash = sha256(source)
                if identities[relative] != original_hash:
                    raise ValueError(f"Fixture identity mismatch: {relative}")
                input_path = work / f"input-{index}.heic"
                shutil.copy2(source, input_path)
                output = work / f"converted-{index}.heic"
                run(f"convert-{index}", [cli, "convert", "--input", input_path, "--output", output])
                outputs = [output, output.with_suffix(".mov")] if index == 0 else [output]
                records = []
                for item in outputs:
                    validation = json.loads(run(f"validate-{index}-{item.suffix[1:]}", [cli, "validate", item, "--json"]))
                    expected_kind = "live-photo" if index == 0 else "iso-hdr-heif"
                    if validation.get("valid") is not True or validation.get("kind") != expected_kind:
                        raise ValueError(f"Unexpected validation: {validation}")
                    records.append({"name": item.name, "bytes": item.stat().st_size, "sha256": sha256(item)})
                if sha256(source) != original_hash or sha256(input_path) != original_hash:
                    raise ValueError("Conversion changed the original input")
                if index == 0:
                    run("reject-existing-pair", [cli, "convert", "--input", input_path, "--output", output], success=False)
                    if any(sha256(item) != record["sha256"] for item, record in zip(outputs, records)):
                        raise ValueError("Rejected conversion changed the existing pair")
                report["fixtures"].append({"source": relative, "sha256": original_hash, "outputs": records})
            report["passed"] = True
    finally:
        report["scratch_removed"] = work is not None and not work.exists()
        (evidence / "package-smoke.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
