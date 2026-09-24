#!/usr/bin/env python3
"""Assemble, relocate, and audit the macOS package's complete Mach-O closure."""
from __future__ import annotations
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import posixpath
import shutil
import subprocess
import tarfile

LOAD_COMMANDS = {"LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LOAD_UPWARD_DYLIB", "LC_LAZY_LOAD_DYLIB"}
LIBRARIES = ("libheif.", "libde265.", "libx265.", "libsharpyuv.")

def output(*args: object) -> str:
    return subprocess.check_output([str(arg) for arg in args], text=True)

def call(*args: object) -> None:
    subprocess.run([str(arg) for arg in args], check=True)

def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def system_reference(value: str) -> bool:
    normalized = posixpath.normpath(value)
    return normalized.startswith(("/usr/lib/", "/System/Library/"))

def parse_load_commands(text: str) -> dict:
    result = {"dependencies": [], "rpaths": [], "install_id": None, "dyld_environment": []}
    command = ""
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("cmd "):
            command = line[4:]
        elif line.startswith("name "):
            value = line[5:].split(" (offset ", 1)[0]
            if command in LOAD_COMMANDS:
                result["dependencies"].append(value)
            elif command == "LC_ID_DYLIB":
                result["install_id"] = value
            elif command == "LC_DYLD_ENVIRONMENT":
                result["dyld_environment"].append(value)
        elif command == "LC_RPATH" and line.startswith("path "):
            result["rpaths"].append(line[5:].split(" (offset ", 1)[0])
        elif command == "LC_BUILD_VERSION" and line.startswith("minos "):
            result["minimum_os"] = line[6:]
        elif command == "LC_VERSION_MIN_MACOSX" and line.startswith("version "):
            result["minimum_os"] = line[8:]
    return result

def inspect(path: Path) -> dict:
    result = parse_load_commands(output("/usr/bin/otool", "-l", path))
    result["architectures"] = output("/usr/bin/lipo", "-archs", path).strip().split()
    return result

def bundled_reference(package: Path, image: Path, reference: str) -> Path:
    if not reference.startswith("@loader_path/"):
        raise ValueError(f"Non-relocatable dependency in {image.name}: {reference}")
    target = (image.parent / reference.removeprefix("@loader_path/")).resolve()
    if not target.is_relative_to(package.resolve()) or not target.is_file():
        raise ValueError(f"Missing or escaping dependency in {image.name}: {reference}")
    return target

def audit(package: Path, arch: str) -> dict:
    package = package.resolve()
    files = [package / "bin/xdremux", package / "libexec/xdremux-apple-adapter"]
    libraries = sorted((package / "lib").glob("*.dylib"))
    if len(libraries) != len(LIBRARIES) or any(sum(path.name.startswith(prefix) for path in libraries) != 1 for prefix in LIBRARIES):
        raise ValueError(f"Expected exactly the pinned HEVC/SharpYUV closure, got {libraries}")
    files += libraries
    known = {path.resolve() for path in files}
    records = []
    for path in files:
        if not path.is_file() or not os.access(path, os.X_OK):
            raise ValueError(f"Missing executable component: {path}")
        if path.is_symlink():
            raise ValueError(f"Distribution components must be real files: {path}")
        facts = inspect(path)
        if facts["architectures"] != [arch] or facts["dyld_environment"]:
            raise ValueError(f"Unexpected architecture or embedded dyld environment: {path}: {facts}")
        for reference in facts["dependencies"]:
            if not system_reference(reference) and bundled_reference(package, path, reference) not in known:
                raise ValueError(f"Unaccounted dynamic dependency: {reference}")
        for rpath in facts["rpaths"]:
            if not system_reference(rpath):
                raise ValueError(f"Non-system rpath survived relocation: {path}: {rpath}")
        if facts["install_id"] is not None and facts["install_id"] != f"@rpath/{path.name}":
            raise ValueError(f"Unrelocated dylib ID: {path}: {facts['install_id']}")
        call("/usr/bin/codesign", "--verify", "--strict", path)
        records.append({"path": path.relative_to(package).as_posix(), "sha256": sha256(path), **facts})
    return {"schema_version": 1, "architecture": arch, "files": records}

