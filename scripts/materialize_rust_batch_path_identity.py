#!/usr/bin/env python3
from pathlib import Path


checkpoint = Path("crates/xdremux-runtime/src/batch_checkpoint.rs")
checkpoint_text = checkpoint.read_text()
checkpoint_text = checkpoint_text.replace(
    "fn canonical_or_absolute(path: &Path) -> Result<PathBuf> {",
    "pub(crate) fn canonical_or_absolute(path: &Path) -> Result<PathBuf> {",
    1,
)
checkpoint.write_text(checkpoint_text)

batch = Path("crates/xdremux-runtime/src/batch.rs")
batch_text = batch.read_text()
if "canonical_or_absolute, source_signature" not in batch_text:
    batch_text = batch_text.replace(
        "    source_signature, CheckpointOutcome, MotionPhotoCheckpoint, MotionPhotoCheckpointWriter,\n",
        "    canonical_or_absolute, source_signature, CheckpointOutcome, MotionPhotoCheckpoint,\n    MotionPhotoCheckpointWriter,\n",
        1,
    )

absolute_helper = """fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).map_err(|error| RuntimeError::external("batch absolute path", error))
}

"""
batch_text = batch_text.replace(absolute_helper, "", 1)
batch_text = batch_text.replace(
    "    let desired_parent = absolute(parent)?;",
    "    let desired_parent = canonical_or_absolute(parent)?;",
    1,
)

old_output_identity = """std::path::absolute(&item.output).map_err(|error| {
                                        RuntimeError::external("batch resume output path", error)
                                    })?"""
batch_text = batch_text.replace(
    old_output_identity,
    "canonical_or_absolute(&item.output)?",
    1,
)
old_video_identity = """std::path::absolute(&output_video).map_err(|error| {
                                            RuntimeError::external("batch resume video path", error)
                                        })?"""
batch_text = batch_text.replace(
    old_video_identity,
    "canonical_or_absolute(&output_video)?",
    1,
)

required = [
    "pub(crate) fn canonical_or_absolute(path: &Path)",
    "canonical_or_absolute, source_signature",
    "let desired_parent = canonical_or_absolute(parent)?;",
    "== canonical_or_absolute(&item.output)?",
    "== canonical_or_absolute(&output_video)?",
]
combined = checkpoint_text + "\n" + batch_text
missing = [marker for marker in required if marker not in combined]
if missing:
    raise SystemExit(f"path-identity materialization incomplete: {missing}")
if "fn absolute(path: &Path)" in batch_text:
    raise SystemExit("legacy lexical absolute helper remains")
if "std::path::absolute(&item.output)" in batch_text or "std::path::absolute(&output_video)" in batch_text:
    raise SystemExit("resume still compares lexical absolute paths")

batch.write_text(batch_text)
