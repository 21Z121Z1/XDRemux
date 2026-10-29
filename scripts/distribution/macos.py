#!/usr/bin/env python3
"""Assemble, relocate, and audit the macOS package's complete Mach-O closure."""
from __future__ import annotations
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import posixpath
import re
import shutil
import subprocess
import tarfile

LOAD_COMMANDS = {"LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LOAD_UPWARD_DYLIB", "LC_LAZY_LOAD_DYLIB"}
LIBRARIES = ("libheif.", "libde265.", "libx265.", "libsharpyuv.")
PRODUCT_ARCH = "arm64"
MINIMUM_MACOS = "27.0"

def require_native_host(arch: str) -> None:
    version = platform.mac_ver()[0].split(".")[0]
    if (arch != PRODUCT_ARCH or platform.system() != "Darwin" or platform.machine() != PRODUCT_ARCH
            or not version.isdecimal() or int(version) < int(MINIMUM_MACOS.split(".")[0])):
        raise ValueError("Product packaging requires native arm64 on macOS 27 or later")

def output(*args: object) -> str:
    return subprocess.check_output([str(arg) for arg in args], text=True)

def call(*args: object) -> None:
    subprocess.run([str(arg) for arg in args], check=True)

def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def stage_project_licenses(package: Path, head: str, checkout: Path) -> None:
    """Include the project's license and immutable source location in an exported package."""
    if re.fullmatch(r"[0-9a-f]{40}", head) is None:
        raise ValueError("Product source revision must be an exact 40-character Git SHA")
    destination = package / "share/licenses/xdremux"
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("LICENSE", "LICENSING.md", "LICENSING.zh-CN.md"):
        shutil.copy2(checkout / name, destination / name)
    (package / "CORRESPONDING_SOURCE.md").write_text(
        "# Corresponding source for XDRemux\n\n"
        f"Product source revision: `{head}`\n\n"
        f"Repository source snapshot: https://github.com/21Z121Z1/XDRemux/tree/{head}\n\n"
        "Build scripts, Rust workspace, Swift adapter source, Cargo.lock, and dependency "
        "declarations live in that snapshot. See `docs/development.en.md` and "
        "`scripts/distribution/` in that exact revision.\n\n"
        "The matching native dependency source archives are in `share/sources/`, "
        "with license notices under `share/licenses/`. Cargo dependency versions "
        "and source identities are recorded in `share/licenses/rust-dependencies.json`. "
        "The source archive is not a claim of patent or Apple SDK rights.\n"
    )


def rust_metadata(checkout: Path) -> dict:
    return json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1",
         "--filter-platform", "aarch64-apple-darwin"],
        cwd=checkout, text=True,
    ))


def stage_rust_dependency_licenses(package: Path, metadata: dict) -> list[dict]:
    """Preserve resolved Rust dependencies' original license texts and identities."""
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    destination = package / "share/licenses/rust"
    destination.mkdir(parents=True, exist_ok=True)
    records = []
    packages = sorted(
        (entry for entry in metadata["packages"]
         if entry["id"] in resolved and entry.get("source") is not None),
        key=lambda entry: (entry["name"], entry["version"], entry["id"]),
    )
    for entry in packages:
        origin = Path(entry["manifest_path"]).parent
        candidates = {
            file for file in origin.iterdir()
            if file.is_file() and file.name.upper().startswith(
                ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "PATENTS")
            )
        }
        if entry.get("license_file"):
            specified = origin / entry["license_file"]
            if specified.is_file():
                candidates.add(specified)
        # Git workspace crates (e.g. LibLivePhoto) can inherit a repository-root
        # license without storing a second copy beside the individual crate.
        if not candidates and entry["source"].startswith("git+"):
            for ancestor in origin.parents:
                if ancestor.name == "checkouts":
                    break
                if (ancestor / "Cargo.toml").is_file() and (ancestor / "crates").is_dir():
                    candidates.update(
                        file for file in ancestor.iterdir()
                        if file.is_file() and file.name.upper().startswith(
                            ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "PATENTS")
                        )
                    )
                    if candidates:
                        break
        if not candidates:
            raise ValueError(f"No Rust dependency license text: {entry['name']} {entry['version']}")
        target = destination / f"{entry['name']}-{entry['version']}"
        target.mkdir()
        notices = []
        for file in sorted(candidates):
            filename = file.name
            if (target / filename).exists():
                raise ValueError(f"Duplicate Rust license filename: {entry['name']} {filename}")
            shutil.copy2(file, target / filename)
            notices.append({"file": filename, "sha256": sha256(target / filename)})
        records.append({
            "name": entry["name"], "version": entry["version"],
            "license": entry.get("license"), "source": entry["source"],
            "notices": notices,
        })
    (package / "share/licenses/rust-dependencies.json").write_text(
        json.dumps({"schema_version": 1, "packages": records}, indent=2) + "\n"
    )
    return records


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
    require_native_host(arch)
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
        if facts.get("minimum_os") not in (MINIMUM_MACOS, MINIMUM_MACOS + ".0"):
            raise ValueError(f"Unexpected deployment target (require {MINIMUM_MACOS}): {path}: {facts.get('minimum_os')}")
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
    return {"schema_version": 1, "architecture": arch, "minimum_macos": MINIMUM_MACOS, "files": records}

def stage(args: argparse.Namespace) -> None:
    require_native_host(args.arch)
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
    checkout = Path(__file__).resolve().parents[2]
    stage_project_licenses(package, args.head, checkout)
    rust_dependencies = stage_rust_dependency_licenses(package, rust_metadata(checkout))
    shutil.copy2(Path(__file__).with_name("package-readme.md"), package / "README.md")
    manifest.update({"head": args.head, "architecture": args.arch, "minimum_macos": MINIMUM_MACOS,
                     "rust_dependencies": len(rust_dependencies),
                     "build_os": platform.mac_ver()[0], "plugin_loading": False,
                     "native_features": "Require their documented OS, SDK and runtime capabilities; portable package smoke does not certify them",
                     "signature": "ad-hoc, not Developer ID or notarized"})
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

def extract_archive(source: Path, destination: Path) -> Path:
    """Extract a single ordinary package tree into a new directory."""
    if destination.exists():
        raise ValueError(f"Refusing to reuse extraction directory: {destination}")
    with tarfile.open(source) as package:
        members = package.getmembers()
        names = set()
        roots = set()
        for member in members:
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts or not path.parts:
                raise ValueError(f"Unsafe archive path: {member.name}")
            identity = path.as_posix().casefold()
            if not (member.isfile() or member.isdir()) or identity in names:
                raise ValueError(f"Unsupported or duplicate archive member: {member.name}")
            names.add(identity)
            roots.add(path.parts[0])
        if len(roots) != 1:
            raise ValueError("Archive must contain one package tree")
        destination.mkdir(parents=True)
        package.extractall(destination, members=members, filter="data")
    return destination / roots.pop()

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("stage")
    for name in ("cli", "adapter", "native", "output"):
        build.add_argument(f"--{name}", required=True, type=Path)
    build.add_argument("--head", required=True)
    build.add_argument("--arch", required=True, choices=(PRODUCT_ARCH,))
    check = commands.add_parser("audit")
    check.add_argument("--package", required=True, type=Path)
    check.add_argument("--arch", required=True, choices=(PRODUCT_ARCH,))
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
        audit(args.package, PRODUCT_ARCH)
        archive(args.package, args.output, args.epoch)

if __name__ == "__main__":
    main()
