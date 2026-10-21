# Motion Photo conversion

Convert a versioned Samsung HEIF Motion Photo into a new HEIC/MOV Live Photo pair and an optional PVT directory, then validate their media through the public CLI. This path avoids re-encoding a large JPEG fixture.

## Sub-features

- Inspect the source's Motion Photo recognition and source timing.
- Publish both members at a fresh shared basename.
- Validate the still and movie through separate public invocations.
- Preserve the original fixture and retain proof after scratch cleanup.
- Select optional PVT output, validate its directory/plist and members, unwrap byte-identical media, and reject a duplicate destination without changing it.

## How to get to it (user POV)

Run `xdremux convert --input <source.heic> --output <new.heic>`, then `xdremux validate <new.heic> --json` and `xdremux validate <new.mov> --json`. Select `--live-photo-output pvt --output <new.pvt>` for a package and validate that directory. Batch is a separate entry point described in the feature index.

## Driving it with the Rust CLI

Preconditions: run from the repository root, set `XDREMUX_VERIFY_CLI` to an executable CLI, and keep the public fixture/manifest unchanged. The recipe deliberately labels an existing binary as unbound to the current source. A new build and completion receipt remain separate evidence.

Run this standard-library recipe. It writes only two new temporary directories, captures commands and hashes, removes its own output directory, and retains proof on success or failure.

```python
import hashlib
import json
import os
import plistlib
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path.cwd()
cli = Path(os.environ["XDREMUX_VERIFY_CLI"]).resolve()
relative = "motion-photo/samsung/heif-ultrahdr-01.heic"
source = root / "fixtures" / relative
proof = Path(tempfile.mkdtemp(prefix="xdremux-verify-proof-"))
work = Path(tempfile.mkdtemp(prefix="xdremux-verify-work-"))
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
commands = []
report = {"feature": "motion-photo", "passed": False,
          "binary": str(cli),
          "build_binding": "existing binary; current source build not established",
          "exact_head_acceptance": False, "proof": str(proof), "scratch": str(work)}
def run(name, arguments, expected_exit=0):
    try:
        result = subprocess.run([str(arg) for arg in arguments], cwd=root,
                                capture_output=True, timeout=180)
    except subprocess.TimeoutExpired as error:
        (proof / f"{name}.stdout").write_bytes(error.stdout or b"")
        (proof / f"{name}.stderr").write_bytes(error.stderr or b"")
        commands.append({"name": name, "command": [str(arg) for arg in arguments],
                         "exit_code": None, "timeout_seconds": 180})
        (proof / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        raise
    (proof / f"{name}.stdout").write_bytes(result.stdout)
    (proof / f"{name}.stderr").write_bytes(result.stderr)
    commands.append({"name": name, "command": [str(arg) for arg in arguments],
                     "exit_code": result.returncode})
    (proof / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
    if result.returncode != expected_exit:
        raise RuntimeError(f"{name} failed with exit {result.returncode}; see {proof}")
    return result.stdout
try:
    report["source_head"] = run("source-head", ["git", "rev-parse", "HEAD"]).decode().strip()
    report["source_status"] = run("source-status", ["git", "status", "--porcelain"]).decode()
    diff = run("source-diff", ["git", "diff", "HEAD", "--binary"])
    report["source_diff_sha256"] = hashlib.sha256(diff).hexdigest()
    report["binary_sha256"] = digest(cli)
    manifest = root / "fixtures/SHA256SUMS"
    report["fixture_manifest_sha256"] = digest(manifest)
    expected = next(line.split()[0] for line in manifest.read_text().splitlines()
                    if line.split(maxsplit=1)[-1].strip() == relative)
    report["source_fixture"] = relative
    report["source_sha256_before"] = digest(source)
    assert report["source_sha256_before"] == expected, "fixture identity mismatch"
    help_text = run("help", [cli, "--help"]).decode()
    assert all(command in help_text for command in ["inspect", "convert", "validate"])
    inspection = json.loads(run("inspect", [cli, "inspect", source, "--json"]))
    assert inspection["kind"] == "motion-photo"
    still = work / "converted.heic"
    movie = work / "converted.mov"
    run("convert", [cli, "convert", "--input", source, "--output", still])
    report["outputs"] = []
    for name, output in [("still", still), ("movie", movie)]:
        assert output.is_file() and output.stat().st_size > 0
        validation = json.loads(run(f"validate-{name}", [cli, "validate", output, "--json"]))
        assert validation["valid"] is True and validation["kind"] == "live-photo"
        report["outputs"].append({"name": output.name, "bytes": output.stat().st_size,
                                  "sha256": digest(output)})
    package = work / "converted-pvt.pvt"
    run("convert-pvt", [cli, "convert", "--input", source, "--output", package,
                        "--live-photo-output", "pvt"])
    assert package.is_dir()
    assert {path.name for path in package.iterdir()} == {
        "converted-pvt.heic", "converted-pvt.mov", "metadata.plist"}
    metadata = plistlib.loads((package / "metadata.plist").read_bytes())
    version = metadata["PFVideoComplementMetadataVersionKey"]
    assert type(version) is str and version == "1"
    package_validation = json.loads(run("validate-pvt", [cli, "validate", package, "--json"]))
    assert package_validation["valid"] is True and package_validation["kind"] == "live-photo"
    package_details = package_validation["details"]["value"]
    assert package_details["content_identifier"]
    unwrapped = work / "unwrapped"
    unwrapped.mkdir()
    report["pvt_outputs"] = []
    for name in ("converted-pvt.heic", "converted-pvt.mov"):
        shutil.copyfile(package / name, unwrapped / name)
    for name in ("converted-pvt.heic", "converted-pvt.mov"):
        member = package / name
        copy = unwrapped / name
        assert member.read_bytes() == copy.read_bytes(), "PVT round-trip media bytes changed"
        validation = json.loads(run(f"validate-unwrapped-{member.suffix[1:]}",
                                    [cli, "validate", copy, "--json"]))
        assert validation["valid"] is True and validation["kind"] == "live-photo"
        details = validation["details"]["value"]
        assert details["content_identifier"] == package_details["content_identifier"]
        assert details["still_time_seconds"] == package_details["still_time_seconds"]
        report["pvt_outputs"].append({"name": name, "bytes": member.stat().st_size,
                                      "sha256": digest(member)})
    before_duplicate = {path.name: digest(path) for path in package.iterdir()}
    run("duplicate-pvt", [cli, "convert", "--input", source, "--output", package,
                          "--live-photo-output", "pvt"], expected_exit=1)
    assert {path.name: digest(path) for path in package.iterdir()} == before_duplicate
    report["pvt_no_clobber"] = True
    report["pvt_native_acceptance"] = "not exercised; structure is not Photos import or wallpaper"
    report["source_sha256_after"] = digest(source)
    assert report["source_sha256_after"] == report["source_sha256_before"]
    assert digest(cli) == report["binary_sha256"], "binary changed during verification"
    report["passed"] = True
except BaseException as error:
    report["error"] = str(error)
    raise
finally:
    shutil.rmtree(work)
    report["cleanup_completed"] = not work.exists()
    (proof / "manifest.json").write_text(json.dumps(report, indent=2) + "\n")
    assert (proof / "manifest.json").is_file() and (proof / "commands.json").is_file()
    print(f"Retained proof: {proof}")
```

