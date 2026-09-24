#!/usr/bin/env python3
"""Build pinned macOS HEVC dependencies. This is build tooling, not product policy."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile
from urllib.parse import urlsplit

MANIFEST = Path(__file__).with_name("native-dependencies.json")

def run(*args: object, **kwargs: object) -> None:
    print("+", " ".join(str(arg) for arg in args), flush=True)
    subprocess.run([str(arg) for arg in args], check=True, **kwargs)

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--arch", choices=("arm64", "x86_64"), required=True)
    args = parser.parse_args()
    if platform.system() != "Darwin" or platform.machine() != args.arch:
        raise SystemExit("Build on a native macOS runner of the requested architecture")
    work = args.work.resolve()
    if work.exists():
        raise SystemExit(f"Refusing to reuse a possibly contaminated native build: {work}")
    work.mkdir(parents=True)
    prefix = work / "prefix"
    sources = work / "sources"
    sources.mkdir()
    packages = json.loads(MANIFEST.read_text())["packages"]
    for name, package in packages.items():
        archive = sources / Path(urlsplit(package["url"]).path).name
        run("curl", "--fail", "--location", "--retry", 3, "--proto", "=https", "--tlsv1.2", package["url"], "--output", archive)
        if hashlib.sha256(archive.read_bytes()).hexdigest() != package["sha256"]:
            raise SystemExit(f"Source checksum mismatch: {name}")
        extracted = work / "src" / name
        extracted.mkdir(parents=True)
        # Verify even pinned archives before stripping their release directory.
        with tarfile.open(archive) as tar:
            for member in tar.getmembers():
                path = Path(member.name)
                if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
                    raise SystemExit(f"Unsafe source archive member: {member.name}")
                if len(path.parts) < 2:
                    continue
                destination = extracted.joinpath(*path.parts[1:])
                if member.isdir():
                    destination.mkdir(parents=True, exist_ok=True)
                elif member.isfile():
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    stream = tar.extractfile(member)
                    assert stream is not None
                    with stream, destination.open("wb") as output:
                        shutil.copyfileobj(stream, output)
                    destination.chmod(member.mode & 0o777)
                else:
                    raise SystemExit(f"Unsupported source archive member: {member.name}")
    common = [
        "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release", f"-DCMAKE_INSTALL_PREFIX={prefix}",
        "-DCMAKE_INSTALL_LIBDIR=lib", f"-DCMAKE_OSX_ARCHITECTURES={args.arch}",
        "-DCMAKE_OSX_DEPLOYMENT_TARGET=15.0", "-DCMAKE_INSTALL_NAME_DIR=@rpath",
        "-DCMAKE_INSTALL_RPATH=", "-DCMAKE_INSTALL_RPATH_USE_LINK_PATH=OFF",
        "-DCMAKE_SHARED_LINKER_FLAGS=-Wl,-headerpad_max_install_names",
        "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
    ]
    jobs = str(os.cpu_count() or 2)
    def configure(source: Path, build: str, flags: list[str]) -> Path:
        directory = work / "build" / build
        run("cmake", "-S", source, "-B", directory, *common, *flags)
        run("cmake", "--build", directory, "--parallel", jobs)
        return directory
    de265 = configure(work / "src/libde265", "libde265", [
        "-DBUILD_SHARED_LIBS=ON", "-DENABLE_DECODER=OFF", "-DENABLE_ENCODER=OFF",
        "-DENABLE_SDL=OFF", "-DENABLE_INTERNAL_DEVELOPMENT_TOOLS=OFF",
    ])
    run("cmake", "--install", de265)
    # Preserve the existing provider's 8/10/12-bit x265 API.
    high = ["-DHIGH_BIT_DEPTH=ON", "-DEXPORT_C_API=OFF", "-DENABLE_SHARED=OFF", "-DENABLE_CLI=OFF"]
    ten = configure(work / "src/x265/source", "x265-10", high + ["-DENABLE_HDR10_PLUS=ON"])
    twelve = configure(work / "src/x265/source", "x265-12", high + ["-DMAIN12=ON"])
    eight = work / "build/x265-8"
    eight.mkdir()
    shutil.copy2(ten / "libx265.a", eight / "libx265_main10.a")
    shutil.copy2(twelve / "libx265.a", eight / "libx265_main12.a")
    configure(work / "src/x265/source", "x265-8", [
        "-DENABLE_SHARED=ON", "-DENABLE_CLI=OFF", "-DLINKED_10BIT=ON", "-DLINKED_12BIT=ON",
        "-DEXTRA_LINK_FLAGS=-L.", "-DEXTRA_LIB=x265_main10.a;x265_main12.a",
    ])
    run("cmake", "--install", eight)
    # Preserve SharpYUV color conversion; only libsharpyuv enters the package.
    webp = work / "src/libwebp"
    options = re.findall(r"^option\((WEBP_BUILD_\w+)", (webp / "CMakeLists.txt").read_text(), re.M)
    webp_build = configure(webp, "libwebp", ["-DBUILD_SHARED_LIBS=ON", "-DWEBP_LINK_STATIC=OFF", *[f"-D{name}=OFF" for name in options]])
    run("cmake", "--install", webp_build)
    heif = work / "src/libheif"
    codecs = re.findall(r"^plugin_option\((\w+)", (heif / "CMakeLists.txt").read_text(), re.M)
    flags = []
    for codec in codecs:
        flags += [f"-DWITH_{codec}={'ON' if codec in ('LIBDE265', 'X265') else 'OFF'}", f"-DWITH_{codec}_PLUGIN=OFF"]
    heif_build = configure(heif, "libheif", [
        *flags, f"-DCMAKE_PREFIX_PATH={prefix}", "-DBUILD_SHARED_LIBS=ON",
        "-DENABLE_PLUGIN_LOADING=OFF", "-DPLUGIN_DIRECTORY=", "-DWITH_LIBSHARPYUV=ON",
        "-DWITH_UNCOMPRESSED_CODEC=OFF", "-DWITH_HEADER_COMPRESSION=OFF",
        "-DWITH_EXAMPLES=OFF", "-DWITH_GDK_PIXBUF=OFF", "-DBUILD_TESTING=OFF",
        "-DBUILD_DOCUMENTATION=OFF", "-DBUILD_DEVELOPMENT_TOOLS=OFF",
    ])
    run("cmake", "--install", heif_build)
    cache = (heif_build / "CMakeCache.txt").read_text()
    for required in ("ENABLE_PLUGIN_LOADING:BOOL=OFF", "WITH_LIBDE265:BOOL=ON", "WITH_X265:BOOL=ON", "WITH_LIBSHARPYUV:BOOL=ON", "WITH_LIBDE265_PLUGIN:BOOL=OFF", "WITH_X265_PLUGIN:BOOL=OFF"):
        if required not in cache:
            raise SystemExit(f"Missing codec build invariant: {required}")
    shutil.copy2(MANIFEST, work / "native-dependencies.json")
    print(f"native-prefix={prefix}", flush=True)

if __name__ == "__main__":
    main()
