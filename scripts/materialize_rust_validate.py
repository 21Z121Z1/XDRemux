#!/usr/bin/env python3
from pathlib import Path
import subprocess


RUNTIME_LIB = Path("crates/xdremux-runtime/src/lib.rs")
CLI_LIB = Path("crates/xdremux-cli/src/lib.rs")
RUNTIME_BASE = "6cf903a8be489e6a2d79c3beaf2d3ea6bdd2559c"


def git_file(commit: str, path: Path) -> str:
    return subprocess.check_output(
        ["git", "show", f"{commit}:{path.as_posix()}"],
        text=True,
    )


# A prior contents-API edit was based on a partial file view. Fail closed and
# restore the last known-good complete runtime before applying the tiny module
# wiring. Do not silently accept a truncated business core.
runtime = RUNTIME_LIB.read_text()
if "fn conform_raster_channels(" not in runtime or "struct AtomicFilePublisher" not in runtime:
    runtime = git_file(RUNTIME_BASE, RUNTIME_LIB)

if "mod validation;" not in runtime:
    anchor = "mod live_photo;\n"
    if anchor not in runtime:
        raise SystemExit("runtime live_photo module anchor not found")
    runtime = runtime.replace(anchor, anchor + "mod validation;\n", 1)

validation_export = """pub use validation::{
    validate_media_file, IsoHdrValidationReport, LivePhotoValidationReport, ValidationReport,
};
"""
if validation_export not in runtime:
    anchor = "pub use live_photo::LivePhotoFileReceipt;\n"
    if anchor not in runtime:
        raise SystemExit("runtime LivePhoto export anchor not found")
    runtime = runtime.replace(anchor, anchor + validation_export, 1)

for marker in (
    "fn conform_raster_channels(",
    "struct AtomicFilePublisher",
    "mod validation;",
    "validate_media_file, IsoHdrValidationReport",
):
    if marker not in runtime:
        raise SystemExit(f"runtime validation wiring missing marker: {marker}")
RUNTIME_LIB.write_text(runtime)

cli = CLI_LIB.read_text()
if "mod validate;" not in cli:
    anchor = "mod categorize;\n"
    if anchor not in cli:
        raise SystemExit("CLI categorize module anchor not found")
    cli = cli.replace(anchor, anchor + "mod validate;\n", 1)

validate_variant = """    /// Validate one canonical output without converting it.
    Validate(validate::ValidateArgs),
"""
if validate_variant not in cli:
    anchor = """    /// Classify photo assets and publish them into deterministic folders.
    Categorize(categorize::CategorizeArgs),
"""
    if anchor not in cli:
        raise SystemExit("CLI categorize command anchor not found")
    cli = cli.replace(anchor, anchor + validate_variant, 1)

validate_route = """        Ok(Cli {
            command: RootCommand::Validate(arguments),
        }) => validate::run(arguments, stdout, stderr),
"""
if validate_route not in cli:
    anchor = """        Ok(Cli {
            command: RootCommand::Categorize(arguments),
        }) => categorize::run(arguments, stdout, stderr),
"""
    if anchor not in cli:
        raise SystemExit("CLI categorize route anchor not found")
    cli = cli.replace(anchor, anchor + validate_route, 1)

for marker in ("mod validate;", "Validate(validate::ValidateArgs)", "validate::run(arguments"):
    if marker not in cli:
        raise SystemExit(f"CLI validation wiring missing marker: {marker}")
CLI_LIB.write_text(cli)