## Gotchas

This recipe proves the exercised binary's portable conversion and structural pair validation. It does not decode the pair in Apple frameworks, import it into Photos, prove all source codecs, test interrupted publication, or establish final-HEAD acceptance. For broader Motion Photo changes, use the existing `scripts/validation/check_rust_motion_photo_real_fixtures.sh` and the owning negative tests. Do not weaken a failed journey for a missing encoder or dependency.

For direct PVT package admission, the `pvt-native-import` job in `research.yml`
runs `scripts/apple/pvt_native_import/check_native_import.py` on the free macOS
runner. It builds the exact source CLI and generates geometric media. It first
compiles a cold SDK canary (240 seconds) and the actual reader (120 seconds)
before creating or booting a simulator. Cold module setup retains toolchain/SDK
identity and ten-second snapshots of only its owned compiler process group;
timeout stops that group and remains non-PASS. It then installs only its
own app and establishes a read-only PhotoKit baseline. It transports the whole
validated directory unchanged into that app. A synthetic `UIImage` control
checks save-action availability without saving it. The probe then offers the
whole directory URL and, if no save action exists, an explicit
`com.apple.live-photo` file representation. It records the SDK's extension/URL
types and automatic item-provider registrations without inventing a PVT UTI.
XCUITest selects only an unambiguous Photos save activity inside the exact
created simulator, at most once. Missing activities stop the owned UI process
group promptly and remain non-PASS. The system callback, phase-bound whole
package input hashes (offered URL or requested file) and independent PhotoKit
asset/resource/identifier/cover/hash readback are all required. A UI callback
alone is not admission. The known unsupported `simctl addmedia` directory route
is not repeated; decomposed-member import and PhotoKit pair creation are not
substitutes. Only fixed text receipts leave the runner; media and UI result
bundles stay local. Missing runtime, SDK, permission, save-action locator,
native acceptance, readback or cleanup remains non-PASS. This probes the iOS
simulator sharing path, not macOS Photos library selection, physical-device
Photos editing or dynamic wallpaper eligibility.
