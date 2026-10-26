# HDR and Apple editing

Verify HDR resource geometry and the actual Apple feature or app consumer affected by the change. Report portable, native, and device results separately.

## Select the affected behavior

Check the relevant behavior:

- HDR reconstruction and coded or display orientation.
- Complete Styles/Texture or Portrait resources.
- Cinematic pending-state recognition.
- The macOS app's switch for the same product feature.
- Preservation of published output when required checks fail.

## Use the public entry points

For standard HDR, use `xdremux convert --input INPUT --output NEW_OUTPUT`.

Add the documented `--apple-styles`, `--apple-portrait`, `--apple-portrait-oppo`, or `--apple-cinematic` intent for the corresponding feature. If app behavior changes, exercise the app switch separately.

## Prepare the environment

Identify the required media and source resources. Before running, confirm the applicable OS, SDK, and helper. Read current Apple feature documentation for compatibility and hardware requirements. Use fresh outputs.

## Run the existing checks

### Orientation

Run `cargo test --locked -p xdremux-codec --test orientation_roundtrip -- --nocapture`. Its real Samsung check provides portable codec evidence.

For ImageIO readback of the emitted artifact, use the existing product CI step that runs `scripts/apple/read_oriented_heif_imageio.swift`. Confirm the meaning of rotation and mirror association order and coded dimensions. Successful parsing alone is insufficient.

### Styles and Texture

For Styles and HDR thumbnails, run `bash scripts/apple/check_rust_style_consumer.sh`.

For Texture, run `python3 scripts/apple/check_texture_styles.py --all-fixtures --output NEW_EVIDENCE_DIRECTORY` on the required macOS 27 host. This check covers conversion, batch, the app bridge, feature validation, and media preservation. It also injects a failure and checks that an existing output is preserved.

### Portrait

For the affected profile, run `bash scripts/apple/check_rust_cli_apple_portrait.sh` or `python3 scripts/apple/check_rust_cli_oppo_portrait.py`. Do not silently substitute one profile for the other.

### Cinematic and app behavior

Use the current `docs/cinematic.en.md` procedure and existing `scripts/apple/verify_cinematic_envelope.py`. Preserve the distinction between pending-state recognition and native preprocessing.

For app changes, run `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh`. Also exercise the affected entry point in the actual app.

### System Gain Map experiment

Use the experiment's current research job and `scripts/apple/measure_system_heif_hdr.sh`. If measurement fails before Styles publication, retain the failure result. Do not describe this optional prototype as the default product path.

## Interpret the evidence

Linux excludes macOS-gated modules. A Linux run cannot establish their native execution. A helper mock can establish only Rust transport and policy behavior.

A parser-visible URI or nonempty dictionary does not prove resource roles, GPU success, final edit state, or Photos behavior. A generic adapter handshake does not establish the availability of private selectors or device resources.

Require final readback and failure-safe publication even for typed internal media values. Real Photos import, parameter response, edit/save/reopen, display rendering, and device-specific quality require their own recorded device evidence.