def stage(args: argparse.Namespace) -> None:
    package = args.output.resolve()
    if package.exists():
        raise ValueError(f"Refusing to overwrite staging directory: {package}")
    for subdir in ("bin", "libexec", "lib", "share/licenses", "share/sources"):
        (package / subdir).mkdir(parents=True, exist_ok=True)
    prefix = (args.native / "prefix").resolve()
    queue = [(args.cli.resolve(), package / "bin/xdremux"), (args.adapter.resolve(), package / "libexec/xdremux-apple-adapter")]
    records: dict[Path, tuple[Path, dict]] = {}
    while queue:
        source, destination = queue.pop(0)
        if destination in records:
            if sha256(source) != sha256(records[destination][0]):
                raise ValueError(f"Dylib basename collision: {source}, {destination}")
            continue
        facts = inspect(source)
        shutil.copy2(source, destination)
        destination.chmod(0o755)
        records[destination] = (source, facts)
        for reference in facts["dependencies"]:
            if system_reference(reference):
                continue
            if reference.startswith("@rpath/"):
                library = prefix / "lib" / reference.removeprefix("@rpath/")
            elif reference.startswith("@loader_path/"):
                library = source.parent / reference.removeprefix("@loader_path/")
            else:
                library = Path(reference)
            library = library.resolve()
            if not library.is_relative_to(prefix) or not library.is_file():
                raise ValueError(f"Dependency escaped pinned native prefix: {source}: {reference}")
            name = Path(reference).name
            if not name.startswith(LIBRARIES) or not name.endswith(".dylib"):
                raise ValueError(f"Unexpected native dependency: {name}")
            queue.append((library, package / "lib" / name))
    for destination, (_, facts) in records.items():
        for reference in facts["dependencies"]:
            if not system_reference(reference):
                relative = os.path.relpath(package / "lib" / Path(reference).name, destination.parent)
                call("/usr/bin/install_name_tool", "-change", reference, f"@loader_path/{relative}", destination)
        if facts["install_id"] is not None:
            call("/usr/bin/install_name_tool", "-id", f"@rpath/{destination.name}", destination)
        for rpath in set(facts["rpaths"]):
            if not system_reference(rpath):
                call("/usr/bin/install_name_tool", "-delete_rpath", rpath, destination)
        call("/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none", destination)
    manifest = json.loads((args.native / "native-dependencies.json").read_text())
    for name in manifest["packages"]:
        destination = package / "share/licenses" / name
        destination.mkdir()
        licenses = [path for path in (args.native / "src" / name).iterdir() if path.is_file() and (path.name.startswith(("COPYING", "LICENSE")) or path.name in ("PATENTS", "AUTHORS"))]
        if not licenses:
            raise ValueError(f"No upstream license text found: {name}")
        for path in licenses:
            shutil.copy2(path, destination / path.name)
    for path in (args.native / "sources").iterdir():
        shutil.copy2(path, package / "share/sources" / path.name)
    shutil.copy2(Path(__file__).with_name("package-readme.md"), package / "README.md")
    manifest.update({"head": args.head, "architecture": args.arch, "validation_target_os": "macOS 26", "plugin_loading": False, "signature": "ad-hoc, not Developer ID or notarized"})
    (package / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    report = audit(package, args.arch)
    report["head"] = args.head
    (package.parent / f"{package.name}-linkage.json").write_text(json.dumps(report, indent=2) + "\n")

def archive(package: Path, destination: Path, epoch: int) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for path in [package, *sorted(package.rglob("*"))]:
                info = tar.gettarinfo(str(path), arcname=path.relative_to(package.parent).as_posix())
                if not (info.isfile() or info.isdir()):
                    raise ValueError(f"Only ordinary files/directories belong in the package: {path}")
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = epoch
                info.pax_headers = {}
                if info.isfile():
                    with path.open("rb") as stream:
                        tar.addfile(info, stream)
                else:
                    tar.addfile(info)
    destination.with_suffix(destination.suffix + ".sha256").write_text(f"{sha256(destination)}  {destination.name}\n")

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("stage")
    for name in ("cli", "adapter", "native", "output"):
        build.add_argument(f"--{name}", required=True, type=Path)
    build.add_argument("--head", required=True)
    build.add_argument("--arch", required=True, choices=("arm64", "x86_64"))
    check = commands.add_parser("audit")
    check.add_argument("--package", required=True, type=Path)
    check.add_argument("--arch", required=True, choices=("arm64", "x86_64"))
    pack = commands.add_parser("archive")
    pack.add_argument("--package", required=True, type=Path)
    pack.add_argument("--output", required=True, type=Path)
    pack.add_argument("--epoch", required=True, type=int)
    args = parser.parse_args()
    if args.command == "stage":
        stage(args)
    elif args.command == "audit":
        print(json.dumps(audit(args.package, args.arch), indent=2))
    else:
        archive(args.package, args.output, args.epoch)

if __name__ == "__main__":
    main()
