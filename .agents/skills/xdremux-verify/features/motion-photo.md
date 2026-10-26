# Motion Photo conversion

Convert a versioned Samsung HEIF Motion Photo into a new HEIC/MOV Live Photo pair. You can also select a PVT directory. Validate the resulting media through the public CLI. This path avoids re-encoding a large JPEG fixture.

## Select the affected behavior

- Inspect Motion Photo recognition and source timing.
- Publish both pair members at a new shared basename.
- Validate the still and movie through separate public invocations.
- Preserve the original fixture and retain proof after scratch cleanup.
- For PVT output, validate the directory, plist, and members. Unwrap byte-identical media. Reject a duplicate destination and preserve its existing contents.

## Use the public entry points

Run `xdremux convert --input <source.heic> --output <new.heic>`.

Then run `xdremux validate <new.heic> --json` and `xdremux validate <new.mov> --json` separately.

For a package, select `--live-photo-output pvt --output <new.pvt>` and validate the output directory. Batch is a separate entry point in the feature index.

## Run the portable recipe

Before starting, use the repository root as the current directory. Set `XDREMUX_VERIFY_CLI` to an executable CLI. Keep the public fixture and manifest unchanged.

The recipe labels an existing binary as unbound to the current source. A new build and a completion receipt provide separate evidence.

Run the standard-library recipe below. It creates two temporary directories and records commands and hashes. It removes its own output directory and retains proof on success or failure.

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

### Interpret the portable result

The recipe proves portable conversion and structural pair validation for the exercised binary. It does not establish these results:

- Apple framework decoding or Photos import.
- Support for every source codec.
- Safe publication after interruption.
- Acceptance of the final committed HEAD.

For broader Motion Photo changes, use `scripts/validation/check_rust_motion_photo_real_fixtures.sh` and the owning negative tests. If an encoder or dependency is missing, retain the failed journey result. Do not weaken the check.

## Check native PVT admission

Use an arm64 macOS host with the required Xcode SDK. For the CI baseline, the installed iOS 27.0 runtime must support iPhone 11. From a clean, committed checkout, run:

```bash
pvt_repo="$(git rev-parse --show-toplevel)"
pvt_head="$(git rev-parse HEAD)"
pvt_proof="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-pvt-proof.XXXXXX")"
python3 scripts/apple/pvt_native_import/readiness_control.py \
  --repo "$pvt_repo" \
  --expected-head "$pvt_head" \
  --output-root "$pvt_proof/control" \
  --product-output-root "$pvt_proof/product" \
  --required-runtime com.apple.CoreSimulator.SimRuntime.iOS-27-0 \
  --required-device-type com.apple.CoreSimulator.SimDeviceType.iPhone-11 \
  --execute-owned-simulator
```

The `pvt-native-import` job in `research.yml` runs this control and product sequence against one exact PR or main checkout. The receipt records the full source SHA, source tree, product owner trees, and probe-file hashes.

The CI job requires this exact runtime and device pair. A missing or incompatible pair blocks simulator creation. For a separate catalog-selected experiment, omit both pin options. Supplying only one is a usage error.

Keep both output roots outside the checkout. The product output root must not exist before the run.

### Require a passed readiness control

The selector checks explicit runtime support and the device type's runtime version bounds. Before creating its simulator, the readiness control independently verifies membership in the supported list. With both pin options, it validates only that pair and selects no replacement.

The control starts and monitors its simulator with `simctl bootstatus OWNED_UDID -b`. Both control and product require exit code `0` and a recognized `Finished` terminal record for the owned device. Migration failure, an already-booted shortcut, or unknown output blocks further work.

After successful boot monitoring, the control reads the installed-app inventory once. It then checks deletion in both the device registry and the host directory. Only a passed and cleaned control permits one product probe.

The control passes its exact pair through `--required-runtime` and `--required-device-type`. The product validates that pair against fresh catalogs without selecting a replacement. After build and preparation, it checks the same pair again immediately before creation. A changed or incompatible pair blocks creation.

The product receipt must match the control's source SHA, runtime, and device type. The control can warm CoreSimulator. It does not establish a cold-start repair or Photos admission.

### Require whole-package admission and readback

The product probe builds the current CLI and generates synthetic media. It compiles the SDK canary and reader, then uses its own simulator.

The probe verifies installed app identity and establishes a read-only PhotoKit baseline. It then offers the whole validated PVT directory through system sharing.

Admission requires all these observations:

- The system callback.
- Whole-package input hashes bound to the share phase.
- Independent PhotoKit readback of the asset, resources, identifiers, cover time, and exported bytes.

A callback alone cannot replace these checks. Neither decomposed-member import nor PhotoKit pair creation can replace them.

Use the probe's existing process budgets. Runtime, build, inventory, install, permission, save-action, readback, and cleanup failures remain non-PASS.

### Preserve stages, clocks, and cleanup evidence

The receipts keep preparation, readiness, XCTest setup and body, each share representation, the save callback, and independent PhotoKit readback as separate observations. Each observation is bound to the source SHA and run session.

New native events record monotonic elapsed time in their named process clock. An outer timeout preserves the last observed boundary and the missing stages. It does not establish a hang, an unsupported representation, or Photos rejection. Archived text logs retain their missing clock boundaries.

Cleanup records command outcomes separately from independent registry and directory absence. Both must pass for cleanup to be accepted. If absence is observed later, link the original receipt hash and preserve its status.

### Retain the native evidence

Keep the control and product `result.json` receipts and their diagnostics. Only the workflow's listed text diagnostics leave the runner. Generated media and UI result bundles stay local.

This check covers the iOS simulator sharing path. It does not establish macOS Photos library behavior, physical-device Photos editing, or dynamic wallpaper eligibility.
