#!/usr/bin/env python3
from pathlib import Path

runtime = Path("crates/xdremux-runtime/src/lib.rs")
text = runtime.read_text()

anchor = '''    pub fn convert_motion_photo_file(
        &self,
        source: &[u8],
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Result<LivePhotoFileReceipt> {
        live_photo::convert_motion_photo_file(
            &self.jpeg,
            &self.heif,
            source,
            input.as_ref(),
            output.as_ref(),
        )
    }
'''
replacement = anchor + '''
    /// Convert a Motion Photo while enforcing product-intent applicability.
    ///
    /// OPPO Gallery compatibility changes the still-image Gain Map graph and
    /// vendor metadata. A Motion Photo conversion has a distinct Live Photo
    /// publication contract, so silently ignoring that intent would be unsafe.
    pub fn convert_motion_photo_file_with_request(
        &self,
        source: &[u8],
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        request: ConversionRequest,
    ) -> Result<LivePhotoFileReceipt> {
        if request.requests_oppo_gallery_compatibility() {
            return Err(RuntimeError::new(
                "Motion Photo conversion",
                "OPPO-compatible output applies to ProXDR still images and cannot be combined with Motion Photo conversion",
            ));
        }
        self.convert_motion_photo_file(source, input, output)
    }
'''
if "pub fn convert_motion_photo_file_with_request(" not in text:
    if anchor not in text:
        raise SystemExit("Motion Photo runtime anchor not found")
    text = text.replace(anchor, replacement, 1)

runtime.write_text(text)

batch = Path("crates/xdremux-runtime/src/batch.rs")
text = batch.read_text()
text = text.replace(
    "match runtime.convert_motion_photo_file(&source, &item.input, &item.output) {",
    "match runtime.convert_motion_photo_file_with_request(\n                    &source,\n                    &item.input,\n                    &item.output,\n                    request,\n                ) {",
)
if "convert_motion_photo_file_with_request(" not in text:
    raise SystemExit("batch did not adopt product-aware Motion Photo conversion")
batch.write_text(text)
